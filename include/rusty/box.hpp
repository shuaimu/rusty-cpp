#ifndef RUSTY_BOX_HPP
#define RUSTY_BOX_HPP

#include <algorithm>
#include <concepts>
#include <new>          // for placement new
#include <string_view>
#include <type_traits>  // for std::enable_if, std::is_convertible, std::is_same
#include <utility>  // for std::move, std::forward
#include <rusty/alloc.hpp>

// Box<T> - A smart pointer for heap-allocated values with single ownership
// Equivalent to Rust's Box<T>
//
// Guarantees:
// - Single ownership (no copying)
// - Automatic deallocation when Box goes out of scope
// - Move semantics only
// - Null state after move

// @safe
namespace rusty {

template<typename Container>
auto as_slice(Container&& container);

template<typename T, typename A = rusty::alloc::Global>
class Box {
private:
    T* ptr;
    // Stored allocator. `[[no_unique_address]]` collapses A to zero bytes
    // whenever A is empty (the common case: A = rusty::alloc::Global), so
    // sizeof(Box<T>) stays equal to sizeof(T*) on all major compilers.
    [[no_unique_address]] A alloc_;

    template<typename, typename> friend class Box;

    // Private constructor for faithful factories that have already obtained
    // raw bytes from `alloc_inst.allocate(...)` and placement-new'd a `T`
    // into them. The Box now owns both the live `T` and the allocator that
    // produced its storage.
    Box(T* p, A alloc_inst) noexcept(std::is_nothrow_move_constructible_v<A>)
        : ptr(p), alloc_(std::move(alloc_inst)) {}

    // The single drop+deallocate path used by the destructor and by move
    // assignment. Safe to call on a moved-from Box (ptr == nullptr).
    void drop_in_place_and_deallocate() noexcept {
        if (ptr != nullptr) {
            ptr->~T();
            alloc_.deallocate(
                rusty::NonNull<std::uint8_t>::from(
                    reinterpret_cast<std::uint8_t*>(ptr)),
                rusty::alloc::Layout::for_value<T>());
            ptr = nullptr;
        }
    }

public:
    // Constructors
    // No default constructor - Box must always own a value (non-nullable)
    Box() = delete;

    // @unsafe — caller promises `p` was allocated via a default-constructed
    // `A`'s allocate path. For A = Global on standard stdlib implementations
    // this is equivalent to a pointer obtained from `::operator new` of the
    // same size/alignment.
    // @lifetime: owned
    explicit Box(T* p)
        noexcept(std::is_nothrow_default_constructible_v<A>)
        requires std::is_default_constructible_v<A>
        : ptr(p), alloc_() {}

    // Factory method - Box::new_() (Rust's Box::new, renamed because `new` is a C++ keyword)
    // @lifetime: owned
    static Box new_(T value) requires std::is_default_constructible_v<A> {
        return new_in(std::move(value), A{});
    }

    // Rust's `Box::new_in(value, alloc)` — faithful: ask `alloc_inst` for raw
    // bytes sized for T, placement-new T into them, store both the pointer and
    // the allocator. Destruction undoes exactly this.
    // @lifetime: owned
    static Box new_in(T value, A alloc_inst) {
        constexpr auto layout = rusty::alloc::Layout::for_value<T>();
        auto result = alloc_inst.allocate(layout);
        if (result.is_err()) {
            rusty::alloc::handle_alloc_error(layout);
        }
        auto raw = result.unwrap();
        // @unsafe { placement-new into freshly allocated raw bytes }
        T* p = ::new (static_cast<void*>(raw.as_ptr())) T(std::move(value));
        return Box(p, std::move(alloc_inst));
    }

    // In-place construct a T from forwarded args, without an intermediate
    // value-move. Used by `make_box(args...)`.
    template<typename... Args>
    static Box emplace(Args&&... args)
        requires std::is_default_constructible_v<A>
    {
        return emplace_in(A{}, std::forward<Args>(args)...);
    }

    template<typename... Args>
    static Box emplace_in(A alloc_inst, Args&&... args) {
        constexpr auto layout = rusty::alloc::Layout::for_value<T>();
        auto result = alloc_inst.allocate(layout);
        if (result.is_err()) {
            rusty::alloc::handle_alloc_error(layout);
        }
        auto raw = result.unwrap();
        // @unsafe { placement-new into freshly allocated raw bytes }
        T* p = ::new (static_cast<void*>(raw.as_ptr())) T(std::forward<Args>(args)...);
        return Box(p, std::move(alloc_inst));
    }

    // Alias for backward compatibility
    // @lifetime: owned
    static Box make(T value) requires std::is_default_constructible_v<A> {
        return new_(std::move(value));
    }

    // No copy constructor - Box cannot be copied
    Box(const Box&) = delete;
    Box& operator=(const Box&) = delete;

    // Move constructor - transfers both the pointer and the allocator state.
    // @lifetime: owned
    Box(Box&& other) noexcept(std::is_nothrow_move_constructible_v<A>)
        : ptr(other.ptr), alloc_(std::move(other.alloc_)) {
        other.ptr = nullptr;
    }

    // Converting move constructor — Box<Derived, A> -> Box<Base, A>. Requires
    // matching allocator type so the destination's `alloc_` faithfully owns
    // the source's bytes. Caller is responsible for ensuring `~Base()` runs
    // the right destructor for the dynamic type (typical pattern: `Base` has
    // a virtual destructor).
    // @lifetime: owned
    template<typename U, typename UA, typename = typename std::enable_if<
        std::is_convertible<U*, T*>::value
        && std::is_same<UA, A>::value
        && !std::is_same<U, T>::value>::type>
    Box(Box<U, UA>&& other) noexcept(std::is_nothrow_move_constructible_v<A>)
        : ptr(other.ptr), alloc_(std::move(other.alloc_)) {
        other.ptr = nullptr;
    }

    // Move assignment - transfers ownership of both ptr and allocator.
    // @lifetime: owned
    Box& operator=(Box&& other) noexcept {
        if (this != &other) {
            drop_in_place_and_deallocate();
            ptr = other.ptr;
            alloc_ = std::move(other.alloc_);
            other.ptr = nullptr;
        }
        return *this;
    }

    // Converting move assignment — same allocator-type requirement as the
    // converting move constructor.
    // @lifetime: owned
    template<typename U, typename UA, typename = typename std::enable_if<
        std::is_convertible<U*, T*>::value
        && std::is_same<UA, A>::value
        && !std::is_same<U, T>::value>::type>
    Box& operator=(Box<U, UA>&& other) noexcept {
        drop_in_place_and_deallocate();
        ptr = other.ptr;
        alloc_ = std::move(other.alloc_);
        other.ptr = nullptr;
        return *this;
    }

    // Clone by deep-copying the pointee, using a copy of the current
    // allocator for the new Box. Mirrors Rust's `impl<T: Clone, A: Allocator + Clone>
    // Clone for Box<T, A>`.
    // @lifetime: owned
    Box clone() const requires std::copyable<A> {
        if constexpr (requires(const T& value) { value.clone(); }) {
            return new_in(ptr->clone(), alloc_);
        } else if constexpr (std::is_copy_constructible<T>::value) {
            return new_in(*ptr, alloc_);
        } else {
            static_assert(
                std::is_copy_constructible<T>::value,
                "rusty::Box::clone requires a cloneable or copyable pointee type"
            );
        }
    }

    // Destructor - runs T's destructor in place and returns the storage to
    // the stored allocator. Equivalent to Rust's `impl<T, A: Allocator>
    // Drop for Box<T, A>` (drop_in_place + alloc.deallocate).
    ~Box() {
        drop_in_place_and_deallocate();
    }
    
    // Dereference - borrow the value
    // @lifetime: (&'a) -> &'a
    T& operator*() {
        // @unsafe
        {
            // Pointer dereference is unsafe, but Box guarantees ptr is valid
            return *ptr;
        }
    }

    // @lifetime: (&'a) -> &'a
    const T& operator*() const {
        // @unsafe
        {
            return *ptr;
        }
    }

    // Arrow operator - access members
    // @lifetime: (&'a) -> &'a
    T* operator->() {
        return ptr;
    }

    // @lifetime: (&'a) -> &'a
    const T* operator->() const {
        return ptr;
    }

    template<typename... Args>
    decltype(auto) insert(Args&&... args)
        requires requires(T& value, Args&&... forwarded) {
            value.insert(std::forward<Args>(forwarded)...);
        }
    {
        return ptr->insert(std::forward<Args>(args)...);
    }

    template<typename... Args>
    decltype(auto) insert(Args&&... args) const
        requires requires(const T& value, Args&&... forwarded) {
            value.insert(std::forward<Args>(forwarded)...);
        }
    {
        return ptr->insert(std::forward<Args>(args)...);
    }
    
    // Check if box contains a value
    bool is_valid() const {
        return ptr != nullptr;
    }
    
    // Explicit bool conversion
    explicit operator bool() const {
        return is_valid();
    }

    // String-like deref coercion for Box<str>/Box<String>-style call sites.
    template<typename U = T>
    requires (std::is_convertible_v<const U&, std::string_view>)
    operator std::string_view() const {
        if (!ptr) {
            return std::string_view();
        }
        return static_cast<std::string_view>(*ptr);
    }
    
    // Take ownership of the raw pointer (Rust: Box::into_raw)
    // After this, the Box is empty and caller is responsible for deletion
    // @unsafe
    // @lifetime: owned
    T* into_raw() {
        T* temp = ptr;
        ptr = nullptr;
        return temp;
    }

    // C++-style alias for into_raw
    // @unsafe
    // @lifetime: owned
    T* release() {
        return into_raw();
    }

    // Take ownership of a raw pointer (Rust: Box::from_raw)
    // Caller must ensure pointer was allocated with compatible allocator.
    // @unsafe
    // @lifetime: owned
    static Box<T> from_raw(T* p) {
        return Box<T>(p);
    }

    // Consume the Box and return the raw pointer, leaking the allocation
    // (Rust: Box::leak). After this, the Box is empty and the value is never
    // deallocated by this owner; the returned pointer outlives any Box-based
    // ownership chain. Modeled as a member method (mirroring `into_raw`) so
    // the transpiler can lower `Box::leak(b)` to `(std::move(b)).leak()`.
    // @unsafe
    // @lifetime: owned
    T* leak() noexcept {
        T* p = ptr;
        ptr = nullptr;
        return p;
    }

    // Get raw pointer without transferring ownership
    // @unsafe - returns raw pointer, use operator* or operator-> instead
    // @lifetime: (&'a) -> &'a
    T* get() const {
        return ptr;
    }

    // Access the stored allocator. Mirrors Rust's `Box::allocator(&self) -> &A`.
    // @lifetime: (&'a) -> &'a
    const A& allocator() const noexcept {
        return alloc_;
    }

    // Note: No reset() method - Box is non-nullable like Rust's Box<T>
    // To replace the value, use assignment: box = Box::make(new_value)
    // To destroy, let it go out of scope or use std::move
};

// Factory function following C++ make_* convention. Routes through Box's
// allocator path (Box<T>::emplace) so destructor faithfully matches storage.
template<typename T, typename... Args>
// @lifetime: owned
Box<T> make_box(Args&&... args) {
    return Box<T>::emplace(std::forward<Args>(args)...);
}

template<typename T, typename A>
Box<T, A> make_box(Box<T, A>& value) {
    return value.clone();
}

template<typename T, typename A>
Box<T, A> make_box(const Box<T, A>& value) {
    return value.clone();
}

template<typename T, typename A>
Box<T, A> make_box(Box<T, A>&& value) {
    return std::move(value);
}

// Deduction-friendly overload for call sites that do not spell `<T>`.
template<typename T>
// @lifetime: owned
Box<std::remove_cvref_t<T>> make_box(T&& value) {
    using U = std::remove_cvref_t<T>;
    return Box<U>::new_(std::forward<T>(value));
}

template<typename L, typename LA, typename R, typename RA>
bool operator==(const Box<L, LA>& lhs, const Box<R, RA>& rhs) {
    auto slice_like_equal = [](const auto& left_slice, const auto& right_slice) {
        if (left_slice.size() != right_slice.size()) {
            return false;
        }
        return std::equal(
            left_slice.begin(),
            left_slice.end(),
            right_slice.begin(),
            [](const auto& l, const auto& r) {
                using LElem = std::remove_cv_t<std::remove_reference_t<decltype(l)>>;
                using RElem = std::remove_cv_t<std::remove_reference_t<decltype(r)>>;
                if constexpr (requires { l == r; }) {
                    return static_cast<bool>(l == r);
                } else if constexpr (requires { r == l; }) {
                    return static_cast<bool>(r == l);
                } else if constexpr (std::is_empty_v<LElem> && std::is_empty_v<RElem>) {
                    return true;
                } else {
                    return false;
                }
            });
    };

    if (!lhs.is_valid() || !rhs.is_valid()) {
        return lhs.get() == rhs.get();
    }
    if constexpr (requires(const L& left, const R& right) { left == right; }) {
        return *lhs == *rhs;
    } else if constexpr (requires(const L& left, const R& right) { right == left; }) {
        return *rhs == *lhs;
    } else if constexpr (requires(const L& left) { rusty::as_slice(left); } &&
                         requires(const R& right) { rusty::as_slice(right); }) {
        return slice_like_equal(rusty::as_slice(*lhs), rusty::as_slice(*rhs));
    } else {
        return static_cast<const void*>(lhs.get()) == static_cast<const void*>(rhs.get());
    }
}

template<typename L, typename LA, typename R, typename RA>
bool operator!=(const Box<L, LA>& lhs, const Box<R, RA>& rhs) {
    return !(lhs == rhs);
}

} // namespace rusty

#endif // RUSTY_BOX_HPP
