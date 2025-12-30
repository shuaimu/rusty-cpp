#ifndef RUSTY_OPTION_HPP
#define RUSTY_OPTION_HPP

#include <utility>
#include <stdexcept>

// Option<T> - Represents an optional value
// Equivalent to Rust's Option<T>
//
// Guarantees:
// - Type-safe null handling
// - No null pointer dereferencing
// - Explicit handling of absence

// @safe
namespace rusty {

// Tag types for Option variants
// @safe
struct None_t {
    constexpr None_t() noexcept = default;
};
#if __cplusplus >= 201703L
inline constexpr None_t None{};
#else
static const None_t None{};
#endif

// @safe
template<typename T>
class Option {
private:
    bool has_value;
    union {
        T value;
        char dummy;  // For when there's no value
    };
    
public:
    // Constructors
    Option() : has_value(false), dummy(0) {}
    
    Option(None_t) : has_value(false), dummy(0) {}
    
    Option(T val) : has_value(true), value(std::move(val)) {}
    
    // Copy constructor
    Option(const Option& other) : has_value(other.has_value) {
        if (has_value) {
            new (&value) T(other.value);
        }
    }

    // Move constructor
    Option(Option&& other) noexcept : has_value(other.has_value) {
        if (has_value) {
            new (&value) T(std::move(other.value));
            other.has_value = false;
        }
    }
    
    // Copy assignment
    Option& operator=(const Option& other) {
        if (this != &other) {
            if (has_value) {
                value.~T();
            }
            has_value = other.has_value;
            if (has_value) {
                new (&value) T(other.value);
            }
        }
        return *this;
    }

    // Move assignment
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
    
    // Destructor
    ~Option() {
        if (has_value) {
            value.~T();
        }
    }
    
    // Check if Option contains a value
    bool is_some() const { return has_value; }
    bool is_none() const { return !has_value; }
    
    // Explicit bool conversion
    explicit operator bool() const { return has_value; }
    
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
            new (&value) T(std::move(new_value));
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
// Implementation uses raw pointers, but API is safe
// @safe
template<typename T>
class Option<T&> {
private:
    T* ptr;  // nullptr if None, otherwise points to the value

public:
    // @safe - Constructors
    Option() : ptr(nullptr) {}

    // @safe
    Option(None_t) : ptr(nullptr) {}

    // @safe
    Option(T& ref) : ptr(&ref) {}

    // @safe - Copy constructor
    Option(const Option& other) : ptr(other.ptr) {}

    // @safe - Move constructor
    Option(Option&& other) noexcept : ptr(other.ptr) {
        other.ptr = nullptr;
    }

    // @safe - Copy assignment
    Option& operator=(const Option& other) {
        ptr = other.ptr;
        return *this;
    }

    // @safe - Move assignment
    Option& operator=(Option&& other) noexcept {
        ptr = other.ptr;
        other.ptr = nullptr;
        return *this;
    }

    // @safe - Destructor (trivial for references)
    ~Option() = default;

    // @safe - Check if Option contains a value
    bool is_some() const { return ptr != nullptr; }
    // @safe
    bool is_none() const { return !ptr; }

    // @safe - Explicit bool conversion
    explicit operator bool() const { return ptr != nullptr; }

    // @safe - Unwrap the reference (panics if None)
    // @lifetime: (&'a) -> &'a T
    T& unwrap() {
        if (!ptr) {
            throw std::runtime_error("Called unwrap on None");
        }
        return *ptr;
    }

    // @safe - Expect with custom message
    // @lifetime: (&'a) -> &'a T
    T& expect(const char* msg) {
        if (!ptr) {
            throw std::runtime_error(msg);
        }
        return *ptr;
    }

    // @safe - Unwrap with default reference
    // @lifetime: (&'a, &'b) -> &'c T where 'a: 'c, 'b: 'c
    T& unwrap_or(T& default_ref) {
        if (ptr) {
            return *ptr;
        }
        return default_ref;
    }

    // @safe - Map function over the reference
    template<typename F>
    // @lifetime: (&'a) -> Option<U>
    auto map(F&& f) -> Option<decltype(f(std::declval<T&>()))> {
        using U = decltype(f(std::declval<T&>()));
        if (ptr) {
            return Option<U>(f(*ptr));
        }
        return Option<U>(None);
    }

    // @safe - Map function over const reference
    template<typename F>
    // @lifetime: (&'a) -> Option<U>
    auto map(F&& f) const -> Option<decltype(f(std::declval<const T&>()))> {
        using U = decltype(f(std::declval<const T&>()));
        if (ptr) {
            return Option<U>(f(*ptr));
        }
        return Option<U>(None);
    }

    // @safe - as_ref() for Option<T&> returns itself (already a reference)
    // @lifetime: (&'a) -> &'a self
    Option<T&> as_ref() & {
        return *this;
    }

    // @safe
    // @lifetime: (&'a) -> &'a self
    Option<const T&> as_ref() const & {
        if (ptr) {
            return Option<const T&>(*ptr);
        }
        return None;
    }

    // @safe - as_mut() for Option<T&> returns itself (already mutable reference)
    // @lifetime: (&'a mut) -> &'a mut self
    Option<T&> as_mut() & {
        return *this;
    }

    // Prevent calling as_ref()/as_mut() on rvalue
    Option<T&> as_ref() && = delete;
    Option<const T&> as_ref() const && = delete;
    Option<T&> as_mut() && = delete;

    // @safe - Check if contains specific value
    bool contains(const T& value) const {
        return ptr && (*ptr == value);
    }
};

// Template specialization for Option<const T&> (const reference types)
// Implementation uses raw pointers, but API is safe
// @safe
template<typename T>
class Option<const T&> {
private:
    const T* ptr;  // nullptr if None, otherwise points to the value

public:
    // @safe - Constructors
    Option() : ptr(nullptr) {}

    // @safe
    Option(None_t) : ptr(nullptr) {}

    // @safe
    Option(const T& ref) : ptr(&ref) {}

    // @safe - Copy constructor
    Option(const Option& other) : ptr(other.ptr) {}

    // @safe - Move constructor
    Option(Option&& other) noexcept : ptr(other.ptr) {
        other.ptr = nullptr;
    }

    // @safe - Copy assignment
    Option& operator=(const Option& other) {
        ptr = other.ptr;
        return *this;
    }

    // @safe - Move assignment
    Option& operator=(Option&& other) noexcept {
        ptr = other.ptr;
        other.ptr = nullptr;
        return *this;
    }

    // @safe - Destructor (trivial for references)
    ~Option() = default;

    // @safe - Check if Option contains a value
    bool is_some() const { return ptr != nullptr; }
    // @safe
    bool is_none() const { return !ptr; }

    // @safe - Explicit bool conversion
    explicit operator bool() const { return ptr != nullptr; }

    // @safe - Unwrap the reference (panics if None)
    // @lifetime: (&'a) -> &'a const T
    const T& unwrap() const {
        if (!ptr) {
            throw std::runtime_error("Called unwrap on None");
        }
        return *ptr;
    }

    // @safe - Expect with custom message
    // @lifetime: (&'a) -> &'a const T
    const T& expect(const char* msg) const {
        if (!ptr) {
            throw std::runtime_error(msg);
        }
        return *ptr;
    }

    // @safe - Unwrap with default reference
    // @lifetime: (&'a, &'b) -> &'c const T where 'a: 'c, 'b: 'c
    const T& unwrap_or(const T& default_ref) const {
        if (ptr) {
            return *ptr;
        }
        return default_ref;
    }

    // @safe - Map function over the reference
    template<typename F>
    // @lifetime: (&'a) -> Option<U>
    auto map(F&& f) const -> Option<decltype(f(std::declval<const T&>()))> {
        using U = decltype(f(std::declval<const T&>()));
        if (ptr) {
            return Option<U>(f(*ptr));
        }
        return Option<U>(None);
    }

    // @safe - as_ref() for Option<const T&> returns itself (already a const reference)
    // @lifetime: (&'a) -> &'a self
    Option<const T&> as_ref() const & {
        return *this;
    }

    // Prevent calling as_ref() on rvalue
    Option<const T&> as_ref() const && = delete;

    // @safe - Check if contains specific value
    bool contains(const T& value) const {
        return ptr && (*ptr == value);
    }
};

// Helper function to create Some variant
// @safe
template<typename T>
// @lifetime: owned
Option<T> Some(T value) {
    return Option<T>(std::move(value));
}

// Equality operators
template<typename T>
bool operator==(const Option<T>& lhs, const Option<T>& rhs) {
    if (lhs.is_none() && rhs.is_none()) return true;
    if (lhs.is_some() && rhs.is_some()) {
        return lhs.as_ref().unwrap() == rhs.as_ref().unwrap();
    }
    return false;
}

template<typename T>
bool operator!=(const Option<T>& lhs, const Option<T>& rhs) {
    return !(lhs == rhs);
}

} // namespace rusty

#endif // RUSTY_OPTION_HPP