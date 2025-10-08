#include <rusty/traits.hpp>
#include <rusty/arc.hpp>
#include <rusty/rc.hpp>
#include <rusty/box.hpp>
#include <rusty/cell.hpp>
#include <rusty/refcell.hpp>
#include <rusty/mutex.hpp>
#include <iostream>

using namespace rusty;

// ============================================================================
// Test Rust Rule 1: &T is Send if T is Sync
// ============================================================================

// Arc<int> is Sync, so const Arc<int>& is Send
static_assert(is_sync<Arc<int>>::value, "Arc<int> should be Sync");
static_assert(is_send<const Arc<int>&>::value, "const Arc<int>& should be Send (Rule 1)");

// Mutex<int> is Sync, so const Mutex<int>& is Send
static_assert(is_sync<Mutex<int>>::value, "Mutex<int> should be Sync");
static_assert(is_send<const Mutex<int>&>::value, "const Mutex<int>& should be Send (Rule 1)");

// Cell<int> is NOT Sync, so const Cell<int>& is NOT Send
static_assert(!is_sync<Cell<int>>::value, "Cell<int> should NOT be Sync");
static_assert(!is_send<const Cell<int>&>::value, "const Cell<int>& should NOT be Send (Rule 1)");

// RefCell<int> is NOT Sync, so const RefCell<int>& is NOT Send
static_assert(!is_sync<RefCell<int>>::value, "RefCell<int> should NOT be Sync");
static_assert(!is_send<const RefCell<int>&>::value, "const RefCell<int>& should NOT be Send (Rule 1)");

// Rc<int> is NOT Sync, so const Rc<int>& is NOT Send
static_assert(!is_sync<Rc<int>>::value, "Rc<int> should NOT be Sync");
static_assert(!is_send<const Rc<int>&>::value, "const Rc<int>& should NOT be Send (Rule 1)");

// Primitives are Sync, so const int& is Send
static_assert(is_sync<int>::value, "int should be Sync");
static_assert(is_send<const int&>::value, "const int& should be Send (Rule 1)");

// ============================================================================
// Test Rust Rule 2: &mut T is Send if T is Send
// ============================================================================

// int is Send, so int& is Send
static_assert(is_send<int>::value, "int should be Send");
static_assert(is_send<int&>::value, "int& should be Send (Rule 2)");

// Arc<int> is Send, so Arc<int>& is Send
static_assert(is_send<Arc<int>>::value, "Arc<int> should be Send");
static_assert(is_send<Arc<int>&>::value, "Arc<int>& should be Send (Rule 2)");

// Rc<int> is NOT Send, so Rc<int>& is NOT Send
static_assert(!is_send<Rc<int>>::value, "Rc<int> should NOT be Send");
static_assert(!is_send<Rc<int>&>::value, "Rc<int>& should NOT be Send (Rule 2)");

// Box<int> is Send, so Box<int>& is Send
static_assert(is_send<Box<int>>::value, "Box<int> should be Send");
static_assert(is_send<Box<int>&>::value, "Box<int>& should be Send (Rule 2)");

// ============================================================================
// Test Rust Rule 3: &T is Sync if T is Sync
// ============================================================================

// Arc<int> is Sync, so const Arc<int>& is Sync
static_assert(is_sync<Arc<int>>::value, "Arc<int> should be Sync");
static_assert(is_sync<const Arc<int>&>::value, "const Arc<int>& should be Sync (Rule 3)");

// int is Sync, so const int& is Sync
static_assert(is_sync<int>::value, "int should be Sync");
static_assert(is_sync<const int&>::value, "const int& should be Sync (Rule 3)");

// Cell<int> is NOT Sync, so const Cell<int>& is NOT Sync
static_assert(!is_sync<Cell<int>>::value, "Cell<int> should NOT be Sync");
static_assert(!is_sync<const Cell<int>&>::value, "const Cell<int>& should NOT be Sync (Rule 3)");

// ============================================================================
// Test Rust Rule 4: &mut T is never Sync
// ============================================================================

// int& is never Sync (mutable references)
static_assert(!is_sync<int&>::value, "int& should NOT be Sync (Rule 4)");

// Arc<int>& is never Sync
static_assert(!is_sync<Arc<int>&>::value, "Arc<int>& should NOT be Sync (Rule 4)");

// Mutex<int>& is never Sync
static_assert(!is_sync<Mutex<int>&>::value, "Mutex<int>& should NOT be Sync (Rule 4)");

// Even if T is Sync, &mut T is not Sync
static_assert(is_sync<Arc<int>>::value, "Arc<int> should be Sync");
static_assert(!is_sync<Arc<int>&>::value, "Arc<int>& should NOT be Sync (Rule 4)");

// ============================================================================
// Test specific type traits
// ============================================================================

// Primitives
static_assert(is_send<int>::value && is_sync<int>::value, "int should be Send + Sync");
static_assert(is_send<float>::value && is_sync<float>::value, "float should be Send + Sync");
static_assert(is_send<bool>::value && is_sync<bool>::value, "bool should be Send + Sync");

// Arc<T>
static_assert(is_send<Arc<int>>::value, "Arc<int> should be Send");
static_assert(is_sync<Arc<int>>::value, "Arc<int> should be Sync");

// Rc<T>
static_assert(!is_send<Rc<int>>::value, "Rc<int> should NOT be Send");
static_assert(!is_sync<Rc<int>>::value, "Rc<int> should NOT be Sync");

// Box<T>
static_assert(is_send<Box<int>>::value, "Box<int> should be Send");
static_assert(is_sync<Box<int>>::value, "Box<int> should be Sync");

// unique_ptr
static_assert(is_send<std::unique_ptr<int>>::value, "unique_ptr<int> should be Send");
static_assert(is_sync<std::unique_ptr<int>>::value, "unique_ptr<int> should be Sync");

// Mutex<T> (Send if T is Send, Sync if T is Send)
static_assert(is_send<Mutex<int>>::value, "Mutex<int> should be Send");
static_assert(is_sync<Mutex<int>>::value, "Mutex<int> should be Sync");

// Cell<T> (Send if T is Send, but never Sync)
static_assert(is_send<Cell<int>>::value, "Cell<int> should be Send");
static_assert(!is_sync<Cell<int>>::value, "Cell<int> should NOT be Sync");

// RefCell<T> (Send if T is Send, but never Sync)
static_assert(is_send<RefCell<int>>::value, "RefCell<int> should be Send");
static_assert(!is_sync<RefCell<int>>::value, "RefCell<int> should NOT be Sync");

// Raw pointers are not Send or Sync
static_assert(!is_send<int*>::value, "int* should NOT be Send");
static_assert(!is_sync<int*>::value, "int* should NOT be Sync");
static_assert(!is_send<const int*>::value, "const int* should NOT be Send");
static_assert(!is_sync<const int*>::value, "const int* should NOT be Sync");

// ============================================================================
// Test concepts (C++20)
// ============================================================================

static_assert(Send<int>, "int satisfies Send concept");
static_assert(Sync<int>, "int satisfies Sync concept");
static_assert(ThreadSafe<int>, "int satisfies ThreadSafe concept");

static_assert(Send<Arc<int>>, "Arc<int> satisfies Send concept");
static_assert(Sync<Arc<int>>, "Arc<int> satisfies Sync concept");
static_assert(ThreadSafe<Arc<int>>, "Arc<int> satisfies ThreadSafe concept");

static_assert(!Send<Rc<int>>, "Rc<int> does NOT satisfy Send concept");
static_assert(!Sync<Rc<int>>, "Rc<int> does NOT satisfy Sync concept");
static_assert(!ThreadSafe<Rc<int>>, "Rc<int> does NOT satisfy ThreadSafe concept");

static_assert(Send<Cell<int>>, "Cell<int> satisfies Send concept");
static_assert(!Sync<Cell<int>>, "Cell<int> does NOT satisfy Sync concept");
static_assert(!ThreadSafe<Cell<int>>, "Cell<int> does NOT satisfy ThreadSafe concept");

int main() {
    std::cout << "All trait tests passed!\n";

    // Runtime verification (just to be thorough)
    std::cout << "Checking Send/Sync traits at runtime:\n";
    std::cout << "  Arc<int> is Send: " << is_send<Arc<int>>::value << "\n";
    std::cout << "  Arc<int> is Sync: " << is_sync<Arc<int>>::value << "\n";
    std::cout << "  Rc<int> is Send: " << is_send<Rc<int>>::value << "\n";
    std::cout << "  Rc<int> is Sync: " << is_sync<Rc<int>>::value << "\n";
    std::cout << "  const Arc<int>& is Send: " << is_send<const Arc<int>&>::value << "\n";
    std::cout << "  const Cell<int>& is Send: " << is_send<const Cell<int>&>::value << "\n";
    std::cout << "  int& is Sync: " << is_sync<int&>::value << "\n";

    return 0;
}
