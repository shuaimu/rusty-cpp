// Demonstration of Explicit Opt-In Send System
// Conservative approach: Types are NOT Send unless explicitly marked

#include "rusty/sync/mpsc.hpp"
#include "rusty/box.hpp"
#include "rusty/arc.hpp"
#include "rusty/rc.hpp"
#include <iostream>

// Example 1: User type that explicitly marks itself as Send
class ThreadSafeQueue {
public:
    static constexpr bool is_send = true;  // Explicit opt-in
    int value = 0;
};

// Example 2: User type that does NOT mark itself (default NOT Send)
class RegularType {
public:
    int value = 0;
};

// Example 3: User type that explicitly marks itself as NOT Send
class ThreadUnsafe {
public:
    static constexpr bool is_send = false;  // Explicit opt-out
    int value = 0;
};

// Example 4: Composite type containing Rc (automatically NOT Send)
struct ContainsRc {
    rusty::Rc<int> data;
};

void test_send_checks() {
    std::cout << "=== Explicit Opt-In Send System ===\n\n";

    // ✅ Primitives are Send (pre-marked)
    std::cout << "1. Primitives (pre-marked as Send):\n";
    std::cout << "   int: " << (rusty::is_send<int>::value ? "Send ✓" : "!Send ✗") << "\n";
    std::cout << "   double: " << (rusty::is_send<double>::value ? "Send ✓" : "!Send ✗") << "\n";
    std::cout << "   int*: " << (rusty::is_send<int*>::value ? "Send ✓" : "!Send ✗") << "\n\n";

    // ✅ Rusty types are Send (if their content is Send)
    std::cout << "2. Rusty types (pre-marked as Send if T is Send):\n";
    std::cout << "   Box<int>: " << (rusty::is_send<rusty::Box<int>>::value ? "Send ✓" : "!Send ✗") << "\n";
    std::cout << "   Arc<int>: " << (rusty::is_send<rusty::Arc<int>>::value ? "Send ✓" : "!Send ✗") << "\n";
    std::cout << "   Rc<int>: " << (rusty::is_send<rusty::Rc<int>>::value ? "Send ✓" : "!Send ✗") << " (NOT Send!)\n\n";

    // ✅ User type with static marker
    std::cout << "3. User types with static marker:\n";
    std::cout << "   ThreadSafeQueue (is_send=true): "
              << (rusty::is_send<ThreadSafeQueue>::value ? "Send ✓" : "!Send ✗") << "\n";
    std::cout << "   ThreadUnsafe (is_send=false): "
              << (rusty::is_send<ThreadUnsafe>::value ? "Send ✓" : "!Send ✗") << "\n\n";

    // ❌ Regular types default to NOT Send
    std::cout << "4. Unmarked types (DEFAULT NOT Send):\n";
    std::cout << "   RegularType (no marker): "
              << (rusty::is_send<RegularType>::value ? "Send ✓" : "!Send ✗") << "\n";
    std::cout << "   ContainsRc (contains Rc): "
              << (rusty::is_send<ContainsRc>::value ? "Send ✓" : "!Send ✗") << "\n\n";

    std::cout << "=== Channel Usage Examples ===\n\n";

    // ✅ Works: int is Send
    std::cout << "Creating channel<int>... ";
    auto [tx1, rx1] = rusty::sync::mpsc::channel<int>();
    std::cout << "✓ Success\n";

    // ✅ Works: Box<int> is Send (because int is Send)
    std::cout << "Creating channel<Box<int>>... ";
    auto [tx2, rx2] = rusty::sync::mpsc::channel<rusty::Box<int>>();
    std::cout << "✓ Success\n";

    // ✅ Works: Arc<int> is Send
    std::cout << "Creating channel<Arc<int>>... ";
    auto [tx3, rx3] = rusty::sync::mpsc::channel<rusty::Arc<int>>();
    std::cout << "✓ Success\n";

    // ✅ Works: ThreadSafeQueue explicitly marked as Send
    std::cout << "Creating channel<ThreadSafeQueue>... ";
    auto [tx4, rx4] = rusty::sync::mpsc::channel<ThreadSafeQueue>();
    std::cout << "✓ Success\n\n";

    // ❌ Compile errors (uncomment to test):
    std::cout << "=== Compile-Time Rejections ===\n\n";

    std::cout << "These would cause compile errors:\n\n";

    std::cout << "1. auto [tx, rx] = channel<Rc<int>>();\n";
    std::cout << "   Error: Rc<int> is NOT Send (non-atomic ref counting)\n\n";

    std::cout << "2. auto [tx, rx] = channel<RegularType>();\n";
    std::cout << "   Error: RegularType not marked as Send (no static marker)\n\n";

    std::cout << "3. auto [tx, rx] = channel<ContainsRc>();\n";
    std::cout << "   Error: ContainsRc not marked as Send\n";
    std::cout << "   (Even though it contains Rc, the REAL reason is lack of marker)\n\n";

    std::cout << "4. auto [tx, rx] = channel<ThreadUnsafe>();\n";
    std::cout << "   Error: ThreadUnsafe explicitly marked as !Send\n\n";

    std::cout << "=== Key Advantage: Compositional Safety ===\n\n";

    std::cout << "Unlike the old approach, struct { Rc<int> } is now\n";
    std::cout << "automatically NOT Send (because it lacks the marker).\n\n";

    std::cout << "Old approach (movable default):\n";
    std::cout << "  ✗ ContainsRc would be Send (has move constructor)\n";
    std::cout << "  ✗ Unsafe! Could send non-thread-safe Rc\n\n";

    std::cout << "New approach (explicit opt-in):\n";
    std::cout << "  ✓ ContainsRc is NOT Send (no marker)\n";
    std::cout << "  ✓ Safe! Must explicitly mark as Send\n";
    std::cout << "  ✓ Forces developer to think about thread-safety\n";
}

int main() {
    test_send_checks();

    std::cout << "\n=== Summary ===\n\n";
    std::cout << "Conservative approach: Default is NOT Send\n";
    std::cout << "  ✓ Catches Rc<T> automatically\n";
    std::cout << "  ✓ Catches struct { Rc<T> } automatically\n";
    std::cout << "  ✓ Solves compositional problem\n";
    std::cout << "  ✓ Forces explicit thinking about thread-safety\n";
    std::cout << "  ✗ Requires marking primitives (done for you)\n";
    std::cout << "  ✗ Requires marking user types (simple: add is_send)\n";

    return 0;
}
