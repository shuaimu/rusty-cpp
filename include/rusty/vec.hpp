#ifndef RUSTY_VEC_HPP
#define RUSTY_VEC_HPP

#include <memory>
#include <algorithm>
#include <initializer_list>
#include <cassert>
#include <utility>  // for std::move, std::forward
#include <cstddef>  // for size_t
#include <cstdint>
#include <cstring>  // for memcpy
#include <limits>
#include <span>
#include <type_traits>
#include <tuple>
#include <iterator>
#if !defined(RUSTY_NO_STD_VECTOR_INTEROP)
#include <vector>
#endif
#include <rusty/alloc.hpp>
#include <rusty/function.hpp>
#include <rusty/io.hpp>
#include <rusty/mem.hpp>
#include <rusty/option.hpp>

// VecLegacy<T> - A growable array with owned elements
// Equivalent to Rust's VecLegacy<T>
//
// Guarantees:
// - Single ownership of the container
// - Elements are owned by the VecLegacy
// - Automatic memory management
// - Move semantics for the container

// @safe
namespace rusty {

template<typename T, typename A = rusty::alloc::Global>
class VecLegacy {
private:
    T* data_;
    size_t size_;
    size_t capacity_;
    // Stored allocator. `[[no_unique_address]]` collapses A to 0 bytes when
    // A is empty (the common A = Global case), so sizeof(VecLegacy<T>) is
    // unchanged on all major compilers.
    [[no_unique_address]] A alloc_;

    template<typename, typename> friend class VecLegacy;

    static constexpr bool can_materialize_capacity(size_t capacity) noexcept {
        return capacity <= std::numeric_limits<size_t>::max() / sizeof(T);
    }

    static constexpr rusty::alloc::Layout storage_layout(size_t capacity) noexcept {
        return rusty::alloc::Layout::from_size_align_unchecked(
            capacity * sizeof(T), alignof(T));
    }

    // Allocate raw storage via the stored allocator. Instance method (not
    // static) so the same A instance that produced the bytes is the one
    // that later releases them.
    T* allocate_storage(size_t capacity) {
        if (capacity == 0) {
            return nullptr;
        }
        if (!can_materialize_capacity(capacity)) {
            throw std::bad_alloc();
        }
        const auto layout = storage_layout(capacity);
        auto result = alloc_.allocate(layout);
        if (result.is_err()) {
            rusty::alloc::handle_alloc_error(layout);
        }
        return reinterpret_cast<T*>(result.unwrap().as_ptr());
    }

    // Release raw storage via the stored allocator.
    void deallocate_storage(T* ptr, size_t capacity) noexcept {
        if (ptr == nullptr || capacity == 0 || !can_materialize_capacity(capacity)) {
            return;
        }
        alloc_.deallocate(
            rusty::NonNull<std::uint8_t>::from(
                reinterpret_cast<std::uint8_t*>(ptr)),
            storage_layout(capacity));
    }

    static constexpr size_t storage_byte_count(size_t capacity) noexcept {
        if (capacity > std::numeric_limits<size_t>::max() / sizeof(T)) {
            return std::numeric_limits<size_t>::max();
        }
        return capacity * sizeof(T);
    }

    template<typename Bytes>
    static std::span<const uint8_t> write_byte_span(Bytes&& bytes) {
        using std::data;
        using std::size;
        auto* ptr = data(bytes);
        using Elem = std::remove_cv_t<std::remove_pointer_t<decltype(ptr)>>;
        static_assert(sizeof(Elem) == 1, "VecLegacy<u8>::write expects a byte-sized buffer");
        return std::span<const uint8_t>(
            reinterpret_cast<const uint8_t*>(ptr),
            static_cast<size_t>(size(bytes)));
    }

    void clear_forgotten_storage_marks() noexcept {
        // Strict null-state convention: there is no global
        // forgotten-address table anymore (see rusty/mem.hpp). Per-element
        // moved-from state lives in T itself. VecLegacy storage doesn't carry
        // its own forgotten markers across reallocations.
    }
    
    void grow() {
        size_t new_capacity = capacity_ == 0 ? 1 : capacity_ * 2;
        T* new_data = allocate_storage(new_capacity);
        
        // Move existing elements
        for (size_t i = 0; i < size_; ++i) {
            new (&new_data[i]) T(std::move(data_[i]));
            data_[i].~T();
        }
        
        clear_forgotten_storage_marks();
        deallocate_storage(data_, capacity_);
        data_ = new_data;
        capacity_ = new_capacity;
    }
    
public:
    // STL-style typedef. Rust iterators express the element type via
    // `type Item = T`, but several C++ APIs (gtest's ValuesIn,
    // generic algorithms that probe for value_type) require the
    // STL-conventional `typename C::value_type`. Exposing it here
    // costs nothing and lets VecLegacy<T> drop into those APIs unchanged.
    using value_type = T;

    // Default constructor - empty vec
    VecLegacy() : data_(nullptr), size_(0), capacity_(0) {}
    
    // Factory method - VecLegacy::new_() (Rust's VecLegacy::new, _ suffix because `new` is C++ keyword)
    // @lifetime: owned
    static VecLegacy<T> new_() {
        return VecLegacy<T>();
    }

    // Rust's `VecLegacy::new_in(alloc)` — construct an empty VecLegacy that will allocate
    // through `alloc_inst` when it grows. Zero allocation until first push.
    // @lifetime: owned
    static VecLegacy new_in(A alloc_inst) {
        VecLegacy v;
        v.alloc_ = std::move(alloc_inst);
        return v;
    }

    // Rust's `VecLegacy::with_capacity_in(cap, alloc)` — pre-reserve `cap` slots
    // through the supplied allocator.
    // @lifetime: owned
    static VecLegacy with_capacity_in(size_t cap, A alloc_inst) {
        VecLegacy v;
        v.alloc_ = std::move(alloc_inst);
        if (cap > 0) {
            v.data_ = v.allocate_storage(cap);
            v.capacity_ = cap;
        }
        return v;
    }

    // Unsafe constructor from raw parts.
    // Caller must guarantee `ptr` points to `cap` contiguous `T` slots with
    // the first `len` elements fully initialized and uniquely owned.
    static VecLegacy<T> from_raw_parts(T* ptr, size_t len, size_t cap) {
        assert(len <= cap);
        assert(ptr != nullptr || cap == 0);
        assert(can_materialize_capacity(cap));
        VecLegacy<T> v;
        v.data_ = ptr;
        v.size_ = len;
        v.capacity_ = cap;
        return v;
    }

    template<typename U>
    static VecLegacy<U> from_raw_parts(U* ptr, size_t len, size_t cap) {
        return VecLegacy<U>::from_raw_parts(ptr, len, cap);
    }

    template<typename Alloc>
    static VecLegacy<T> from_raw_parts_in(T* ptr, size_t len, size_t cap, Alloc&&) {
        return from_raw_parts(ptr, len, cap);
    }

    template<typename U, typename Alloc>
    static VecLegacy<U> from_raw_parts_in(U* ptr, size_t len, size_t cap, Alloc&&) {
        return VecLegacy<U>::from_raw_parts(ptr, len, cap);
    }

    template<typename Iter>
    static VecLegacy<T> from_iter(Iter&& iter) {
        VecLegacy<T> result;
        auto option_like_has_value = [](const auto& opt) {
            if constexpr (requires { opt.is_some(); }) {
                return opt.is_some();
            } else if constexpr (requires { opt.has_value(); }) {
                return opt.has_value();
            } else {
                return static_cast<bool>(opt);
            }
        };
        auto option_like_take_value = [](auto& opt) {
            if constexpr (requires { opt.unwrap(); }) {
                return opt.unwrap();
            } else if constexpr (requires { opt.has_value(); opt.reset(); }) {
                auto value = std::move(*opt);
                opt.reset();
                return value;
            } else {
                return std::move(*opt);
            }
        };
        auto normalize_item_for_vec = [](auto&& value) -> T {
            using Value = std::remove_cvref_t<decltype(value)>;
            if constexpr (std::is_pointer_v<Value> && !std::is_pointer_v<T>) {
                using Pointee = std::remove_cv_t<std::remove_pointer_t<Value>>;
                if constexpr (std::is_convertible_v<Pointee, T>) {
                    return static_cast<T>(*value);
                } else {
                    return T(*value);
                }
            } else if constexpr (requires { value.get(); } && !std::is_pointer_v<T>) {
                using GetType = std::remove_cvref_t<decltype(value.get())>;
                if constexpr (std::is_pointer_v<GetType>) {
                    using Pointee = std::remove_cv_t<std::remove_pointer_t<GetType>>;
                    if constexpr (std::is_convertible_v<Pointee, T>) {
                        return static_cast<T>(*value.get());
                    } else {
                        return T(*value.get());
                    }
                } else if constexpr (std::is_convertible_v<decltype(value.get()), T>) {
                    return static_cast<T>(value.get());
                } else {
                    return T(value.get());
                }
            } else if constexpr (std::is_convertible_v<decltype(value), T>) {
                return static_cast<T>(std::forward<decltype(value)>(value));
            } else {
                return T(std::forward<decltype(value)>(value));
            }
        };
        if constexpr (requires { std::forward<Iter>(iter).next(); }) {
            auto&& next_iter = std::forward<Iter>(iter);
            while (true) {
                auto item = next_iter.next();
                if (!option_like_has_value(item)) {
                    break;
                }
                result.push(normalize_item_for_vec(option_like_take_value(item)));
            }
        } else if constexpr (requires { std::forward<Iter>(iter).into_iter(); }) {
            return from_iter(std::forward<Iter>(iter).into_iter());
        } else {
            for (auto&& item : std::forward<Iter>(iter)) {
                result.push(normalize_item_for_vec(std::forward<decltype(item)>(item)));
            }
        }
        return result;
    }

    // Alias for backward compatibility
    static VecLegacy<T> make() {
        return VecLegacy<T>();
    }
    
    // Rust-idiomatic factory with capacity - VecLegacy::with_capacity()
    // @lifetime: owned
    static VecLegacy<T> with_capacity(size_t cap) {
        VecLegacy<T> v;
        if (cap > 0) {
            v.data_ = v.allocate_storage(cap);
            v.capacity_ = cap;
        }
        return v;
    }
    
    // Constructor with initial capacity (C++ style)
    explicit VecLegacy(size_t initial_capacity) 
        : data_(nullptr), size_(0), capacity_(0) {
        if (initial_capacity > 0) {
            data_ = allocate_storage(initial_capacity);
            capacity_ = initial_capacity;
        }
    }
    
    // Initializer list constructor
    VecLegacy(std::initializer_list<T> init) 
        : data_(nullptr), size_(0), capacity_(0) {
        reserve(init.size());
        for (const T& item : init) {
            push(item);
        }
    }
    
    // Copy constructor with clone()-fallback for move-only Rust-like payloads.
    VecLegacy(const VecLegacy& other) : data_(nullptr), size_(0), capacity_(0) {
        reserve(other.size_);
        for (size_t i = 0; i < other.size_; ++i) {
            if constexpr (std::is_copy_constructible_v<T>) {
                push(other.data_[i]);
            } else if constexpr (requires(const T& value) { value.clone(); }) {
                push(other.data_[i].clone());
            } else {
                static_assert(
                    std::is_copy_constructible_v<T>,
                    "VecLegacy copy requires copy-constructible or clone()-able elements");
            }
        }
    }

    VecLegacy& operator=(const VecLegacy& other) {
        if (this == &other) {
            return *this;
        }
        clear();
        reserve(other.size_);
        for (size_t i = 0; i < other.size_; ++i) {
            if constexpr (std::is_copy_constructible_v<T>) {
                push(other.data_[i]);
            } else if constexpr (requires(const T& value) { value.clone(); }) {
                push(other.data_[i].clone());
            } else {
                static_assert(
                    std::is_copy_constructible_v<T>,
                    "VecLegacy copy assignment requires copy-constructible or clone()-able elements");
            }
        }
        return *this;
    }
    
    // Move constructor
    VecLegacy(VecLegacy&& other) noexcept 
        : data_(other.data_), size_(other.size_), capacity_(other.capacity_) {
        other.data_ = nullptr;
        other.size_ = 0;
        other.capacity_ = 0;
    }

    // Converting move constructor for VecLegacy<U, UA> -> VecLegacy<T, A> when element conversion exists.
    template<typename U, typename UA>
    requires (!(std::is_same_v<U, T> && std::is_same_v<UA, A>) && std::is_constructible_v<T, U>)
    VecLegacy(VecLegacy<U, UA>&& other) : data_(nullptr), size_(0), capacity_(0) {
        reserve(other.len());
        for (size_t i = 0; i < other.len(); ++i) {
            if constexpr (std::is_convertible_v<U, T>) {
                push(static_cast<T>(std::move(other[i])));
            } else {
                push(T(std::move(other[i])));
            }
        }
        other.clear();
    }
    
    // Move assignment
    VecLegacy& operator=(VecLegacy&& other) {
        if (this != &other) {
            // Clean up existing data
            clear();
            clear_forgotten_storage_marks();
            deallocate_storage(data_, capacity_);
            
            // Take ownership
            data_ = other.data_;
            size_ = other.size_;
            capacity_ = other.capacity_;
            
            other.data_ = nullptr;
            other.size_ = 0;
            other.capacity_ = 0;
        }
        return *this;
    }

    #if !defined(RUSTY_NO_STD_VECTOR_INTEROP)
    // Interop bridge for code paths that still materialize std::vector.
    operator std::vector<T>() && {
        std::vector<T> out;
        out.reserve(size_);
        for (size_t i = 0; i < size_; ++i) {
            out.push_back(std::move(data_[i]));
        }
        clear();
        return out;
    }
    #endif
    
    // Destructor.
    //
    // The only operation in the body that can possibly throw is
    // clear(), which invokes ~T() for each element. The other two
    // calls are noexcept (operator delete and pure bookkeeping).
    // Mirror std::vector's design: propagate T's destructor's
    // exception specification. That way VecLegacy<T> for T with a
    // noexcept destructor (uint64_t, std::string, ...) is itself
    // noexcept-destructible, which is required for any class that
    // holds a VecLegacy<T> member and overrides a virtual destructor of
    // a noexcept base — the situation that arises in gtest fixture
    // hierarchies and Masstree's search_range_callback.
    ~VecLegacy() noexcept(std::is_nothrow_destructible_v<T>) {
        clear();
        clear_forgotten_storage_marks();
        deallocate_storage(data_, capacity_);
    }
    
    // Push element to the back
    void push(T value) {
        if (size_ >= capacity_) {
            grow();
        }
        new (&data_[size_]) T(std::move(value));
        ++size_;
    }

    template<typename Bytes>
    io::Result<size_t> write(Bytes&& buf)
    requires(std::is_same_v<std::remove_cv_t<T>, uint8_t>
             && requires(std::remove_reference_t<Bytes>& bytes) {
                    std::data(bytes);
                    std::size(bytes);
                })
    {
        auto bytes = write_byte_span(buf);
        reserve(size_ + bytes.size());
        // Fast path: memcpy. The per-byte push loop below was ~10x
        // slower on Marshal V2 / Cursor<VecLegacy<u8>> byte writes.
        if (bytes.size() > 0) {
            std::memcpy(data_ + size_, bytes.data(), bytes.size());
        }
        size_ += bytes.size();
        return io::Result<size_t>::ok(bytes.size());
    }

    template<typename Bytes>
    io::Result<std::tuple<>> write_all(Bytes&& buf)
    requires(std::is_same_v<std::remove_cv_t<T>, uint8_t>
             && requires(std::remove_reference_t<Bytes>& bytes) {
                    std::data(bytes);
                    std::size(bytes);
                })
    {
        auto result = write(std::forward<Bytes>(buf));
        if (result.is_err()) {
            return io::Result<std::tuple<>>::err(result.unwrap_err());
        }
        return io::Result<std::tuple<>>::ok(std::make_tuple());
    }

    // Insert element at index, shifting trailing elements to the right.
    void insert(size_t index, T value) {
        assert(index <= size_);
        if (size_ >= capacity_) {
            grow();
        }
        if (index == size_) {
            new (&data_[size_]) T(std::move(value));
            ++size_;
            return;
        }
        for (size_t i = size_; i > index; --i) {
            new (&data_[i]) T(std::move(data_[i - 1]));
            data_[i - 1].~T();
        }
        new (&data_[index]) T(std::move(value));
        ++size_;
    }

    template<typename Iter>
    void extend(Iter&& iter) {
        auto option_like_has_value = [](const auto& opt) {
            if constexpr (requires { opt.is_some(); }) {
                return opt.is_some();
            } else if constexpr (requires { opt.has_value(); }) {
                return opt.has_value();
            } else {
                return static_cast<bool>(opt);
            }
        };
        auto option_like_take_value = [](auto& opt) {
            if constexpr (requires { opt.unwrap(); }) {
                return opt.unwrap();
            } else if constexpr (requires { opt.has_value(); opt.reset(); }) {
                auto value = std::move(*opt);
                opt.reset();
                return value;
            } else {
                return std::move(*opt);
            }
        };
        auto normalize_item_for_vec = [](auto&& value) -> T {
            using Value = std::remove_cvref_t<decltype(value)>;
            if constexpr (std::is_pointer_v<Value> && !std::is_pointer_v<T>) {
                using Pointee = std::remove_cv_t<std::remove_pointer_t<Value>>;
                if constexpr (std::is_convertible_v<Pointee, T>) {
                    return static_cast<T>(*value);
                } else {
                    return T(*value);
                }
            } else if constexpr (requires { value.get(); } && !std::is_pointer_v<T>) {
                using GetType = std::remove_cvref_t<decltype(value.get())>;
                if constexpr (std::is_pointer_v<GetType>) {
                    using Pointee = std::remove_cv_t<std::remove_pointer_t<GetType>>;
                    if constexpr (std::is_convertible_v<Pointee, T>) {
                        return static_cast<T>(*value.get());
                    } else {
                        return T(*value.get());
                    }
                } else if constexpr (std::is_convertible_v<decltype(value.get()), T>) {
                    return static_cast<T>(value.get());
                } else {
                    return T(value.get());
                }
            } else if constexpr (requires { value.clone(); }) {
                return value.clone();
            } else if constexpr (std::is_convertible_v<decltype(value), T>) {
                return static_cast<T>(std::forward<decltype(value)>(value));
            } else {
                return T(std::forward<decltype(value)>(value));
            }
        };
        if constexpr (requires { std::forward<Iter>(iter).next(); }) {
            auto&& next_iter = std::forward<Iter>(iter);
            while (true) {
                auto item = next_iter.next();
                if (!option_like_has_value(item)) {
                    break;
                }
                push(normalize_item_for_vec(option_like_take_value(item)));
            }
        } else if constexpr (requires { std::forward<Iter>(iter).into_iter(); }) {
            extend(std::forward<Iter>(iter).into_iter());
        } else {
            for (auto&& item : std::forward<Iter>(iter)) {
                push(normalize_item_for_vec(std::forward<decltype(item)>(item)));
            }
        }
    }

    void extend_from_slice(std::span<const T> other) {
        reserve(size_ + other.size());
        if constexpr (std::is_trivially_copyable_v<T>
                      && !requires(const T& t) { t.clone(); }) {
            // Fast path: memcpy the whole slice. For uint8_t this is ~10x
            // faster than the per-element push loop below — both Marshal
            // V2 and io::Cursor<VecLegacy<u8>>::write depend on this.
            if (other.size() > 0) {
                std::memcpy(data_ + size_, other.data(), other.size() * sizeof(T));
            }
            size_ += other.size();
        } else {
            for (const auto& item : other) {
                if constexpr (requires { item.clone(); }) {
                    push(item.clone());
                } else {
                    push(item);
                }
            }
        }
    }
    
    // Pop element from the back
    Option<T> pop() {
        if (size_ == 0) {
            return Option<T>(None);
        }
        --size_;
        T result = std::move(data_[size_]);
        data_[size_].~T();
        return Option<T>(std::move(result));
    }
    
    // Access element by index
    // @lifetime: (&'a) -> &'a
    T& operator[](size_t index) {
        assert(index < size_);
        return data_[index];
    }
    
    // @lifetime: (&'a) -> &'a
    const T& operator[](size_t index) const {
        assert(index < size_);
        return data_[index];
    }

    Option<T&> get(size_t index) {
        if (index < size_) {
            return Option<T&>(data_[index]);
        }
        return Option<T&>(None);
    }

    Option<const T&> get(size_t index) const {
        if (index < size_) {
            return Option<const T&>(data_[index]);
        }
        return Option<const T&>(None);
    }

    Option<T&> get_mut(size_t index) {
        return get(index);
    }

    Option<const T&> get_mut(size_t index) const {
        return get(index);
    }

    Option<const T&> first() const {
        return get(0);
    }

    Option<T&> first_mut() {
        return get_mut(0);
    }

    Option<const T&> last() const {
        if (size_ == 0) {
            return Option<const T&>(None);
        }
        return Option<const T&>(data_[size_ - 1]);
    }

    Option<T&> last_mut() {
        if (size_ == 0) {
            return Option<T&>(None);
        }
        return Option<T&>(data_[size_ - 1]);
    }

    // Get first element
    // @lifetime: (&'a) -> &'a
    T& front() {
        assert(size_ > 0);
        return data_[0];
    }
    
    // @lifetime: (&'a) -> &'a
    const T& front() const {
        assert(size_ > 0);
        return data_[0];
    }
    
    // Get last element
    // @lifetime: (&'a) -> &'a
    T& back() {
        assert(size_ > 0);
        return data_[size_ - 1];
    }
    
    // @lifetime: (&'a) -> &'a
    const T& back() const {
        assert(size_ > 0);
        return data_[size_ - 1];
    }
    
    // Get size
    size_t len() const { return size_; }

    // Access the stored allocator. Mirrors Rust's `VecLegacy::allocator(&self) -> &A`.
    // @lifetime: (&'a) -> &'a
    const A& allocator() const noexcept { return alloc_; }
    size_t size() const { return size_; }

    // Unsafe-style length override (Rust VecLegacy::set_len semantics).
    // Caller is responsible for initialization/drop invariants.
    void set_len(size_t new_len) {
        assert(new_len <= capacity_);
        size_ = new_len;
    }
    
    // Check if empty
    bool is_empty() const { return size_ == 0; }
    
    // Get capacity
    size_t capacity() const { return capacity_; }
    
    // Reserve capacity. Grows geometrically (max of requested and
    // 2*current) so a sequence of small reserve(size+N) calls
    // amortizes to O(N) total work — matches Rust's VecLegacy::reserve.
    // Use `reserve_exact` if you want the exact bound.
    void reserve(size_t new_capacity) {
        if (new_capacity > capacity_) {
            size_t target = capacity_ * 2;
            if (target < new_capacity) {
                target = new_capacity;
            }
            reserve_exact_capacity(target);
        }
    }

    // Reserve at least `additional` extra slots beyond the current
    // `size`. Uses the geometric-growth reserve.
    void reserve_exact(size_t additional) {
        reserve(size_ + additional);
    }

private:
    // Allocate to exactly `new_capacity` slots — no over-allocation.
    // Used internally by the geometric `reserve` and any future
    // shrink_to_fit-style operation.
    void reserve_exact_capacity(size_t new_capacity) {
        if (new_capacity > capacity_) {
            T* new_data = allocate_storage(new_capacity);

            // Move existing elements
            for (size_t i = 0; i < size_; ++i) {
                new (&new_data[i]) T(std::move(data_[i]));
                data_[i].~T();
            }

            clear_forgotten_storage_marks();
            deallocate_storage(data_, capacity_);
            data_ = new_data;
            capacity_ = new_capacity;
        }
    }

public:
    
    // Clear all elements
    void clear() {
        for (size_t i = 0; i < size_; ++i) {
            data_[i].~T();
        }
        size_ = 0;
    }
    
    // Iterator support
    // @lifetime: (&'a) -> &'a
    T* begin() { return data_; }
    const T* begin() const { return data_; }

    // Slice-style data access used by shared iterator helpers.
    T* data() { return data_; }
    const T* data() const { return data_; }
    
    // @lifetime: (&'a) -> &'a
    T* end() { return data_ + size_; }
    const T* end() const { return data_ + size_; }
    
    // Clone the VecLegacy (explicit deep copy)
    // @lifetime: owned
    VecLegacy clone() const {
        VecLegacy result = VecLegacy::with_capacity(capacity_);
        for (size_t i = 0; i < size_; ++i) {
            if constexpr (std::is_copy_constructible_v<T>) {
                result.push(data_[i]);
            } else if constexpr (requires(const T& value) { value.clone(); }) {
                result.push(data_[i].clone());
            } else {
                static_assert(
                    std::is_copy_constructible_v<T>,
                    "VecLegacy::clone requires copy-constructible or clone()-able elements");
            }
        }
        return result;
    }
    
    // Equality comparison
    bool operator==(const VecLegacy& other) const {
        if (size_ != other.size_) return false;
        for (size_t i = 0; i < size_; ++i) {
            if (!(data_[i] == other.data_[i])) return false;
        }
        return true;
    }

    template<typename U, typename UA>
    bool operator==(const VecLegacy<U, UA>& other) const {
        if (size_ != other.len()) return false;
        for (size_t i = 0; i < size_; ++i) {
            if constexpr (requires(const T& lhs, const U& rhs) { lhs == rhs; }) {
                if (!(data_[i] == other[i])) return false;
            } else if constexpr (requires(const T& lhs, const U& rhs) { rhs == lhs; }) {
                if (!(other[i] == data_[i])) return false;
            } else if constexpr (std::is_empty_v<std::remove_cv_t<T>>
                                 && std::is_empty_v<std::remove_cv_t<U>>) {
                continue;
            } else {
                return false;
            }
        }
        return true;
    }

    bool operator!=(const VecLegacy& other) const {
        return !(*this == other);
    }

    template<typename U, typename UA>
    bool operator!=(const VecLegacy<U, UA>& other) const {
        return !(*this == other);
    }

    // Retain only elements where predicate returns true
    // Similar to Rust's VecLegacy::retain
    // Uses rusty::Function for type-erased, move-only callable (no ref captures)
    // Predicate signature: bool(const T&) - takes immutable borrow of element
    void retain(Function<bool(const T&)> predicate) {
        size_t write = 0;
        for (size_t read = 0; read < size_; ++read) {
            if (predicate(static_cast<const T&>(data_[read]))) {
                if (write != read) {
                    // Move element to new position
                    new (&data_[write]) T(std::move(data_[read]));
                    data_[read].~T();
                }
                ++write;
            } else {
                // Destroy element that doesn't match predicate
                data_[read].~T();
            }
        }
        size_ = write;
    }

    // Extract elements where predicate returns true, removing them from this VecLegacy
    // Similar to Rust's VecLegacy::extract_if (formerly drain_filter)
    // Returns a new VecLegacy containing the extracted elements
    // Uses rusty::Function for type-erased, move-only callable (no ref captures)
    // Predicate signature: bool(const T&) - takes immutable borrow of element
    VecLegacy<T> extract_if(Function<bool(const T&)> predicate) {
        VecLegacy<T> extracted = VecLegacy<T>::with_capacity(size_ / 2);  // Reasonable initial guess
        size_t write = 0;
        for (size_t read = 0; read < size_; ++read) {
            if (predicate(static_cast<const T&>(data_[read]))) {
                // Move to extracted VecLegacy
                extracted.push(std::move(data_[read]));
                data_[read].~T();
            } else {
                // Keep in this VecLegacy
                if (write != read) {
                    new (&data_[write]) T(std::move(data_[read]));
                    data_[read].~T();
                }
                ++write;
            }
        }
        size_ = write;
        return extracted;
    }
};

// Helper function to create a VecLegacy
template<typename T>
// @lifetime: owned
VecLegacy<T> vec_of(std::initializer_list<T> init) {
    return VecLegacy<T>(init);
}

template<typename T>
void vec_extend_from_slice(VecLegacy<T>& self, std::span<const T> other) {
    self.reserve(self.size() + other.size());
    for (const auto& item : other) {
        if constexpr (requires { item.clone(); }) {
            self.push(item.clone());
        } else {
            self.push(item);
        }
    }
}

// Transitional alias: `rusty::Vec` is now an alias for the hand-written
// `rusty::VecLegacy`. The transpiled vec_port::Vec is the long-term
// canonical name; this alias keeps existing code (BTreeMap internals,
// HashMap, tests, examples) working while the migration completes.
//
// When the transpiled Vec is mature enough to drop in, retire this
// alias in favor of `using Vec = vec_port::Vec` (or rename
// vec_port::Vec → rusty::Vec at the module level).
template<typename T, typename A = rusty::alloc::Global>
using Vec = VecLegacy<T, A>;

} // namespace rusty

#endif // RUSTY_VEC_HPP
