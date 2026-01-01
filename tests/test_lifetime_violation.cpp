// Test file to verify if RustyCpp can detect lifetime violations
// This file contains intentional bugs that a borrow checker should catch

#include "rusty/arc.hpp"
#include "rusty/rc.hpp"

using namespace rusty;

// =============================================================================
// Test 1: Reference outlives Arc (use-after-free)
// =============================================================================
// The reference from get_mut() should not outlive the Arc
// @lifetime violation: ref has lifetime 'a, but Arc is destroyed
int* test_arc_use_after_free() {
    int* bad_ptr;
    {
        auto arc = Arc<int>::make(42);
        // get_mut() returns Option<T&> with lifetime tied to arc
        // @lifetime: (&'a mut self) -> Option<&'a mut T>
        auto opt = arc.get_mut();
        if (opt.is_some()) {
            bad_ptr = &opt.unwrap();  // Taking address of reference
        }
        // arc is destroyed here, bad_ptr now dangles
    }
    return bad_ptr;  // LIFETIME VIOLATION: returning dangling pointer
}

// =============================================================================
// Test 2: Reference escapes scope via assignment
// =============================================================================
int& test_reference_escapes_scope() {
    static int dummy = 0;
    int* ref_holder = &dummy;
    {
        auto arc = Arc<int>::make(100);
        auto opt = arc.get_mut();
        if (opt.is_some()) {
            ref_holder = &opt.unwrap();
        }
        // arc destroyed here
    }
    return *ref_holder;  // LIFETIME VIOLATION: dereferencing dangling pointer
}

// =============================================================================
// Test 3: Storing reference in struct that outlives source
// =============================================================================
struct RefHolder {
    int* ptr;
    RefHolder() : ptr(nullptr) {}
    void store(int& ref) { ptr = &ref; }
    int get() { return *ptr; }
};

int test_struct_holds_dangling_ref() {
    RefHolder holder;
    {
        auto arc = Arc<int>::make(200);
        auto opt = arc.get_mut();
        if (opt.is_some()) {
            holder.store(opt.unwrap());
        }
        // arc destroyed here
    }
    return holder.get();  // LIFETIME VIOLATION: accessing through dangling pointer
}

// =============================================================================
// Test 4: Reference from Rc outlives Rc
// =============================================================================
int* test_rc_use_after_free() {
    int* bad_ptr = nullptr;
    {
        auto rc = Rc<int>::make(42);
        auto opt = rc.get_mut();
        if (opt.is_some()) {
            bad_ptr = &opt.unwrap();
        }
        // rc destroyed here
    }
    return bad_ptr;  // LIFETIME VIOLATION
}

// =============================================================================
// Test 5: Arc replaced while reference is held
// =============================================================================
void test_arc_replaced_while_borrowed() {
    auto arc = Arc<int>::make(1);
    auto opt = arc.get_mut();
    if (opt.is_some()) {
        int& ref = opt.unwrap();
        arc = Arc<int>::make(2);  // VIOLATION: arc reassigned while borrowed
        ref = 100;  // Use after free - original Arc's data is gone
    }
}

// =============================================================================
// Test 6: Valid usage (should NOT be flagged)
// =============================================================================
void test_valid_usage() {
    auto arc = Arc<int>::make(42);
    {
        auto opt = arc.get_mut();
        if (opt.is_some()) {
            opt.unwrap() = 100;  // OK: reference used within arc's lifetime
        }
    }
    // arc still valid here
}

int main() {
    // These should all be caught by the borrow checker at compile/analysis time
    // Running them would cause undefined behavior

    // Uncomment to see runtime crashes:
    // test_arc_use_after_free();
    // test_reference_escapes_scope();
    // test_struct_holds_dangling_ref();
    // test_rc_use_after_free();
    // test_arc_replaced_while_borrowed();

    // This one is safe
    test_valid_usage();

    return 0;
}
