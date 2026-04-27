#ifndef RUSTY_OPTION_HPP
#define RUSTY_OPTION_HPP

#include <optional>
#include <utility>
#include <stdexcept>
#include <cstdlib>
#include <iterator>
#include <type_traits>

// Option<T> - Represents an optional value
// Equivalent to Rust's Option<T>
//
// Guarantees:
// - Type-safe null handling
// - No null pointer dereferencing
// - Explicit handling of absence
//
// Note: This header is marked @unsafe because it implements low-level
// data structures using raw pointers internally. The external API is
// designed to be safe, but the implementation requires unsafe operations.

// @unsafe
namespace rusty {

// Forward declaration for Result (defined in result.hpp, included later)
template<typename T, typename E> class Result;

template<typename X>
struct option_is_result : std::false_type {};

template<typename T, typename E>
struct option_is_result<Result<T, E>> : std::true_type {};

template<typename X>
inline constexpr bool option_is_result_v = option_is_result<std::remove_cvref_t<X>>::value;

template<typename X>
using option_result_ok_t = typename std::remove_cvref_t<X>::ok_type;

template<typename X>
using option_result_err_t = typename std::remove_cvref_t<X>::err_type;

// Tag types for Option variants
struct None_t {
    constexpr None_t() noexcept = default;
};
#if __cplusplus >= 201703L
inline constexpr None_t None{};
#else
static const None_t None{};
#endif

template<typename T>
class Option {
private:
    bool has_value;
    union {
        T value;
        char dummy;  // For when there's no value
    };
    
public:
    using value_type = T;

    static Option none() { return Option(None); }

    // Constructors
    Option() : has_value(false), dummy(0) {}
    
    Option(None_t) : has_value(false), dummy(0) {}

    Option(std::nullopt_t) : has_value(false), dummy(0) {}
    
    Option(T val) : has_value(true), value(std::move(val)) {}

    template<typename U>
    requires (!std::is_same_v<U, T> && std::is_constructible_v<T, U&&>)
    Option(Option<U>&& other) : has_value(other.is_some()) {
        if (has_value) {
            new (&value) T(std::move(other.unwrap()));
        } else {
            dummy = 0;
        }
    }

    template<typename U>
    requires (!std::is_same_v<U, T> && !std::is_constructible_v<T, U&&>)
    Option(Option<U>&& other) : has_value(false), dummy(0) {
        if (other.is_some()) {
            throw std::runtime_error("invalid Option conversion with value");
        }
    }

    template<typename U>
    requires (!std::is_same_v<U, T> && !std::is_constructible_v<T, const U&>)
    Option(const Option<U>& other) : has_value(false), dummy(0) {
        if (other.is_some()) {
            throw std::runtime_error("invalid Option conversion with value");
        }
    }

    template<typename U>
    requires std::is_same_v<std::remove_cvref_t<U>, std::optional<T>>
    Option(U&& opt) : has_value(opt.has_value()) {
        if (has_value) {
            if constexpr (std::is_lvalue_reference_v<U>) {
                new (&value) T(*opt);
            } else {
                new (&value) T(std::move(*opt));
            }
        } else {
            dummy = 0;
        }
    }
    
    // Copy constructor with clone()-fallback for move-only Rust-like payloads.
    Option(const Option& other) : has_value(other.has_value) {
        if (has_value) {
            if constexpr (std::is_copy_constructible_v<T>) {
                new (&value) T(other.value);
            } else if constexpr (requires(const T& v) { v.clone(); }) {
                new (&value) T(other.value.clone());
            } else {
                static_assert(
                    std::is_copy_constructible_v<T>,
                    "Option copy requires copy-constructible or clone()-able payload");
            }
        } else {
            dummy = 0;
        }
    }

    // Move constructor
    Option(Option&& other) noexcept : has_value(other.has_value) {
        if (has_value) {
            new (&value) T(std::move(other.value));
            other.has_value = false;
        }
    }

    // Copy assignment with clone()-fallback for move-only Rust-like payloads.
    Option& operator=(const Option& other) {
        if (this != &other) {
            if (has_value) value.~T();
            has_value = other.has_value;
            if (has_value) {
                if constexpr (std::is_copy_constructible_v<T>) {
                    new (&value) T(other.value);
                } else if constexpr (requires(const T& v) { v.clone(); }) {
                    new (&value) T(other.value.clone());
                } else {
                    static_assert(
                        std::is_copy_constructible_v<T>,
                        "Option copy assignment requires copy-constructible or clone()-able payload");
                }
            } else {
                dummy = 0;
            }
        }
        return *this;
    }

    // Move assignment
    // @lifetime: (&'a mut self) -> &'a mut self
    Option& operator=(Option&& other) noexcept {
        if (this != &other) {
            if (has_value) {
                value.~T();
            }
            has_value = other.has_value;
            if (has_value) {
                new (&value) T(std::move(other.value));
                other.has_value = false;
            }
        }
        return *this;
    }

    template<typename U>
    requires (!std::is_same_v<U, T> && std::is_constructible_v<T, U&&>)
    Option& operator=(Option<U>&& other) {
        if (has_value) {
            value.~T();
        }
        has_value = other.is_some();
        if (has_value) {
            new (&value) T(std::move(other.unwrap()));
        } else {
            dummy = 0;
        }
        return *this;
    }

    template<typename U>
    requires std::is_same_v<std::remove_cvref_t<U>, std::optional<T>>
    Option& operator=(U&& opt) {
        if (has_value) {
            value.~T();
        }
        has_value = opt.has_value();
        if (has_value) {
            if constexpr (std::is_lvalue_reference_v<U>) {
                new (&value) T(*opt);
            } else {
                new (&value) T(std::move(*opt));
            }
        } else {
            dummy = 0;
        }
        return *this;
    }
    
    // Destructor
    ~Option() {
        if (has_value) {
            value.~T();
        }
    }
    
    // Check if Option contains a value
    bool is_some() const { return has_value; }
    bool is_none() const { return !has_value; }
    bool is_ok() const { return has_value; }
    bool is_err() const { return !has_value; }

    // Clone the Option (explicit copy) - Rust style
    // Requires T to be copyable
    // @lifetime: owned
    Option clone() const {
        if (has_value) {
            if constexpr (std::is_copy_constructible_v<T>) {
                return Option(T(value));
            } else if constexpr (requires(const T& v) { v.clone(); }) {
                return Option(T(value.clone()));
            } else {
                static_assert(
                    std::is_copy_constructible_v<T>,
                    "Option::clone requires copy-constructible or clone()-able payload");
            }
        }
        return Option();
    }

    // Explicit bool conversion
    explicit operator bool() const { return has_value; }

    // Compatibility shims for transpiled `*option_expr` / `option_expr->...`
    // shapes that should preserve Option semantics rather than payload access.
    Option& operator*() & { return *this; }
    const Option& operator*() const & { return *this; }
    Option&& operator*() && { return std::move(*this); }
    Option* operator->() { return this; }
    const Option* operator->() const { return this; }
    
    // Unwrap the value (panics if None) - Rust style
    // @lifetime: owned
    T unwrap() {
        if (!has_value) {
            throw std::runtime_error("Called unwrap on None");
        }
        T result = std::move(value);
        value.~T();
        has_value = false;
        return result;
    }

    // Const unwrap - returns const reference to inner value (for borrowed access)
    const T& unwrap() const {
        if (!has_value) {
            throw std::runtime_error("Called unwrap on None");
        }
        return value;
    }

    // Unsafe Rust parity helper. Runtime checks remain for now.
    T unwrap_unchecked() { return unwrap(); }

    // Unsafe Rust parity helper for borrowed access.
    const T& unwrap_unchecked() const { return unwrap(); }

    // Expect with custom message - Rust style
    // @lifetime: owned
    T expect(const char* msg) {
        if (!has_value) {
            throw std::runtime_error(msg);
        }
        return unwrap();
    }
    
    // Unwrap with default value
    // @lifetime: owned
    T unwrap_or(T default_value) {
        if (has_value) {
            return unwrap();
        }
        return default_value;
    }

    // Const fallback for call sites that only have borrowed Option access.
    T unwrap_or(T default_value) const {
        if (has_value) {
            return value;
        }
        return default_value;
    }

    // Lazily compute fallback value only for None.
    template<typename F>
    T unwrap_or_else(F&& default_fn) {
        if (has_value) {
            return unwrap();
        }
        using fallback_result_t = std::invoke_result_t<F&&>;
        if constexpr (std::is_void_v<fallback_result_t>) {
            // Diverging Rust fallbacks (for example unreachable!()) can lower as void.
            std::forward<F>(default_fn)();
            std::abort();
        } else {
            return std::forward<F>(default_fn)();
        }
    }

    // Map function over the value
    template<typename F>
    // @lifetime: owned
    auto map(F&& f) -> Option<decltype(f(std::declval<T>()))> {
        using U = decltype(f(std::declval<T>()));
        if (has_value) {
            return Option<U>(f(std::move(value)));
        }
        return Option<U>(None);
    }
    
    // and_then: flatMap — f returns Option<U>, None propagates
    template<typename F>
    auto and_then(F&& f) {
        using ResultType = decltype(f(std::move(value)));
        if (has_value) {
            return f(std::move(value));
        }
        return ResultType(None);
    }

    // and_then on const reference
    template<typename F>
    auto and_then(F&& f) const {
        using ResultType = decltype(f(value));
        if (has_value) {
            return f(value);
        }
        return ResultType(None);
    }

    // Rust parity: Option::or(self, optb) -> Option<T>
    Option or_(Option other) {
        if (has_value) {
            return Option(std::move(value));
        }
        return other;
    }

    // Rust parity: Option::or_else(self, f) -> Option<T>
    template<typename F>
    Option or_else(F&& f) {
        if (has_value) {
            return Option(std::move(value));
        }
        return std::forward<F>(f)();
    }

    template<typename F>
    Option or_else(F&& f) const {
        if (has_value) {
            return Option(value);
        }
        return std::forward<F>(f)();
    }

    // Rust parity: Option::get_or_insert(self, value) -> &mut T
    T& get_or_insert(T default_value) {
        if (!has_value) {
            new (&value) T(std::move(default_value));
            has_value = true;
        }
        return value;
    }

    // Rust parity: Option::get_or_insert_with(self, f) -> &mut T
    template<typename F>
    T& get_or_insert_with(F&& f) {
        if (!has_value) {
            new (&value) T(std::forward<F>(f)());
            has_value = true;
        }
        return value;
    }

    // ok_or_else: convert Option<T> to Result<T, E> using closure for error
    template<typename E>
    auto ok_or(E err) -> Result<T, E> {
        if (has_value) {
            return Result<T, E>::Ok(std::move(value));
        }
        return Result<T, E>::Err(std::move(err));
    }

    // ok_or on const reference
    template<typename E>
    auto ok_or(E err) const -> Result<T, E> {
        if (has_value) {
            return Result<T, E>::Ok(value);
        }
        return Result<T, E>::Err(std::move(err));
    }

    // ok_or_else: convert Option<T> to Result<T, E> using closure for error
    template<typename F>
    auto ok_or_else(F&& err_fn) -> Result<T, decltype(err_fn())> {
        using E = decltype(err_fn());
        if (has_value) {
            return Result<T, E>::Ok(std::move(value));
        }
        return Result<T, E>::Err(err_fn());
    }

    // ok_or_else on const reference
    template<typename F>
    auto ok_or_else(F&& err_fn) const -> Result<T, decltype(err_fn())> {
        using E = decltype(err_fn());
        if (has_value) {
            return Result<T, E>::Ok(value);
        }
        return Result<T, E>::Err(err_fn());
    }

    // Map function over reference
    template<typename F>
    // @lifetime: (&'a) -> owned
    auto map_ref(F&& f) const -> Option<decltype(f(std::declval<const T&>()))> {
        using U = decltype(f(std::declval<const T&>()));
        if (has_value) {
            return Option<U>(f(value));
        }
        return Option<U>(None);
    }

    template<typename U, typename F>
    U map_or(U default_value, F&& f) const {
        if (has_value) {
            return f(value);
        }
        return default_value;
    }

    template<typename D, typename F>
    auto map_or_else(D&& default_fn, F&& f) const -> decltype(f(value)) {
        if (has_value) {
            return f(value);
        }
        return default_fn();
    }

    // Rust parity: Option<Result<T, E>>::transpose(self) -> Result<Option<T>, E>
    template<typename Q = T>
    auto transpose() -> Result<Option<option_result_ok_t<Q>>, option_result_err_t<Q>>
    requires option_is_result_v<Q> {
        using InnerOk = option_result_ok_t<Q>;
        using InnerErr = option_result_err_t<Q>;

        if (!has_value) {
            return Result<Option<InnerOk>, InnerErr>::Ok(Option<InnerOk>(None));
        }

        auto inner = std::move(value);
        if (inner.is_ok()) {
            return Result<Option<InnerOk>, InnerErr>::Ok(Option<InnerOk>(inner.unwrap()));
        }
        return Result<Option<InnerOk>, InnerErr>::Err(inner.unwrap_err());
    }
    
    // Take the value out, leaving None
    // @lifetime: owned
    Option<T> take() {
        Option<T> result = std::move(*this);
        *this = Option<T>(None);
        return result;
    }
    
    // Replace the value
    void replace(T new_value) {
        if (has_value) {
            value = std::move(new_value);
        } else {
            // @unsafe
            {
                new (&value) T(std::move(new_value));
            }
            has_value = true;
        }
    }

    // Convert to Option<T&> without consuming the value
    // @lifetime: (&'a) -> Option<&'a T>
    Option<T&> as_ref() & {
        if (has_value) {
            return Option<T&>(value);
        }
        return None;
    }

    // Convert to Option<const T&> for const access
    // @lifetime: (&'a) -> Option<&'a const T>
    Option<const T&> as_ref() const & {
        if (has_value) {
            return Option<const T&>(value);
        }
        return None;
    }

    // Convert to Option<T&> for mutable access
    // @lifetime: (&'a mut) -> Option<&'a mut T>
    Option<T&> as_mut() & {
        if (has_value) {
            return Option<T&>(value);
        }
        return None;
    }

    // Prevent calling as_ref()/as_mut() on rvalue (temporary)
    Option<T&> as_ref() && = delete;
    Option<const T&> as_ref() const && = delete;
    Option<T&> as_mut() && = delete;
};

// Template specialization for Option<T&> (reference types)
// This allows holding references without storing them in a union
// Implementation uses raw pointers internally
template<typename T>
class Option<T&> {
private:
    T* ptr;  // nullptr if None, otherwise points to the value

public:
    using value_type = T&;

    static Option none() { return Option(None); }

    // Constructors
    Option() : ptr(nullptr) {}

    
    Option(None_t) : ptr(nullptr) {}

    Option(std::nullopt_t) : ptr(nullptr) {}

    
    Option(T& ref) : ptr(&ref) {}

    template<typename U>
    requires (!std::is_same_v<std::remove_cvref_t<U>, T>)
          && requires(U& u) { *u; }
          && std::is_convertible_v<decltype(*std::declval<U&>()), T&>
    Option(U& ref_like) : ptr(&static_cast<T&>(*ref_like)) {}

    // Copy constructor - pointer/reference options are always copyable
    Option(const Option& other) = default;

    // Move constructor
    Option(Option&& other) noexcept : ptr(other.ptr) {
        other.ptr = nullptr;
    }

    // Copy assignment - pointer/reference options are always copyable
    Option& operator=(const Option& other) = default;

    // Move assignment
    // @lifetime: (&'a mut self) -> &'a mut self
    Option& operator=(Option&& other) noexcept {
        ptr = other.ptr;
        other.ptr = nullptr;
        return *this;
    }

    // Destructor (trivial for references)
    ~Option() = default;

    // Check if Option contains a value
    bool is_some() const { return ptr != nullptr; }
    bool is_none() const { return !ptr; }
    bool is_ok() const { return ptr != nullptr; }
    bool is_err() const { return !ptr; }

    // Clone the Option (explicit copy) - Rust style
    // @lifetime: owned
    Option clone() const {
        if (ptr) {
            return Option(*ptr);
        }
        return Option();
    }

    // Take the reference and leave this option as None.
    // @lifetime: owned
    Option<T&> take() {
        Option<T&> result = *this;
        ptr = nullptr;
        return result;
    }

    // Explicit bool conversion
    explicit operator bool() const { return ptr != nullptr; }

    // Compatibility shims for transpiled `*option_expr` / `option_expr->...`
    Option& operator*() & { return *this; }
    const Option& operator*() const & { return *this; }
    Option&& operator*() && { return std::move(*this); }
    Option* operator->() { return this; }
    const Option* operator->() const { return this; }

    // Unwrap the reference (panics if None)
    // @lifetime: (&'a) -> &'a T
    T& unwrap() {
        if (!ptr) {
            throw std::runtime_error("Called unwrap on None");
        }
        return *ptr;
    }

    // Const overload for borrowed Option references.
    T& unwrap() const {
        if (!ptr) {
            throw std::runtime_error("Called unwrap on None");
        }
        return *ptr;
    }

    // Unsafe Rust parity helper. Runtime checks remain for now.
    T& unwrap_unchecked() { return unwrap(); }

    // Unsafe Rust parity helper for const receivers.
    T& unwrap_unchecked() const {
        if (!ptr) {
            throw std::runtime_error("Called unwrap on None");
        }
        return *ptr;
    }

    // Expect with custom message
    // @lifetime: (&'a) -> &'a T
    T& expect(const char* msg) {
        if (!ptr) {
            throw std::runtime_error(msg);
        }
        return *ptr;
    }

    T& expect(const char* msg) const {
        if (!ptr) {
            throw std::runtime_error(msg);
        }
        return *ptr;
    }

    // Unwrap with default reference
    // @lifetime: (&'a, &'b) -> &'c T where 'a: 'c, 'b: 'c
    T& unwrap_or(T& default_ref) {
        if (ptr) {
            return *ptr;
        }
        return default_ref;
    }

    T& unwrap_or(T& default_ref) const {
        if (ptr) {
            return *ptr;
        }
        return default_ref;
    }

    template<typename F>
    T& unwrap_or_else(F&& f) {
        if (ptr) {
            return *ptr;
        }
        if constexpr (std::is_void_v<std::invoke_result_t<F>>) {
            std::forward<F>(f)();
            throw std::runtime_error("Called unwrap_or_else on None");
        } else {
            return static_cast<T&>(std::forward<F>(f)());
        }
    }

    template<typename F>
    T& unwrap_or_else(F&& f) const {
        if (ptr) {
            return *ptr;
        }
        if constexpr (std::is_void_v<std::invoke_result_t<F>>) {
            std::forward<F>(f)();
            throw std::runtime_error("Called unwrap_or_else on None");
        } else {
            return static_cast<T&>(std::forward<F>(f)());
        }
    }

    // Map function over the reference
    template<typename F>
    // @lifetime: (&'a) -> Option<U>
    auto map(F&& f) -> Option<decltype(f(std::declval<T&>()))> {
        using U = decltype(f(std::declval<T&>()));
        if (ptr) {
            return Option<U>(f(*ptr));
        }
        return Option<U>(None);
    }

    // Map function over const reference
    template<typename F>
    // @lifetime: (&'a) -> Option<U>
    auto map(F&& f) const -> Option<decltype(f(std::declval<const T&>()))> {
        using U = decltype(f(std::declval<const T&>()));
        if (ptr) {
            return Option<U>(f(*ptr));
        }
        return Option<U>(None);
    }

    // Rust parity: Option<&T>::copied() -> Option<T>
    Option<std::remove_cv_t<T>> copied() const {
        using U = std::remove_cv_t<T>;
        if (ptr) {
            return Option<U>(static_cast<U>(*ptr));
        }
        return Option<U>(None);
    }

    Option or_(Option other) const {
        if (ptr) {
            return Option(*ptr);
        }
        return other;
    }

    // as_ref() for Option<T&> returns itself (already a reference)
    // @lifetime: (&'a) -> &'a self
    Option<T&> as_ref() & {
        if (ptr) {
            return Option<T&>(*ptr);
        }
        return None;
    }

    
    // @lifetime: (&'a) -> &'a self
    Option<const T&> as_ref() const & {
        if (ptr) {
            return Option<const T&>(*ptr);
        }
        return None;
    }

    // as_mut() for Option<T&> returns itself (already mutable reference)
    // @lifetime: (&'a mut) -> &'a mut self
    Option<T&> as_mut() & {
        if (ptr) {
            return Option<T&>(*ptr);
        }
        return None;
    }

    // Prevent calling as_ref()/as_mut() on rvalue
    Option<T&> as_ref() && = delete;
    Option<const T&> as_ref() const && = delete;
    Option<T&> as_mut() && = delete;

    // Check if contains specific value
    bool contains(const T& value) const {
        return ptr && (*ptr == value);
    }
};

// Template specialization for Option<const T&> (const reference types)
// Implementation uses raw pointers internally
template<typename T>
class Option<const T&> {
private:
    const T* ptr;  // nullptr if None, otherwise points to the value

public:
    using value_type = const T&;

    static Option none() { return Option(None); }

    // Constructors
    Option() : ptr(nullptr) {}

    
    Option(None_t) : ptr(nullptr) {}

    Option(std::nullopt_t) : ptr(nullptr) {}

    
    Option(const T& ref) : ptr(&ref) {}

    template<typename U>
    requires (!std::is_same_v<std::remove_cvref_t<U>, T>)
          && requires(const U& u) { *u; }
          && std::is_convertible_v<decltype(*std::declval<const U&>()), const T&>
    Option(const U& ref_like) : ptr(&static_cast<const T&>(*ref_like)) {}

    // Convert Option<U&> -> Option<const T&> when references are compatible.
    template<typename U>
    requires std::is_convertible_v<U&, T&>
    Option(const Option<U&>& other)
        : ptr(other.is_some() ? &static_cast<const T&>(other.unwrap()) : nullptr) {}

    // Convert Option<const U&> -> Option<const T&> when references are compatible.
    template<typename U>
    requires std::is_convertible_v<const U&, const T&>
    Option(const Option<const U&>& other)
        : ptr(other.is_some() ? &static_cast<const T&>(other.unwrap()) : nullptr) {}

    // Copy constructor - pointer/reference options are always copyable
    Option(const Option& other) = default;

    // Move constructor
    Option(Option&& other) noexcept : ptr(other.ptr) {
        other.ptr = nullptr;
    }

    // Copy assignment - pointer/reference options are always copyable
    Option& operator=(const Option& other) = default;

    // Move assignment
    // @lifetime: (&'a mut self) -> &'a mut self
    Option& operator=(Option&& other) noexcept {
        ptr = other.ptr;
        other.ptr = nullptr;
        return *this;
    }

    // Destructor (trivial for references)
    ~Option() = default;

    // Check if Option contains a value
    bool is_some() const { return ptr != nullptr; }
    bool is_none() const { return !ptr; }
    bool is_ok() const { return ptr != nullptr; }
    bool is_err() const { return !ptr; }

    // Clone the Option (explicit copy) - Rust style
    // @lifetime: owned
    Option clone() const {
        if (ptr) {
            return Option(*ptr);
        }
        return Option();
    }

    // Take the reference and leave this option as None.
    // @lifetime: owned
    Option<const T&> take() {
        Option<const T&> result = *this;
        ptr = nullptr;
        return result;
    }

    // Explicit bool conversion
    explicit operator bool() const { return ptr != nullptr; }

    // Compatibility shims for transpiled `*option_expr` / `option_expr->...`
    Option& operator*() & { return *this; }
    const Option& operator*() const & { return *this; }
    Option&& operator*() && { return std::move(*this); }
    Option* operator->() { return this; }
    const Option* operator->() const { return this; }

    // Unwrap the reference (panics if None)
    // @lifetime: (&'a) -> &'a const T
    const T& unwrap() const {
        if (!ptr) {
            throw std::runtime_error("Called unwrap on None");
        }
        return *ptr;
    }

    // Unsafe Rust parity helper. Runtime checks remain for now.
    const T& unwrap_unchecked() const { return unwrap(); }

    // Expect with custom message
    // @lifetime: (&'a) -> &'a const T
    const T& expect(const char* msg) const {
        if (!ptr) {
            throw std::runtime_error(msg);
        }
        return *ptr;
    }

    // Unwrap with default reference
    // @lifetime: (&'a, &'b) -> &'c const T where 'a: 'c, 'b: 'c
    const T& unwrap_or(const T& default_ref) const {
        if (ptr) {
            return *ptr;
        }
        return default_ref;
    }

    template<typename F>
    const T& unwrap_or_else(F&& f) const {
        if (ptr) {
            return *ptr;
        }
        if constexpr (std::is_void_v<std::invoke_result_t<F>>) {
            std::forward<F>(f)();
            throw std::runtime_error("Called unwrap_or_else on None");
        } else {
            return static_cast<const T&>(std::forward<F>(f)());
        }
    }

    // Map function over the reference
    template<typename F>
    // @lifetime: (&'a) -> Option<U>
    auto map(F&& f) const -> Option<decltype(f(std::declval<const T&>()))> {
        using U = decltype(f(std::declval<const T&>()));
        if (ptr) {
            return Option<U>(f(*ptr));
        }
        return Option<U>(None);
    }

    // Rust parity: Option<&T>::copied() -> Option<T>
    Option<std::remove_cv_t<T>> copied() const {
        using U = std::remove_cv_t<T>;
        if (ptr) {
            return Option<U>(static_cast<U>(*ptr));
        }
        return Option<U>(None);
    }

    Option or_(Option other) const {
        if (ptr) {
            return Option(*ptr);
        }
        return other;
    }

    // as_ref() for Option<const T&> returns itself (already a const reference)
    // @lifetime: (&'a) -> &'a self
    Option<const T&> as_ref() const & {
        if (ptr) {
            return Option<const T&>(*ptr);
        }
        return None;
    }

    // as_mut() on Option<const T&> preserves const-reference payloads.
    // This matches Rust parity for APIs that treat reference options uniformly.
    Option<const T&> as_mut() const & {
        if (ptr) {
            return Option<const T&>(*ptr);
        }
        return None;
    }

    // Prevent calling as_ref() on rvalue
    Option<const T&> as_ref() const && = delete;
    Option<const T&> as_mut() const && = delete;

    // Check if contains specific value
    bool contains(const T& value) const {
        return ptr && (*ptr == value);
    }
};

// Helper function to create Some variant (owned value)

template<typename T>
// @lifetime: owned
Option<std::decay_t<T>> Some(T&& value) {
    using value_type = std::decay_t<T>;
    // Rust `Some(x)` moves `x`. For non-copyable lvalue payloads, mimic
    // move semantics to avoid forcing an invalid copy in generated code.
    if constexpr (std::is_lvalue_reference_v<T&&>
                  && !std::is_copy_constructible_v<value_type>
                  && std::is_move_constructible_v<value_type>) {
        return Option<value_type>(std::move(value));
    } else {
        return Option<value_type>(std::forward<T>(value));
    }
}

// Helper function to create Some variant (mutable reference)
// No std::move - references don't transfer ownership

template<typename T>
// @lifetime: (&'a mut) -> Option<&'a mut>
Option<T&> SomeRef(T& ref) {
    return Option<T&>(ref);
}

// Helper function to create Some variant (const reference)

template<typename T>
// @lifetime: (&'a) -> Option<&'a>
Option<const T&> SomeRef(const T& ref) {
    return Option<const T&>(ref);
}

// Equality operators
template<typename L, typename R>
bool option_payload_equals(const L& lhs, const R& rhs) {
    if constexpr (requires { lhs == rhs; }) {
        return lhs == rhs;
    } else if constexpr (requires { rhs == lhs; }) {
        return rhs == lhs;
    } else if constexpr (
        requires { std::begin(lhs); std::end(lhs); std::begin(rhs); std::end(rhs); }
    ) {
        auto lit = std::begin(lhs);
        auto lend = std::end(lhs);
        auto rit = std::begin(rhs);
        auto rend = std::end(rhs);
        for (; lit != lend && rit != rend; ++lit, ++rit) {
            if (!option_payload_equals(*lit, *rit)) {
                return false;
            }
        }
        return lit == lend && rit == rend;
    } else {
        return false;
    }
}

template<typename T>
bool operator==(const Option<T>& lhs, const Option<T>& rhs) {
    if (lhs.is_none() && rhs.is_none()) return true;
    if (lhs.is_some() && rhs.is_some()) {
        return option_payload_equals(lhs.as_ref().unwrap(), rhs.as_ref().unwrap());
    }
    return false;
}

template<typename T>
bool operator!=(const Option<T>& lhs, const Option<T>& rhs) {
    return !(lhs == rhs);
}

template<typename T, typename U>
requires (!std::is_same_v<T, U>)
      && (requires(const T& l, const U& r) { l == r; }
          || requires(const T& l, const U& r) { r == l; })
bool operator==(const Option<T>& lhs, const Option<U>& rhs) {
    if (lhs.is_none() && rhs.is_none()) return true;
    if (lhs.is_some() && rhs.is_some()) {
        const auto& l = lhs.as_ref().unwrap();
        const auto& r = rhs.as_ref().unwrap();
        if constexpr (requires { l == r; }) {
            return l == r;
        } else {
            return r == l;
        }
    }
    return false;
}

template<typename T, typename U>
requires (!std::is_same_v<T, U>)
      && (requires(const T& l, const U& r) { l == r; }
          || requires(const T& l, const U& r) { r == l; })
bool operator!=(const Option<T>& lhs, const Option<U>& rhs) {
    return !(lhs == rhs);
}

template<typename T>
bool operator==(const Option<T>& lhs, std::nullopt_t) {
    return lhs.is_none();
}

template<typename T>
bool operator==(std::nullopt_t, const Option<T>& rhs) {
    return rhs.is_none();
}

template<typename T>
bool operator==(const Option<T>& lhs, None_t) {
    return lhs.is_none();
}

template<typename T>
bool operator==(None_t, const Option<T>& rhs) {
    return rhs.is_none();
}

template<typename T>
bool operator!=(const Option<T>& lhs, None_t) {
    return lhs.is_some();
}

template<typename T>
bool operator!=(None_t, const Option<T>& rhs) {
    return rhs.is_some();
}

template<typename T>
bool operator!=(const Option<T>& lhs, std::nullopt_t) {
    return lhs.is_some();
}

template<typename T>
bool operator!=(std::nullopt_t, const Option<T>& rhs) {
    return rhs.is_some();
}

template<typename T, typename U>
requires requires(const T& l, const U& r) { l == r; }
bool operator==(const Option<T>& lhs, const std::optional<U>& rhs) {
    if (lhs.is_none() && !rhs.has_value()) return true;
    if (lhs.is_some() && rhs.has_value()) {
        return lhs.as_ref().unwrap() == *rhs;
    }
    return false;
}

template<typename U, typename T>
requires requires(const T& l, const U& r) { l == r; }
bool operator==(const std::optional<U>& lhs, const Option<T>& rhs) {
    return rhs == lhs;
}

template<typename T, typename U>
requires requires(const T& l, const U& r) { l == r; }
bool operator!=(const Option<T>& lhs, const std::optional<U>& rhs) {
    return !(lhs == rhs);
}

template<typename U, typename T>
requires requires(const T& l, const U& r) { l == r; }
bool operator!=(const std::optional<U>& lhs, const Option<T>& rhs) {
    return !(lhs == rhs);
}

} // namespace rusty

#endif // RUSTY_OPTION_HPP
