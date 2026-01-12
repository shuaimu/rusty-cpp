#ifndef RUSTY_RESULT_HPP
#define RUSTY_RESULT_HPP

#include <utility>
#include <stdexcept>
#include <new>
#include <type_traits>

// Result<T, E> - Represents either success (Ok) or failure (Err)
// Equivalent to Rust's Result<T, E>
//
// Guarantees:
// - Explicit error handling
// - No hidden exceptions
// - Composable error propagation
//
// Note: This header is marked @unsafe because it implements low-level
// data structures using raw pointers internally.

// @unsafe
namespace rusty {

template<typename T, typename E>
class Result {
private:
    // Use aligned storage for union-like behavior (C++11 compatible)
    union Storage {
        typename std::aligned_storage<sizeof(T), alignof(T)>::type ok_storage;
        typename std::aligned_storage<sizeof(E), alignof(E)>::type err_storage;
        
        Storage() {}
        ~Storage() {}
    } storage;
    
    bool is_ok_value;
    
    T& ok_ref() { return *reinterpret_cast<T*>(&storage.ok_storage); }
    const T& ok_ref() const { return *reinterpret_cast<const T*>(&storage.ok_storage); }
    E& err_ref() { return *reinterpret_cast<E*>(&storage.err_storage); }
    const E& err_ref() const { return *reinterpret_cast<const E*>(&storage.err_storage); }
    
    void destroy() {
        if (is_ok_value) {
            ok_ref().~T();
        } else {
            err_ref().~E();
        }
    }
    
public:
    // Constructors for Ok variant
    static Result Ok(T value) {
        Result r;
        new (&r.storage.ok_storage) T(std::move(value));
        r.is_ok_value = true;
        return r;
    }
    
    // Constructors for Err variant
    static Result Err(E error) {
        Result r;
        new (&r.storage.err_storage) E(std::move(error));
        r.is_ok_value = false;
        return r;
    }
    
    // Default constructor (creates Err with default E)
    Result() : is_ok_value(false) {
        new (&storage.err_storage) E();
    }
    
    // Copy constructor
    Result(const Result& other) : is_ok_value(other.is_ok_value) {
        if (is_ok_value) {
            new (&storage.ok_storage) T(other.ok_ref());
        } else {
            new (&storage.err_storage) E(other.err_ref());
        }
    }
    
    // Move constructor
    Result(Result&& other) noexcept : is_ok_value(other.is_ok_value) {
        if (is_ok_value) {
            new (&storage.ok_storage) T(std::move(other.ok_ref()));
        } else {
            new (&storage.err_storage) E(std::move(other.err_ref()));
        }
    }
    
    // Copy assignment
    Result& operator=(const Result& other) {
        if (this != &other) {
            destroy();
            is_ok_value = other.is_ok_value;
            if (is_ok_value) {
                new (&storage.ok_storage) T(other.ok_ref());
            } else {
                new (&storage.err_storage) E(other.err_ref());
            }
        }
        return *this;
    }
    
    // Move assignment
    Result& operator=(Result&& other) noexcept {
        if (this != &other) {
            destroy();
            is_ok_value = other.is_ok_value;
            if (is_ok_value) {
                new (&storage.ok_storage) T(std::move(other.ok_ref()));
            } else {
                new (&storage.err_storage) E(std::move(other.err_ref()));
            }
        }
        return *this;
    }
    
    // Destructor
    ~Result() {
        destroy();
    }
    
    // Check if Result is Ok
    bool is_ok() const { return is_ok_value; }
    
    // Check if Result is Err
    bool is_err() const { return !is_ok_value; }
    
    // Unwrap Ok value (panics if Err)
    T unwrap() {
        if (!is_ok_value) {
            throw std::runtime_error("Called unwrap on an Err value");
        }
        return std::move(ok_ref());
    }
    
    // Unwrap Err value (panics if Ok)
    E unwrap_err() {
        if (is_ok_value) {
            throw std::runtime_error("Called unwrap_err on an Ok value");
        }
        return std::move(err_ref());
    }
    
    // Unwrap Ok value or return default
    T unwrap_or(T default_value) {
        if (is_ok_value) {
            return std::move(ok_ref());
        }
        return std::move(default_value);
    }
    
    // Map over Ok value
    template<typename F>
    auto map(F f) -> Result<decltype(f(std::declval<T>())), E> {
        using NewT = decltype(f(std::declval<T>()));
        if (is_ok_value) {
            return Result<NewT, E>::Ok(f(ok_ref()));
        } else {
            return Result<NewT, E>::Err(err_ref());
        }
    }
    
    // Map over Err value
    template<typename F>
    auto map_err(F f) -> Result<T, decltype(f(std::declval<E>()))> {
        using NewE = decltype(f(std::declval<E>()));
        if (is_ok_value) {
            return Result<T, NewE>::Ok(ok_ref());
        } else {
            return Result<T, NewE>::Err(f(err_ref()));
        }
    }
    
    // Chain operations that return Result
    template<typename F>
    auto and_then(F f) -> decltype(f(std::declval<T>())) {
        using ReturnType = decltype(f(std::declval<T>()));
        if (is_ok_value) {
            return f(ok_ref());
        } else {
            return ReturnType::Err(err_ref());
        }
    }
    
    // Provide alternative Result if this is Err
    template<typename F>
    Result or_else(F f) {
        if (is_ok_value) {
            return *this;
        } else {
            return f(err_ref());
        }
    }
    
    // Explicit bool conversion - true if Ok
    explicit operator bool() const {
        return is_ok_value;
    }
};

// Specialization for Result<void, E>
template<typename E>
class Result<void, E> {
private:
    union Storage {
        typename std::aligned_storage<sizeof(E), alignof(E)>::type err_storage;
        
        Storage() {}
        ~Storage() {}
    } storage;
    
    bool is_ok_value;
    
    E& err_ref() { return *reinterpret_cast<E*>(&storage.err_storage); }
    const E& err_ref() const { return *reinterpret_cast<const E*>(&storage.err_storage); }
    
    void destroy() {
        if (!is_ok_value) {
            err_ref().~E();
        }
    }
    
public:
    // Constructor for Ok variant
    static Result Ok() {
        Result r;
        r.is_ok_value = true;
        return r;
    }
    
    // Constructor for Err variant
    static Result Err(E error) {
        Result r;
        new (&r.storage.err_storage) E(std::move(error));
        r.is_ok_value = false;
        return r;
    }
    
    // Default constructor (creates Ok)
    Result() : is_ok_value(true) {}
    
    // Copy constructor
    Result(const Result& other) : is_ok_value(other.is_ok_value) {
        if (!is_ok_value) {
            new (&storage.err_storage) E(other.err_ref());
        }
    }
    
    // Move constructor
    Result(Result&& other) noexcept : is_ok_value(other.is_ok_value) {
        if (!is_ok_value) {
            new (&storage.err_storage) E(std::move(other.err_ref()));
        }
    }
    
    // Destructor
    ~Result() {
        destroy();
    }
    
    // Check if Result is Ok
    bool is_ok() const { return is_ok_value; }
    
    // Check if Result is Err
    bool is_err() const { return !is_ok_value; }
    
    // Unwrap Err value (panics if Ok)
    E unwrap_err() {
        if (is_ok_value) {
            throw std::runtime_error("Called unwrap_err on an Ok value");
        }
        return std::move(err_ref());
    }
    
    // Explicit bool conversion - true if Ok
    explicit operator bool() const {
        return is_ok_value;
    }
};

// Helper function to create Ok Result
template<typename T, typename E>
Result<T, E> Ok(T value) {
    return Result<T, E>::Ok(std::move(value));
}

// Helper function to create Err Result
template<typename T, typename E>
Result<T, E> Err(E error) {
    return Result<T, E>::Err(std::move(error));
}

} // namespace rusty

#endif // RUSTY_RESULT_HPP