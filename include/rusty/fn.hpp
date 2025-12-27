#ifndef RUSTY_FN_HPP
#define RUSTY_FN_HPP

#include <cstddef>
#include <type_traits>
#include <utility>

// rusty::SafeFn<Signature> - Type-safe wrapper for function pointers to @safe functions
// rusty::UnsafeFn<Signature> - Type-safe wrapper for function pointers to @unsafe functions
//
// These types encode function pointer safety at the type level, enabling the
// RustyCpp analyzer to track safety through function pointers.
//
// Key differences from rusty::Function:
// - SafeFn/UnsafeFn are thin wrappers around raw function pointers (no type erasure)
// - They encode safety information in the type system
// - Zero overhead (just a pointer, inline call)
// - Not type-erased (can only hold functions with exact matching signature)
//
// Usage:
//   // @safe
//   void safe_func(int x);
//
//   // @unsafe
//   void unsafe_func(int x);
//
//   // @safe
//   void example() {
//       SafeFn<void(int)> sf = &safe_func;  // OK - target is @safe
//       sf(42);  // OK - calling SafeFn is safe
//
//       UnsafeFn<void(int)> uf = &unsafe_func;  // OK
//       // uf(42);  // ERROR - UnsafeFn has no operator()
//       // @unsafe
//       {
//           uf.call_unsafe(42);  // OK in @unsafe block
//       }
//   }
//
// This mirrors Rust's distinction between fn() and unsafe fn() types.

// @safe
namespace rusty {

// ============================================================================
// SafeFn - Wrapper for pointers to @safe functions
// ============================================================================

template<typename Signature>
class SafeFn;

/// SafeFn<Ret(Args...)> - A function pointer that is safe to call
///
/// The RustyCpp analyzer will verify at assignment time that the target
/// function is marked @safe. Once stored in a SafeFn, calling it is always safe.
///
/// @safe
template<typename Ret, typename... Args>
class SafeFn<Ret(Args...)> {
public:
    using signature = Ret(Args...);
    using pointer = Ret (*)(Args...);
    using result_type = Ret;

private:
    pointer ptr_;

public:
    /// @safe - Default constructor (null)
    constexpr SafeFn() noexcept : ptr_(nullptr) {}

    /// @safe - Nullptr constructor
    constexpr SafeFn(std::nullptr_t) noexcept : ptr_(nullptr) {}

    /// @safe - Construct from function pointer
    /// The analyzer will verify the target function is @safe
    constexpr SafeFn(pointer fn) noexcept : ptr_(fn) {}

    /// @safe - Copy constructor
    constexpr SafeFn(const SafeFn&) noexcept = default;

    /// @safe - Copy assignment
    constexpr SafeFn& operator=(const SafeFn&) noexcept = default;

    /// @safe - Assign from function pointer
    constexpr SafeFn& operator=(pointer fn) noexcept {
        ptr_ = fn;
        return *this;
    }

    /// @safe - Assign nullptr
    constexpr SafeFn& operator=(std::nullptr_t) noexcept {
        ptr_ = nullptr;
        return *this;
    }

    /// @safe - Call the function
    /// Always safe because the analyzer verified the target is @safe
    Ret operator()(Args... args) const {
        return ptr_(std::forward<Args>(args)...);
    }

    /// @safe - Check if non-null
    constexpr explicit operator bool() const noexcept {
        return ptr_ != nullptr;
    }

    /// @safe - Get the underlying pointer
    /// Returns the raw function pointer (for interop with C APIs)
    constexpr pointer get() const noexcept {
        return ptr_;
    }

    /// @safe - Comparison operators
    constexpr bool operator==(SafeFn other) const noexcept {
        return ptr_ == other.ptr_;
    }

    constexpr bool operator!=(SafeFn other) const noexcept {
        return ptr_ != other.ptr_;
    }

    constexpr bool operator==(std::nullptr_t) const noexcept {
        return ptr_ == nullptr;
    }

    constexpr bool operator!=(std::nullptr_t) const noexcept {
        return ptr_ != nullptr;
    }
};

// ============================================================================
// UnsafeFn - Wrapper for pointers to @unsafe functions
// ============================================================================

template<typename Signature>
class UnsafeFn;

/// UnsafeFn<Ret(Args...)> - A function pointer that requires @unsafe to call
///
/// Can hold any function pointer. Calling requires explicit use of call_unsafe()
/// within an @unsafe block.
///
/// @safe (the type itself is safe; only calling requires @unsafe)
template<typename Ret, typename... Args>
class UnsafeFn<Ret(Args...)> {
public:
    using signature = Ret(Args...);
    using pointer = Ret (*)(Args...);
    using result_type = Ret;

private:
    pointer ptr_;

public:
    /// @safe - Default constructor (null)
    constexpr UnsafeFn() noexcept : ptr_(nullptr) {}

    /// @safe - Nullptr constructor
    constexpr UnsafeFn(std::nullptr_t) noexcept : ptr_(nullptr) {}

    /// @safe - Construct from function pointer
    /// Can hold any function (safe or unsafe)
    constexpr UnsafeFn(pointer fn) noexcept : ptr_(fn) {}

    /// @safe - Construct from SafeFn (safe functions can be stored as unsafe)
    constexpr UnsafeFn(SafeFn<Ret(Args...)> safe_fn) noexcept : ptr_(safe_fn.get()) {}

    /// @safe - Copy constructor
    constexpr UnsafeFn(const UnsafeFn&) noexcept = default;

    /// @safe - Copy assignment
    constexpr UnsafeFn& operator=(const UnsafeFn&) noexcept = default;

    /// @safe - Assign from function pointer
    constexpr UnsafeFn& operator=(pointer fn) noexcept {
        ptr_ = fn;
        return *this;
    }

    /// @safe - Assign from SafeFn
    constexpr UnsafeFn& operator=(SafeFn<Ret(Args...)> safe_fn) noexcept {
        ptr_ = safe_fn.get();
        return *this;
    }

    /// @safe - Assign nullptr
    constexpr UnsafeFn& operator=(std::nullptr_t) noexcept {
        ptr_ = nullptr;
        return *this;
    }

    // NOTE: No operator() - calling must go through call_unsafe()
    // This prevents accidental unsafe calls

    /// @unsafe - Call the function (requires @unsafe context)
    /// Use this method inside an @unsafe block
    Ret call_unsafe(Args... args) const {
        return ptr_(std::forward<Args>(args)...);
    }

    /// @safe - Check if non-null
    constexpr explicit operator bool() const noexcept {
        return ptr_ != nullptr;
    }

    /// @safe - Get the underlying pointer
    constexpr pointer get() const noexcept {
        return ptr_;
    }

    /// @safe - Comparison operators
    constexpr bool operator==(UnsafeFn other) const noexcept {
        return ptr_ == other.ptr_;
    }

    constexpr bool operator!=(UnsafeFn other) const noexcept {
        return ptr_ != other.ptr_;
    }

    constexpr bool operator==(std::nullptr_t) const noexcept {
        return ptr_ == nullptr;
    }

    constexpr bool operator!=(std::nullptr_t) const noexcept {
        return ptr_ != nullptr;
    }
};

// ============================================================================
// SafeMemFn - Wrapper for member function pointers to @safe methods
// ============================================================================

template<typename Signature>
class SafeMemFn;

/// SafeMemFn for non-const member functions
/// @safe
template<typename Ret, typename Class, typename... Args>
class SafeMemFn<Ret (Class::*)(Args...)> {
public:
    using signature = Ret (Class::*)(Args...);
    using pointer = Ret (Class::*)(Args...);
    using result_type = Ret;
    using class_type = Class;

private:
    pointer ptr_;

public:
    /// @safe - Default constructor
    constexpr SafeMemFn() noexcept : ptr_(nullptr) {}

    /// @safe - Nullptr constructor
    constexpr SafeMemFn(std::nullptr_t) noexcept : ptr_(nullptr) {}

    /// @safe - Construct from member function pointer
    constexpr SafeMemFn(pointer fn) noexcept : ptr_(fn) {}

    /// @safe - Call on object reference
    Ret operator()(Class& obj, Args... args) const {
        return (obj.*ptr_)(std::forward<Args>(args)...);
    }

    /// @safe - Call on object pointer
    Ret operator()(Class* obj, Args... args) const {
        return (obj->*ptr_)(std::forward<Args>(args)...);
    }

    /// @safe - Check if non-null
    constexpr explicit operator bool() const noexcept {
        return ptr_ != nullptr;
    }

    /// @safe - Get the underlying pointer
    constexpr pointer get() const noexcept {
        return ptr_;
    }
};

/// SafeMemFn for const member functions
/// @safe
template<typename Ret, typename Class, typename... Args>
class SafeMemFn<Ret (Class::*)(Args...) const> {
public:
    using signature = Ret (Class::*)(Args...) const;
    using pointer = Ret (Class::*)(Args...) const;
    using result_type = Ret;
    using class_type = Class;

private:
    pointer ptr_;

public:
    /// @safe - Default constructor
    constexpr SafeMemFn() noexcept : ptr_(nullptr) {}

    /// @safe - Nullptr constructor
    constexpr SafeMemFn(std::nullptr_t) noexcept : ptr_(nullptr) {}

    /// @safe - Construct from member function pointer
    constexpr SafeMemFn(pointer fn) noexcept : ptr_(fn) {}

    /// @safe - Call on const object reference
    Ret operator()(const Class& obj, Args... args) const {
        return (obj.*ptr_)(std::forward<Args>(args)...);
    }

    /// @safe - Call on const object pointer
    Ret operator()(const Class* obj, Args... args) const {
        return (obj->*ptr_)(std::forward<Args>(args)...);
    }

    /// @safe - Check if non-null
    constexpr explicit operator bool() const noexcept {
        return ptr_ != nullptr;
    }

    /// @safe - Get the underlying pointer
    constexpr pointer get() const noexcept {
        return ptr_;
    }
};

// ============================================================================
// UnsafeMemFn - Wrapper for member function pointers to @unsafe methods
// ============================================================================

template<typename Signature>
class UnsafeMemFn;

/// UnsafeMemFn for non-const member functions
/// @safe (type is safe; calling requires @unsafe)
template<typename Ret, typename Class, typename... Args>
class UnsafeMemFn<Ret (Class::*)(Args...)> {
public:
    using signature = Ret (Class::*)(Args...);
    using pointer = Ret (Class::*)(Args...);
    using result_type = Ret;
    using class_type = Class;

private:
    pointer ptr_;

public:
    /// @safe - Default constructor
    constexpr UnsafeMemFn() noexcept : ptr_(nullptr) {}

    /// @safe - Nullptr constructor
    constexpr UnsafeMemFn(std::nullptr_t) noexcept : ptr_(nullptr) {}

    /// @safe - Construct from member function pointer
    constexpr UnsafeMemFn(pointer fn) noexcept : ptr_(fn) {}

    /// @safe - Construct from SafeMemFn
    constexpr UnsafeMemFn(SafeMemFn<Ret (Class::*)(Args...)> safe_fn) noexcept
        : ptr_(safe_fn.get()) {}

    // No operator() - must use call_unsafe

    /// @unsafe - Call on object reference
    Ret call_unsafe(Class& obj, Args... args) const {
        return (obj.*ptr_)(std::forward<Args>(args)...);
    }

    /// @unsafe - Call on object pointer
    Ret call_unsafe(Class* obj, Args... args) const {
        return (obj->*ptr_)(std::forward<Args>(args)...);
    }

    /// @safe - Check if non-null
    constexpr explicit operator bool() const noexcept {
        return ptr_ != nullptr;
    }

    /// @safe - Get the underlying pointer
    constexpr pointer get() const noexcept {
        return ptr_;
    }
};

/// UnsafeMemFn for const member functions
/// @safe (type is safe; calling requires @unsafe)
template<typename Ret, typename Class, typename... Args>
class UnsafeMemFn<Ret (Class::*)(Args...) const> {
public:
    using signature = Ret (Class::*)(Args...) const;
    using pointer = Ret (Class::*)(Args...) const;
    using result_type = Ret;
    using class_type = Class;

private:
    pointer ptr_;

public:
    /// @safe - Default constructor
    constexpr UnsafeMemFn() noexcept : ptr_(nullptr) {}

    /// @safe - Nullptr constructor
    constexpr UnsafeMemFn(std::nullptr_t) noexcept : ptr_(nullptr) {}

    /// @safe - Construct from member function pointer
    constexpr UnsafeMemFn(pointer fn) noexcept : ptr_(fn) {}

    /// @safe - Construct from SafeMemFn
    constexpr UnsafeMemFn(SafeMemFn<Ret (Class::*)(Args...) const> safe_fn) noexcept
        : ptr_(safe_fn.get()) {}

    // No operator() - must use call_unsafe

    /// @unsafe - Call on const object reference
    Ret call_unsafe(const Class& obj, Args... args) const {
        return (obj.*ptr_)(std::forward<Args>(args)...);
    }

    /// @unsafe - Call on const object pointer
    Ret call_unsafe(const Class* obj, Args... args) const {
        return (obj->*ptr_)(std::forward<Args>(args)...);
    }

    /// @safe - Check if non-null
    constexpr explicit operator bool() const noexcept {
        return ptr_ != nullptr;
    }

    /// @safe - Get the underlying pointer
    constexpr pointer get() const noexcept {
        return ptr_;
    }
};

// ============================================================================
// Deduction guides (C++17)
// ============================================================================

// Deduce SafeFn from function pointer
template<typename Ret, typename... Args>
SafeFn(Ret (*)(Args...)) -> SafeFn<Ret(Args...)>;

// Deduce UnsafeFn from function pointer
template<typename Ret, typename... Args>
UnsafeFn(Ret (*)(Args...)) -> UnsafeFn<Ret(Args...)>;

// Deduce SafeMemFn from member function pointer
template<typename Ret, typename Class, typename... Args>
SafeMemFn(Ret (Class::*)(Args...)) -> SafeMemFn<Ret (Class::*)(Args...)>;

template<typename Ret, typename Class, typename... Args>
SafeMemFn(Ret (Class::*)(Args...) const) -> SafeMemFn<Ret (Class::*)(Args...) const>;

// Deduce UnsafeMemFn from member function pointer
template<typename Ret, typename Class, typename... Args>
UnsafeMemFn(Ret (Class::*)(Args...)) -> UnsafeMemFn<Ret (Class::*)(Args...)>;

template<typename Ret, typename Class, typename... Args>
UnsafeMemFn(Ret (Class::*)(Args...) const) -> UnsafeMemFn<Ret (Class::*)(Args...) const>;

// ============================================================================
// nullptr comparison operators
// ============================================================================

template<typename Sig>
constexpr bool operator==(std::nullptr_t, SafeFn<Sig> fn) noexcept {
    return fn == nullptr;
}

template<typename Sig>
constexpr bool operator!=(std::nullptr_t, SafeFn<Sig> fn) noexcept {
    return fn != nullptr;
}

template<typename Sig>
constexpr bool operator==(std::nullptr_t, UnsafeFn<Sig> fn) noexcept {
    return fn == nullptr;
}

template<typename Sig>
constexpr bool operator!=(std::nullptr_t, UnsafeFn<Sig> fn) noexcept {
    return fn != nullptr;
}

} // namespace rusty

#endif // RUSTY_FN_HPP
