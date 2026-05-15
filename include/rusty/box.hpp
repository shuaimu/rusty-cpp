#ifndef RUSTY_BOX_HPP
#define RUSTY_BOX_HPP

#include <algorithm>
#include <string_view>
#include <type_traits>  // for std::enable_if, std::is_convertible, std::is_same
#include <utility>  // for std::move, std::forward

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

template<typename T>
class Box {
private:
    T* ptr;
    
public:
    // Constructors
    // No default constructor - Box must always own a value (non-nullable)
    Box() = delete;

    // @lifetime: owned
    explicit Box(T* p) : ptr(p) {}

    // Factory method - Box::new_() (Rust's Box::new, renamed because `new` is a C++ keyword)
    // @lifetime: owned
    static Box<T> new_(T value) {
        // @unsafe
        {
            return Box<T>(new T(std::move(value)));
        }
    }

    // Alias for backward compatibility
    // @lifetime: owned
    static Box<T> make(T value) {
        // @unsafe
        {
            // new and std::move are unsafe operations
            return Box<T>(new T(std::move(value)));
        }
    }
    
    // No copy constructor - Box cannot be copied
    Box(const Box&) = delete;
    Box& operator=(const Box&) = delete;
    
    // Move constructor - transfers ownership
    // @lifetime: owned
    Box(Box&& other) noexcept : ptr(other.ptr) {
        other.ptr = nullptr;  // Other box becomes empty
    }

    // Converting move constructor - allows Box<Derived> to convert to Box<Base>
    // Only enabled when U* is convertible to T* (i.e., U derives from T)
    // @lifetime: owned
    template<typename U, typename = typename std::enable_if<
        std::is_convertible<U*, T*>::value && !std::is_same<U, T>::value>::type>
    Box(Box<U>&& other) noexcept : ptr(other.release()) {}

    // Move assignment - transfers ownership
    // @lifetime: owned
    Box& operator=(Box&& other) noexcept {
        // @unsafe
        {
            if (this != &other) {
                delete ptr;
                ptr = other.ptr;
                other.ptr = nullptr;
            }
            return *this;
        }
    }

    // Converting move assignment - allows Box<Derived> to assign to Box<Base>
    // @lifetime: owned
    template<typename U, typename = typename std::enable_if<
        std::is_convertible<U*, T*>::value && !std::is_same<U, T>::value>::type>
    Box& operator=(Box<U>&& other) noexcept {
        // @unsafe
        {
            delete ptr;
            ptr = other.release();
            return *this;
        }
    }

    // Clone by deep-copying the pointee when the pointee supports cloning.
    // This mirrors Rust's `Clone for Box<T>` behavior.
    // @lifetime: owned
    Box clone() const {
        if constexpr (requires(const T& value) { value.clone(); }) {
            // @unsafe
            {
                return Box<T>(new T(ptr->clone()));
            }
        } else if constexpr (std::is_copy_constructible<T>::value) {
            // @unsafe
            {
                return Box<T>(new T(*ptr));
            }
        } else {
            static_assert(
                std::is_copy_constructible<T>::value,
                "rusty::Box::clone requires a cloneable or copyable pointee type"
            );
        }
    }

    // Destructor - automatic cleanup
    ~Box() {
        // @unsafe
        {
            delete ptr;
        }
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

    // Note: No reset() method - Box is non-nullable like Rust's Box<T>
    // To replace the value, use assignment: box = Box::make(new_value)
    // To destroy, let it go out of scope or use std::move
};

// Factory function following C++ make_* convention
template<typename T, typename... Args>
// @lifetime: owned
Box<T> make_box(Args&&... args) {
    // @unsafe
    {
        // new and std::forward are unsafe operations
        return Box<T>(new T(std::forward<Args>(args)...));
    }
}

template<typename T>
Box<T> make_box(Box<T>& value) {
    return value.clone();
}

template<typename T>
Box<T> make_box(const Box<T>& value) {
    return value.clone();
}

template<typename T>
Box<T> make_box(Box<T>&& value) {
    return std::move(value);
}

// Deduction-friendly overload for call sites that do not spell `<T>`.
template<typename T>
// @lifetime: owned
Box<std::remove_cvref_t<T>> make_box(T&& value) {
    using U = std::remove_cvref_t<T>;
    // @unsafe
    {
        return Box<U>(new U(std::forward<T>(value)));
    }
}

template<typename L, typename R>
bool operator==(const Box<L>& lhs, const Box<R>& rhs) {
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

template<typename L, typename R>
bool operator!=(const Box<L>& lhs, const Box<R>& rhs) {
    return !(lhs == rhs);
}

} // namespace rusty

#endif // RUSTY_BOX_HPP
