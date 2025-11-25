#ifndef RUSTY_FUNCTION_HPP
#define RUSTY_FUNCTION_HPP

#include <cstddef>
#include <cstring>
#include <memory>
#include <new>
#include <type_traits>
#include <utility>

// rusty::Function<Sig> - Move-only type-erased callable wrapper
//
// Similar to std::move_only_function (C++23) but available in C++20.
// Unlike std::function, this can store move-only callables (lambdas
// capturing unique_ptr, Box, Arc, etc.)
//
// Features:
// - Move-only (no copy)
// - Small buffer optimization (SBO) for small callables
// - Type-erased storage via vtable
// - Supports const-qualified function signatures
//
// Example:
//   auto ptr = rusty::Box<int>::make(42);
//   rusty::Function<int()> fn = [p = std::move(ptr)]() { return *p; };
//   int value = fn();

namespace rusty {

// Forward declaration
template<typename Sig>
class Function;

namespace detail {

// Small buffer size: 3 pointers (24 bytes on 64-bit)
// This fits most lambdas with 1-2 captured values
constexpr std::size_t kFunctionSBOSize = 3 * sizeof(void*);
constexpr std::size_t kFunctionSBOAlign = alignof(std::max_align_t);

// Aligned storage for SBO
using FunctionStorage = std::aligned_storage_t<kFunctionSBOSize, kFunctionSBOAlign>;

// Check if a type fits in SBO
template<typename T>
constexpr bool fits_in_sbo_v =
    sizeof(T) <= kFunctionSBOSize &&
    alignof(T) <= kFunctionSBOAlign &&
    std::is_nothrow_move_constructible_v<T>;

// Vtable for type-erased operations
template<typename R, typename... Args>
struct FunctionVTable {
    R (*invoke)(void* storage, Args...);
    void (*move)(void* dst, void* src);
    void (*destroy)(void* storage);
    bool is_inline;  // True if stored inline (SBO), false if on heap
};

// Vtable implementation for inline (SBO) storage
template<typename Callable, typename R, typename... Args>
struct InlineVTableImpl {
    static R invoke(void* storage, Args... args) {
        Callable& callable = *static_cast<Callable*>(storage);
        return callable(std::forward<Args>(args)...);
    }

    static void move(void* dst, void* src) {
        new (dst) Callable(std::move(*static_cast<Callable*>(src)));
        static_cast<Callable*>(src)->~Callable();
    }

    static void destroy(void* storage) {
        static_cast<Callable*>(storage)->~Callable();
    }

    static constexpr FunctionVTable<R, Args...> vtable = {
        &invoke, &move, &destroy, true
    };
};

template<typename Callable, typename R, typename... Args>
constexpr FunctionVTable<R, Args...> InlineVTableImpl<Callable, R, Args...>::vtable;

// Vtable implementation for heap storage
template<typename Callable, typename R, typename... Args>
struct HeapVTableImpl {
    static R invoke(void* storage, Args... args) {
        Callable* ptr = *static_cast<Callable**>(storage);
        return (*ptr)(std::forward<Args>(args)...);
    }

    static void move(void* dst, void* src) {
        // Just move the pointer
        Callable** dst_ptr = static_cast<Callable**>(dst);
        Callable** src_ptr = static_cast<Callable**>(src);
        *dst_ptr = *src_ptr;
        *src_ptr = nullptr;
    }

    static void destroy(void* storage) {
        Callable* ptr = *static_cast<Callable**>(storage);
        delete ptr;
    }

    static constexpr FunctionVTable<R, Args...> vtable = {
        &invoke, &move, &destroy, false
    };
};

template<typename Callable, typename R, typename... Args>
constexpr FunctionVTable<R, Args...> HeapVTableImpl<Callable, R, Args...>::vtable;

// Get the appropriate vtable for a callable type
template<typename Callable, typename R, typename... Args>
constexpr const FunctionVTable<R, Args...>* get_vtable() {
    if constexpr (fits_in_sbo_v<Callable>) {
        return &InlineVTableImpl<Callable, R, Args...>::vtable;
    } else {
        return &HeapVTableImpl<Callable, R, Args...>::vtable;
    }
}

// Helper to extract signature components
template<typename Sig>
struct FunctionTraits;

// Non-const signature: R(Args...)
template<typename R, typename... Args>
struct FunctionTraits<R(Args...)> {
    using ReturnType = R;
    using VTableType = FunctionVTable<R, Args...>;
    static constexpr bool is_const = false;
};

// Const signature: R(Args...) const
template<typename R, typename... Args>
struct FunctionTraits<R(Args...) const> {
    using ReturnType = R;
    using VTableType = FunctionVTable<R, Args...>;
    static constexpr bool is_const = true;
};

} // namespace detail

// Main Function class template
// @safe - Move-only type-erased callable wrapper
template<typename R, typename... Args>
class Function<R(Args...)> {
public:
    using result_type = R;

private:
    using VTable = detail::FunctionVTable<R, Args...>;

    detail::FunctionStorage storage_;
    const VTable* vtable_ = nullptr;

    // Store callable inline (SBO)
    template<typename Callable>
    void store_inline(Callable&& callable) {
        using DecayedCallable = std::decay_t<Callable>;
        static_assert(detail::fits_in_sbo_v<DecayedCallable>,
                      "Callable does not fit in SBO");
        new (&storage_) DecayedCallable(std::forward<Callable>(callable));
        vtable_ = detail::get_vtable<DecayedCallable, R, Args...>();
    }

    // Store callable on heap
    template<typename Callable>
    void store_heap(Callable&& callable) {
        using DecayedCallable = std::decay_t<Callable>;
        DecayedCallable* ptr = new DecayedCallable(std::forward<Callable>(callable));
        *reinterpret_cast<DecayedCallable**>(&storage_) = ptr;
        vtable_ = detail::get_vtable<DecayedCallable, R, Args...>();
    }

public:
    // @safe - Default constructor - creates empty Function
    Function() noexcept = default;

    // @safe - Nullptr constructor - creates empty Function
    Function(std::nullptr_t) noexcept : Function() {}

    // @safe - Construct from callable
    template<typename Callable,
             typename = std::enable_if_t<
                 !std::is_same_v<std::decay_t<Callable>, Function> &&
                 std::is_invocable_r_v<R, Callable, Args...>
             >>
    Function(Callable&& callable) {
        using DecayedCallable = std::decay_t<Callable>;

        if constexpr (detail::fits_in_sbo_v<DecayedCallable>) {
            store_inline(std::forward<Callable>(callable));
        } else {
            store_heap(std::forward<Callable>(callable));
        }
    }

    // @safe - Move constructor
    Function(Function&& other) noexcept {
        if (other.vtable_) {
            other.vtable_->move(&storage_, &other.storage_);
            vtable_ = other.vtable_;
            other.vtable_ = nullptr;
        }
    }

    // No copy constructor
    Function(const Function&) = delete;

    // @safe - Destructor
    ~Function() {
        if (vtable_) {
            vtable_->destroy(&storage_);
        }
    }

    // @safe - Move assignment
    Function& operator=(Function&& other) noexcept {
        if (this != &other) {
            // Destroy current callable
            if (vtable_) {
                vtable_->destroy(&storage_);
                vtable_ = nullptr;
            }

            // Move from other
            if (other.vtable_) {
                other.vtable_->move(&storage_, &other.storage_);
                vtable_ = other.vtable_;
                other.vtable_ = nullptr;
            }
        }
        return *this;
    }

    // No copy assignment
    Function& operator=(const Function&) = delete;

    // @safe - Assign nullptr (clear)
    Function& operator=(std::nullptr_t) noexcept {
        if (vtable_) {
            vtable_->destroy(&storage_);
            vtable_ = nullptr;
        }
        return *this;
    }

    // @safe - Assign callable
    template<typename Callable,
             typename = std::enable_if_t<
                 !std::is_same_v<std::decay_t<Callable>, Function> &&
                 std::is_invocable_r_v<R, Callable, Args...>
             >>
    Function& operator=(Callable&& callable) {
        Function tmp(std::forward<Callable>(callable));
        *this = std::move(tmp);
        return *this;
    }

    // @safe - Invoke the stored callable
    R operator()(Args... args) {
        if (!vtable_) {
            // Undefined behavior to call empty Function, but we'll assert
            std::abort();
        }
        return vtable_->invoke(&storage_, std::forward<Args>(args)...);
    }

    // @safe - Check if Function contains a callable
    explicit operator bool() const noexcept {
        return vtable_ != nullptr;
    }

    // @safe - Check if Function is empty
    bool is_empty() const noexcept {
        return vtable_ == nullptr;
    }

    // @safe - Swap with another Function
    void swap(Function& other) noexcept {
        Function tmp(std::move(other));
        other = std::move(*this);
        *this = std::move(tmp);
    }

    // @safe - Check if using inline storage (SBO)
    bool is_inline() const noexcept {
        return vtable_ && vtable_->is_inline;
    }
};

// Const-qualified function signature specialization
// @safe - Move-only type-erased callable wrapper (const version)
template<typename R, typename... Args>
class Function<R(Args...) const> {
public:
    using result_type = R;

private:
    using VTable = detail::FunctionVTable<R, Args...>;

    detail::FunctionStorage storage_;
    const VTable* vtable_ = nullptr;

    template<typename Callable>
    void store_inline(Callable&& callable) {
        using DecayedCallable = std::decay_t<Callable>;
        static_assert(detail::fits_in_sbo_v<DecayedCallable>,
                      "Callable does not fit in SBO");
        new (&storage_) DecayedCallable(std::forward<Callable>(callable));
        vtable_ = detail::get_vtable<DecayedCallable, R, Args...>();
    }

    template<typename Callable>
    void store_heap(Callable&& callable) {
        using DecayedCallable = std::decay_t<Callable>;
        DecayedCallable* ptr = new DecayedCallable(std::forward<Callable>(callable));
        *reinterpret_cast<DecayedCallable**>(&storage_) = ptr;
        vtable_ = detail::get_vtable<DecayedCallable, R, Args...>();
    }

public:
    // @safe - Default constructor
    Function() noexcept = default;
    // @safe - Nullptr constructor
    Function(std::nullptr_t) noexcept : Function() {}

    // @safe - Construct from callable
    template<typename Callable,
             typename = std::enable_if_t<
                 !std::is_same_v<std::decay_t<Callable>, Function> &&
                 std::is_invocable_r_v<R, const std::decay_t<Callable>&, Args...>
             >>
    Function(Callable&& callable) {
        using DecayedCallable = std::decay_t<Callable>;

        if constexpr (detail::fits_in_sbo_v<DecayedCallable>) {
            store_inline(std::forward<Callable>(callable));
        } else {
            store_heap(std::forward<Callable>(callable));
        }
    }

    // @safe - Move constructor
    Function(Function&& other) noexcept {
        if (other.vtable_) {
            other.vtable_->move(&storage_, &other.storage_);
            vtable_ = other.vtable_;
            other.vtable_ = nullptr;
        }
    }

    Function(const Function&) = delete;

    // @safe - Destructor
    ~Function() {
        if (vtable_) {
            vtable_->destroy(&storage_);
        }
    }

    // @safe - Move assignment
    Function& operator=(Function&& other) noexcept {
        if (this != &other) {
            if (vtable_) {
                vtable_->destroy(&storage_);
                vtable_ = nullptr;
            }
            if (other.vtable_) {
                other.vtable_->move(&storage_, &other.storage_);
                vtable_ = other.vtable_;
                other.vtable_ = nullptr;
            }
        }
        return *this;
    }

    Function& operator=(const Function&) = delete;

    // @safe - Assign nullptr
    Function& operator=(std::nullptr_t) noexcept {
        if (vtable_) {
            vtable_->destroy(&storage_);
            vtable_ = nullptr;
        }
        return *this;
    }

    // @safe - Assign callable
    template<typename Callable,
             typename = std::enable_if_t<
                 !std::is_same_v<std::decay_t<Callable>, Function> &&
                 std::is_invocable_r_v<R, const std::decay_t<Callable>&, Args...>
             >>
    Function& operator=(Callable&& callable) {
        Function tmp(std::forward<Callable>(callable));
        *this = std::move(tmp);
        return *this;
    }

    // @safe - Const invoke - callable must be const-invocable
    R operator()(Args... args) const {
        if (!vtable_) {
            std::abort();
        }
        return vtable_->invoke(const_cast<void*>(static_cast<const void*>(&storage_)),
                               std::forward<Args>(args)...);
    }

    // @safe - Check if Function contains a callable
    explicit operator bool() const noexcept {
        return vtable_ != nullptr;
    }

    // @safe - Check if Function is empty
    bool is_empty() const noexcept {
        return vtable_ == nullptr;
    }

    // @safe - Swap with another Function
    void swap(Function& other) noexcept {
        Function tmp(std::move(other));
        other = std::move(*this);
        *this = std::move(tmp);
    }

    // @safe - Check if using inline storage (SBO)
    bool is_inline() const noexcept {
        return vtable_ && vtable_->is_inline;
    }
};

// Non-member swap
template<typename Sig>
void swap(Function<Sig>& lhs, Function<Sig>& rhs) noexcept {
    lhs.swap(rhs);
}

// Comparison with nullptr
template<typename Sig>
bool operator==(const Function<Sig>& f, std::nullptr_t) noexcept {
    return f.is_empty();
}

template<typename Sig>
bool operator==(std::nullptr_t, const Function<Sig>& f) noexcept {
    return f.is_empty();
}

template<typename Sig>
bool operator!=(const Function<Sig>& f, std::nullptr_t) noexcept {
    return !f.is_empty();
}

template<typename Sig>
bool operator!=(std::nullptr_t, const Function<Sig>& f) noexcept {
    return !f.is_empty();
}

} // namespace rusty

#endif // RUSTY_FUNCTION_HPP
