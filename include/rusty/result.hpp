#ifndef RUSTY_RESULT_HPP
#define RUSTY_RESULT_HPP

#include <utility>
#include <stdexcept>
#include <new>
#include <type_traits>
#include <tuple>
#include <array>
#include <cstdlib>
#include <memory>
#include <string>
#include <string_view>
#include <rusty/option.hpp>

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

template<typename X>
struct result_is_option : std::false_type {};

template<typename T>
struct result_is_option<Option<T>> : std::true_type {};

template<typename X>
inline constexpr bool result_is_option_v = result_is_option<std::remove_cvref_t<X>>::value;

template<typename X>
using result_option_value_t = typename std::remove_cvref_t<X>::value_type;

template<typename T, typename E>
class Result {
private:
    template<typename X>
    struct array_meta {
        static constexpr bool is_array = false;
        using value_type = void;
        static constexpr size_t extent = 0;
    };

    template<typename U, size_t N>
    struct array_meta<std::array<U, N>> {
        static constexpr bool is_array = true;
        using value_type = U;
        static constexpr size_t extent = N;
    };

    template<typename X>
    using stored_type_t = std::conditional_t<
        std::is_reference_v<X>,
        std::add_pointer_t<std::remove_reference_t<X>>,
        X>;

    using OkStored = stored_type_t<T>;
    using ErrStored = stored_type_t<E>;

    struct UninitTag {};

    // Use aligned storage for union-like behavior (C++11 compatible)
    union Storage {
        typename std::aligned_storage<sizeof(OkStored), alignof(OkStored)>::type ok_storage;
        typename std::aligned_storage<sizeof(ErrStored), alignof(ErrStored)>::type err_storage;
        
        Storage() {}
        ~Storage() {}
    } storage;
    
    bool is_ok_value;

    // Construct storage without materializing either payload variant.
    explicit Result(UninitTag) noexcept : is_ok_value(false) {}
    
    OkStored& ok_stored_ref() { return *reinterpret_cast<OkStored*>(&storage.ok_storage); }
    const OkStored& ok_stored_ref() const {
        return *reinterpret_cast<const OkStored*>(&storage.ok_storage);
    }
    ErrStored& err_stored_ref() { return *reinterpret_cast<ErrStored*>(&storage.err_storage); }
    const ErrStored& err_stored_ref() const {
        return *reinterpret_cast<const ErrStored*>(&storage.err_storage);
    }

    static OkStored to_ok_storage(T value) requires std::is_reference_v<T> {
        return std::addressof(value);
    }

    template<typename U>
    static OkStored to_ok_storage(U&& value) requires (!std::is_reference_v<T>) {
        return OkStored(std::forward<U>(value));
    }

    static ErrStored to_err_storage(E value) requires std::is_reference_v<E> {
        return std::addressof(value);
    }

    template<typename U>
    static ErrStored to_err_storage(U&& value) requires (!std::is_reference_v<E>) {
        return ErrStored(std::forward<U>(value));
    }

    decltype(auto) ok_ref() {
        if constexpr (std::is_reference_v<T>) {
            return *ok_stored_ref();
        } else {
            return *reinterpret_cast<std::remove_reference_t<T>*>(&storage.ok_storage);
        }
    }
    decltype(auto) ok_ref() const {
        if constexpr (std::is_reference_v<T>) {
            return *ok_stored_ref();
        } else {
            return *reinterpret_cast<const std::remove_reference_t<T>*>(&storage.ok_storage);
        }
    }
    decltype(auto) err_ref() {
        if constexpr (std::is_reference_v<E>) {
            return *err_stored_ref();
        } else {
            return *reinterpret_cast<std::remove_reference_t<E>*>(&storage.err_storage);
        }
    }
    decltype(auto) err_ref() const {
        if constexpr (std::is_reference_v<E>) {
            return *err_stored_ref();
        } else {
            return *reinterpret_cast<const std::remove_reference_t<E>*>(&storage.err_storage);
        }
    }
    
    void destroy() {
        if (is_ok_value) {
            if constexpr (!std::is_reference_v<T>) {
                ok_ref().~T();
            }
        } else {
            if constexpr (!std::is_reference_v<E>) {
                err_ref().~E();
            }
        }
    }

    template<typename Msg>
    [[noreturn]] static void throw_expect_failure(Msg&& msg) {
        if constexpr (std::is_convertible_v<Msg, std::string_view>) {
            throw std::runtime_error(std::string(std::string_view(std::forward<Msg>(msg))));
        } else {
            throw std::runtime_error("Result expectation failed");
        }
    }
    
public:
    using ok_type = T;
    using err_type = E;

    // Constructors for Ok variant
    static Result Ok(T value) requires std::is_reference_v<T> {
        Result r(UninitTag{});
        new (&r.storage.ok_storage) OkStored(to_ok_storage(value));
        r.is_ok_value = true;
        return r;
    }

    template<typename U = T>
    requires (!std::is_reference_v<T> && !std::is_lvalue_reference_v<U> && std::is_constructible_v<OkStored, U&&>)
    static Result Ok(U&& value) {
        Result r(UninitTag{});
        new (&r.storage.ok_storage) OkStored(to_ok_storage(std::forward<U>(value)));
        r.is_ok_value = true;
        return r;
    }

    // Lvalue Ok fallback for move-only payloads that expose clone().
    template<typename U = std::remove_cvref_t<T>>
    requires (!std::is_reference_v<T>)
    static Result Ok(const U& value) {
        Result r(UninitTag{});
        if constexpr (std::is_constructible_v<OkStored, const U&>) {
            new (&r.storage.ok_storage) OkStored(value);
        } else if constexpr (requires(const U& v) { v.clone(); }
                             && std::is_constructible_v<OkStored, decltype(value.clone())>) {
            new (&r.storage.ok_storage) OkStored(value.clone());
        } else {
            static_assert(
                std::is_constructible_v<OkStored, const U&>,
                "Result::Ok lvalue requires copy-constructible or clone()-able payload");
        }
        r.is_ok_value = true;
        return r;
    }

    template<typename U, size_t N>
    requires (array_meta<std::remove_cv_t<T>>::is_array
              && array_meta<std::remove_cv_t<T>>::extent == N
              && std::is_convertible_v<U, typename array_meta<std::remove_cv_t<T>>::value_type>)
    static Result Ok(const std::array<U, N>& value) {
        using target_array = std::remove_cv_t<T>;
        using target_elem = typename array_meta<target_array>::value_type;
        target_array converted{};
        for (size_t i = 0; i < N; ++i) {
            converted[i] = static_cast<target_elem>(value[i]);
        }
        return Ok(std::move(converted));
    }
    
    // Constructors for Err variant
    static Result Err(E error) requires std::is_reference_v<E> {
        Result r(UninitTag{});
        new (&r.storage.err_storage) ErrStored(to_err_storage(error));
        r.is_ok_value = false;
        return r;
    }

    template<typename U = E>
    requires (!std::is_reference_v<E> && !std::is_lvalue_reference_v<U> && std::is_constructible_v<ErrStored, U&&>)
    static Result Err(U&& error) {
        Result r(UninitTag{});
        new (&r.storage.err_storage) ErrStored(to_err_storage(std::forward<U>(error)));
        r.is_ok_value = false;
        return r;
    }

    // Lvalue Err fallback for move-only payloads that expose clone().
    template<typename U = std::remove_cvref_t<E>>
    requires (!std::is_reference_v<E>)
    static Result Err(const U& error) {
        Result r(UninitTag{});
        if constexpr (std::is_constructible_v<ErrStored, const U&>) {
            new (&r.storage.err_storage) ErrStored(error);
        } else if constexpr (requires(const U& e) { e.clone(); }
                             && std::is_constructible_v<ErrStored, decltype(error.clone())>) {
            new (&r.storage.err_storage) ErrStored(error.clone());
        } else {
            static_assert(
                std::is_constructible_v<ErrStored, const U&>,
                "Result::Err lvalue requires copy-constructible or clone()-able payload");
        }
        r.is_ok_value = false;
        return r;
    }
    
    // Default constructor (creates Err with default E)
    Result() : is_ok_value(false) {
        new (&storage.err_storage) ErrStored();
    }
    
    // Copy constructor
    Result(const Result& other) : is_ok_value(other.is_ok_value) {
        if (is_ok_value) {
            new (&storage.ok_storage) OkStored(other.ok_stored_ref());
        } else {
            new (&storage.err_storage) ErrStored(other.err_stored_ref());
        }
    }
    
    // Move constructor
    Result(Result&& other) noexcept : is_ok_value(other.is_ok_value) {
        if (is_ok_value) {
            new (&storage.ok_storage) OkStored(std::move(other.ok_stored_ref()));
        } else {
            new (&storage.err_storage) ErrStored(std::move(other.err_stored_ref()));
        }
    }
    
    // Copy assignment
    Result& operator=(const Result& other) {
        if (this != &other) {
            destroy();
            is_ok_value = other.is_ok_value;
            if (is_ok_value) {
                new (&storage.ok_storage) OkStored(other.ok_stored_ref());
            } else {
                new (&storage.err_storage) ErrStored(other.err_stored_ref());
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
                new (&storage.ok_storage) OkStored(std::move(other.ok_stored_ref()));
            } else {
                new (&storage.err_storage) ErrStored(std::move(other.err_stored_ref()));
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

    // Convert Result<T, E> into Option<T> by discarding Err.
    Option<T> ok() {
        if (is_ok_value) {
            return Option<T>(std::move(ok_ref()));
        }
        return Option<T>(None);
    }

    // Convert Result<T, E> into Option<E> by discarding Ok.
    Option<E> err() {
        if (is_ok_value) {
            return Option<E>(None);
        }
        return Option<E>(std::move(err_ref()));
    }

    // Borrow payload by pointer without moving the Result value.
    // Uses remove_reference_t to avoid forming pointer-to-reference
    // when T or E is a reference type (e.g., Result<const int&, Error>).
    using AsRefOk = const std::remove_reference_t<T>*;
    using AsRefErr = const std::remove_reference_t<E>*;
    using AsMutOk = std::remove_reference_t<T>*;
    using AsMutErr = std::remove_reference_t<E>*;

    Result<AsRefOk, AsRefErr> as_ref() const & {
        if (is_ok_value) {
            return Result<AsRefOk, AsRefErr>::Ok(&ok_ref());
        }
        return Result<AsRefOk, AsRefErr>::Err(&err_ref());
    }

    Result<AsMutOk, AsMutErr> as_mut() & {
        if (is_ok_value) {
            return Result<AsMutOk, AsMutErr>::Ok(&ok_ref());
        }
        return Result<AsMutOk, AsMutErr>::Err(&err_ref());
    }

    Result<AsRefOk, AsRefErr> as_ref() && = delete;
    Result<AsRefOk, AsRefErr> as_ref() const && = delete;
    Result<AsMutOk, AsMutErr> as_mut() && = delete;
    
    // Unwrap Ok value (panics if Err)
    T unwrap() {
        if (!is_ok_value) {
            throw std::runtime_error("Called unwrap on an Err value");
        }
        if constexpr (std::is_reference_v<T>) {
            return ok_ref();
        } else {
            return std::move(ok_ref());
        }
    }

    // Const unwrap fallback for read-only Result values.
    auto unwrap() const
        -> std::conditional_t<std::is_reference_v<T>, T, const std::remove_reference_t<T>&> {
        if (!is_ok_value) {
            throw std::runtime_error("Called unwrap on an Err value");
        }
        return ok_ref();
    }

    template<typename Msg>
    T expect(Msg&& msg) {
        if (!is_ok_value) {
            throw_expect_failure(std::forward<Msg>(msg));
        }
        if constexpr (std::is_reference_v<T>) {
            return ok_ref();
        } else {
            return std::move(ok_ref());
        }
    }

    template<typename Msg>
    auto expect(Msg&& msg) const
        -> std::conditional_t<std::is_reference_v<T>, T, const std::remove_reference_t<T>&> {
        if (!is_ok_value) {
            throw_expect_failure(std::forward<Msg>(msg));
        }
        return ok_ref();
    }
    
    // Unwrap Err value (panics if Ok)
    E unwrap_err() {
        if (is_ok_value) {
            throw std::runtime_error("Called unwrap_err on an Ok value");
        }
        if constexpr (std::is_reference_v<E>) {
            return err_ref();
        } else {
            return std::move(err_ref());
        }
    }

    // Const unwrap_err fallback for read-only Result values.
    auto unwrap_err() const
        -> std::conditional_t<std::is_reference_v<E>, E, const std::remove_reference_t<E>&> {
        if (is_ok_value) {
            throw std::runtime_error("Called unwrap_err on an Ok value");
        }
        return err_ref();
    }

    template<typename Msg>
    E expect_err(Msg&& msg) {
        if (is_ok_value) {
            throw_expect_failure(std::forward<Msg>(msg));
        }
        if constexpr (std::is_reference_v<E>) {
            return err_ref();
        } else {
            return std::move(err_ref());
        }
    }

    template<typename Msg>
    auto expect_err(Msg&& msg) const
        -> std::conditional_t<std::is_reference_v<E>, E, const std::remove_reference_t<E>&> {
        if (is_ok_value) {
            throw_expect_failure(std::forward<Msg>(msg));
        }
        return err_ref();
    }
    
    // Unwrap Ok value or return default
    T unwrap_or(T default_value) {
        if (is_ok_value) {
            return std::move(ok_ref());
        }
        return std::move(default_value);
    }

    template<typename F>
    T unwrap_or_else(F f) {
        if (is_ok_value) {
            return std::move(ok_ref());
        }
        using FReturn = decltype(f(std::move(err_ref())));
        if constexpr (std::is_void_v<FReturn>) {
            f(std::move(err_ref()));
            std::abort();
        } else {
            return f(std::move(err_ref()));
        }
    }
    
    // Map over Ok value
    template<typename F>
    auto map(F f) -> Result<decltype(f(std::declval<T>())), E> {
        using NewT = decltype(f(std::declval<T>()));
        if (is_ok_value) {
            return Result<NewT, E>::Ok(f(std::move(ok_ref())));
        } else {
            return Result<NewT, E>::Err(std::move(err_ref()));
        }
    }
    
    // Map over Err value
    template<typename F>
    auto map_err(F f) -> Result<T, decltype(f(std::declval<E>()))> {
        using NewE = decltype(f(std::declval<E>()));
        if (is_ok_value) {
            return Result<T, NewE>::Ok(std::move(ok_ref()));
        } else {
            return Result<T, NewE>::Err(f(std::move(err_ref())));
        }
    }

    // Rust parity: Result<Option<T>, E>::transpose(self) -> Option<Result<T, E>>
    template<typename Q = T>
    auto transpose() -> Option<Result<result_option_value_t<Q>, E>>
    requires result_is_option_v<Q> {
        using Inner = result_option_value_t<Q>;
        if (is_ok_value) {
            auto inner = std::move(ok_ref());
            if (inner.is_some()) {
                return Option<Result<Inner, E>>(Result<Inner, E>::Ok(inner.unwrap()));
            }
            return Option<Result<Inner, E>>(None);
        }
        return Option<Result<Inner, E>>(Result<Inner, E>::Err(std::move(err_ref())));
    }
    
    // Chain operations that return Result
    template<typename F>
    auto and_then(F f) -> decltype(f(std::declval<T>())) {
        using ReturnType = decltype(f(std::declval<T>()));
        if (is_ok_value) {
            return f(std::move(ok_ref()));
        } else {
            return ReturnType::Err(std::move(err_ref()));
        }
    }
    
    // Provide alternative Result if this is Err
    template<typename F>
    Result or_else(F f) {
        if (is_ok_value) {
            return std::move(*this);
        } else {
            return f(std::move(err_ref()));
        }
    }
    
    // Explicit bool conversion - true if Ok
    explicit operator bool() const {
        return is_ok_value;
    }

    bool operator==(const Result& other) const {
        if (is_ok_value != other.is_ok_value) {
            return false;
        }
        if (is_ok_value) {
            return ok_ref() == other.ok_ref();
        }
        return err_ref() == other.err_ref();
    }

    bool operator!=(const Result& other) const {
        return !(*this == other);
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

    template<typename Msg>
    [[noreturn]] static void throw_expect_failure(Msg&& msg) {
        if constexpr (std::is_convertible_v<Msg, std::string_view>) {
            throw std::runtime_error(std::string(std::string_view(std::forward<Msg>(msg))));
        } else {
            throw std::runtime_error("Result expectation failed");
        }
    }
    
public:
    using ok_type = void;
    using err_type = E;

    // Constructor for Ok variant
    static Result Ok() {
        Result r;
        r.is_ok_value = true;
        return r;
    }
    
    // Constructor for Err variant
    static Result Err(E error) requires std::is_reference_v<E> {
        Result r;
        new (&r.storage.err_storage) E(error);
        r.is_ok_value = false;
        return r;
    }

    template<typename U = E>
    requires (!std::is_reference_v<E> && !std::is_lvalue_reference_v<U> && std::is_constructible_v<E, U&&>)
    static Result Err(U&& error) {
        Result r;
        new (&r.storage.err_storage) E(std::forward<U>(error));
        r.is_ok_value = false;
        return r;
    }

    template<typename U = std::remove_cvref_t<E>>
    requires (!std::is_reference_v<E>)
    static Result Err(const U& error) {
        Result r;
        if constexpr (std::is_constructible_v<E, const U&>) {
            new (&r.storage.err_storage) E(error);
        } else if constexpr (requires(const U& e) { e.clone(); }
                             && std::is_constructible_v<E, decltype(error.clone())>) {
            new (&r.storage.err_storage) E(error.clone());
        } else {
            static_assert(
                std::is_constructible_v<E, const U&>,
                "Result<void, E>::Err lvalue requires copy-constructible or clone()-able payload");
        }
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

    // Convert Result<(), E> into Option<()> by discarding Err.
    Option<std::tuple<>> ok() {
        if (is_ok_value) {
            return Option<std::tuple<>>(std::tuple<>{});
        }
        return Option<std::tuple<>>(None);
    }

    // Convert Result<(), E> into Option<E> by discarding Ok.
    Option<E> err() {
        if (is_ok_value) {
            return Option<E>(None);
        }
        return Option<E>(std::move(err_ref()));
    }

    Result<void, const E*> as_ref() const & {
        if (is_ok_value) {
            return Result<void, const E*>::Ok();
        }
        return Result<void, const E*>::Err(&err_ref());
    }

    Result<void, E*> as_mut() & {
        if (is_ok_value) {
            return Result<void, E*>::Ok();
        }
        return Result<void, E*>::Err(&err_ref());
    }

    Result<void, const E*> as_ref() && = delete;
    Result<void, const E*> as_ref() const && = delete;
    Result<void, E*> as_mut() && = delete;
    
    // Unwrap Err value (panics if Ok)
    void unwrap() const {
        if (!is_ok_value) {
            throw std::runtime_error("Called unwrap on an Err value");
        }
    }

    template<typename Msg>
    void expect(Msg&& msg) const {
        if (!is_ok_value) {
            throw_expect_failure(std::forward<Msg>(msg));
        }
    }

    // Unwrap Err value (panics if Ok)
    E unwrap_err() {
        if (is_ok_value) {
            throw std::runtime_error("Called unwrap_err on an Ok value");
        }
        if constexpr (std::is_reference_v<E>) {
            return err_ref();
        } else {
            return std::move(err_ref());
        }
    }

    E unwrap_err() const {
        if (is_ok_value) {
            throw std::runtime_error("Called unwrap_err on an Ok value");
        }
        return err_ref();
    }

    template<typename Msg>
    E expect_err(Msg&& msg) {
        if (is_ok_value) {
            throw_expect_failure(std::forward<Msg>(msg));
        }
        if constexpr (std::is_reference_v<E>) {
            return err_ref();
        } else {
            return std::move(err_ref());
        }
    }

    template<typename Msg>
    auto expect_err(Msg&& msg) const
        -> std::conditional_t<std::is_reference_v<E>, E, const std::remove_reference_t<E>&> {
        if (is_ok_value) {
            throw_expect_failure(std::forward<Msg>(msg));
        }
        return err_ref();
    }
    
    // Explicit bool conversion - true if Ok
    explicit operator bool() const {
        return is_ok_value;
    }

    bool operator==(const Result& other) const {
        if (is_ok_value != other.is_ok_value) {
            return false;
        }
        if (is_ok_value) {
            return true;
        }
        return err_ref() == other.err_ref();
    }

    bool operator!=(const Result& other) const {
        return !(*this == other);
    }
};

template<typename R>
struct result_ok_type;

template<typename R>
struct result_err_type;

template<typename T, typename E>
struct result_ok_type<Result<T, E>> {
    using type = T;
};

template<typename T, typename E>
struct result_err_type<Result<T, E>> {
    using type = E;
};

template<typename R>
using result_ok_t = typename result_ok_type<std::remove_cvref_t<R>>::type;

template<typename R>
using result_err_t = typename result_err_type<std::remove_cvref_t<R>>::type;

// Helper function to create Ok Result
template<typename T, typename E, typename U>
Result<T, E> Ok(U&& value) {
    return Result<T, E>::Ok(std::forward<U>(value));
}

// Helper function to create Err Result
template<typename T, typename E, typename U>
Result<T, E> Err(U&& error) {
    return Result<T, E>::Err(std::forward<U>(error));
}

} // namespace rusty

#endif // RUSTY_RESULT_HPP
