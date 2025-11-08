// Demonstration of Hybrid Send Trait System
// Combines explicit opt-out, static markers, and default heuristics

#include <type_traits>
#include <iostream>
#include <atomic>

namespace rusty {

// ==================================================================
// HYBRID SEND TRAIT SYSTEM
// ==================================================================

// Step 1: Explicit opt-out mechanism
template<typename T>
struct is_explicitly_not_send : std::false_type {};

// Step 2: Check for static marker
template<typename T, typename = void>
struct has_send_marker : std::false_type {};

template<typename T>
struct has_send_marker<T, std::void_t<decltype(T::is_send)>> : std::true_type {};

// Step 3: Main is_send trait with priority system
template<typename T>
struct is_send {
    static constexpr bool value = []() {
        // Priority 1: Check explicit opt-out specialization
        if constexpr (is_explicitly_not_send<T>::value) {
            return false;
        }
        // Priority 2: Check for static marker
        else if constexpr (has_send_marker<T>::value) {
            return T::is_send;
        }
        // Priority 3: Default heuristic (move-constructible)
        else {
            return std::is_move_constructible_v<T> &&
                   std::is_move_assignable_v<T> &&
                   std::is_destructible_v<T>;
        }
    }();
};

// ==================================================================
// EXAMPLE TYPES
// ==================================================================

// Type 1: Uses explicit opt-out specialization
template<typename T>
class Rc {
    T* ptr_;
    size_t* ref_count_;  // Non-atomic!
public:
    Rc(T value) : ptr_(new T(value)), ref_count_(new size_t(1)) {}
    Rc(Rc&& other) noexcept : ptr_(other.ptr_), ref_count_(other.ref_count_) {
        other.ptr_ = nullptr;
        other.ref_count_ = nullptr;
    }
    ~Rc() { /* cleanup */ }
};

// Rc opts-out via specialization
template<typename T>
struct is_explicitly_not_send<Rc<T>> : std::true_type {};

// Type 2: Uses static marker
template<typename T>
class Arc {
    T* ptr_;
    // std::atomic<size_t>* ref_count_;  // Atomic! (omitted for demo)
public:
    static constexpr bool is_send = true;  // Static marker

    Arc(T value) : ptr_(new T(value)) {}
    Arc(Arc&& other) noexcept = default;
    ~Arc() { delete ptr_; }
};

// Type 3: User type with static marker (opt-out)
class ThreadUnsafeCache {
public:
    static constexpr bool is_send = false;  // Not thread-safe!
    ThreadUnsafeCache() = default;
    ThreadUnsafeCache(ThreadUnsafeCache&&) = default;
    ThreadUnsafeCache& operator=(ThreadUnsafeCache&&) = default;
};

// Type 4: User type with static marker (opt-in)
class ThreadSafeQueue {
public:
    static constexpr bool is_send = true;  // Thread-safe!
    ThreadSafeQueue() = default;
    ThreadSafeQueue(ThreadSafeQueue&&) = default;
    ThreadSafeQueue& operator=(ThreadSafeQueue&&) = default;
};

// Type 5: Third-party type (no marker, uses default)
class ThirdPartyType {
public:
    ThirdPartyType() = default;
    ThirdPartyType(ThirdPartyType&&) = default;
    ThirdPartyType& operator=(ThirdPartyType&&) = default;
};

// Type 6: Third-party unsafe type - specialize externally
class ThirdPartyUnsafe {
public:
    ThirdPartyUnsafe() = default;
    ThirdPartyUnsafe(ThirdPartyUnsafe&&) = default;
    ThirdPartyUnsafe& operator=(ThirdPartyUnsafe&&) = default;
};

// External specialization for third-party type
template<>
struct is_explicitly_not_send<ThirdPartyUnsafe> : std::true_type {};

// Type 7: Not movable
class NotMovable {
    NotMovable(NotMovable&&) = delete;
};

} // namespace rusty

// ==================================================================
// DEMONSTRATION
// ==================================================================

template<typename T>
void check_send(const char* name) {
    std::cout << name << ": "
              << (rusty::is_send<T>::value ? "Send ✓" : "!Send ✗")
              << "\n";
}

int main() {
    std::cout << "=== Hybrid Send Trait System Demo ===\n\n";

    std::cout << "Priority 1: Explicit opt-out specialization\n";
    check_send<rusty::Rc<int>>("Rc<int>");
    check_send<rusty::ThirdPartyUnsafe>("ThirdPartyUnsafe");

    std::cout << "\nPriority 2: Static marker\n";
    check_send<rusty::Arc<int>>("Arc<int> (marker: true)");
    check_send<rusty::ThreadUnsafeCache>("ThreadUnsafeCache (marker: false)");
    check_send<rusty::ThreadSafeQueue>("ThreadSafeQueue (marker: true)");

    std::cout << "\nPriority 3: Default heuristic (move-constructible)\n";
    check_send<rusty::ThirdPartyType>("ThirdPartyType (no marker, movable)");
    check_send<int>("int (primitive, movable)");
    check_send<int*>("int* (pointer, movable)");

    std::cout << "\nNot movable (caught by all approaches)\n";
    check_send<rusty::NotMovable>("NotMovable");

    std::cout << "\n=== How Each Type Was Checked ===\n\n";

    std::cout << "Rc<int>:\n";
    std::cout << "  1. is_explicitly_not_send? YES → !Send ✗\n";
    std::cout << "  (Specialization: is_explicitly_not_send<Rc<T>>)\n\n";

    std::cout << "Arc<int>:\n";
    std::cout << "  1. is_explicitly_not_send? NO\n";
    std::cout << "  2. has static marker? YES → Send ✓\n";
    std::cout << "  (Static: Arc::is_send = true)\n\n";

    std::cout << "ThreadUnsafeCache:\n";
    std::cout << "  1. is_explicitly_not_send? NO\n";
    std::cout << "  2. has static marker? YES → !Send ✗\n";
    std::cout << "  (Static: ThreadUnsafeCache::is_send = false)\n\n";

    std::cout << "ThirdPartyType:\n";
    std::cout << "  1. is_explicitly_not_send? NO\n";
    std::cout << "  2. has static marker? NO\n";
    std::cout << "  3. is_move_constructible? YES → Send ✓\n";
    std::cout << "  (Default heuristic)\n\n";

    std::cout << "ThirdPartyUnsafe:\n";
    std::cout << "  1. is_explicitly_not_send? YES → !Send ✗\n";
    std::cout << "  (External specialization)\n\n";

    std::cout << "=== Three Ways to Mark !Send ===\n\n";

    std::cout << "1. Specialization (for your types):\n";
    std::cout << "   template<typename T>\n";
    std::cout << "   struct is_explicitly_not_send<Rc<T>> : std::true_type {};\n\n";

    std::cout << "2. Static marker (for your types):\n";
    std::cout << "   class MyType {\n";
    std::cout << "       static constexpr bool is_send = false;\n";
    std::cout << "   };\n\n";

    std::cout << "3. External specialization (for third-party types):\n";
    std::cout << "   template<>\n";
    std::cout << "   struct is_explicitly_not_send<ThirdPartyUnsafe> : std::true_type {};\n\n";

    std::cout << "=== Advantages ===\n\n";
    std::cout << "✓ Multiple ways to opt-out (flexibility)\n";
    std::cout << "✓ Works with our types (Rc via specialization)\n";
    std::cout << "✓ Works with user types (static marker)\n";
    std::cout << "✓ Works with third-party types (external specialization)\n";
    std::cout << "✓ Reasonable defaults (move-constructible)\n";
    std::cout << "✓ No intrusive inheritance required\n\n";

    std::cout << "=== Limitations ===\n\n";
    std::cout << "✗ Still requires manual marking\n";
    std::cout << "✗ Doesn't auto-detect like Rust\n";
    std::cout << "✗ New third-party types default to Send\n";
    std::cout << "  (Must specialize if they're actually !Send)\n";

    return 0;
}
