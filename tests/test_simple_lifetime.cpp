// Simple test for lifetime checking - no template dependencies
// These patterns should be detectable by the RAII tracker

// @safe
int* test_dangling_pointer() {
    int* ptr;
    {
        int x = 42;
        ptr = &x;  // ptr borrows from x
    }  // x goes out of scope, ptr is now dangling
    return ptr;  // LIFETIME VIOLATION: returning dangling pointer
}

// @safe
int& test_dangling_reference() {
    int* ref_holder;
    {
        int y = 100;
        ref_holder = &y;  // ref_holder borrows from y
    }  // y goes out of scope
    return *ref_holder;  // LIFETIME VIOLATION: dereferencing dangling pointer
}

// @safe
void test_reassign_while_borrowed() {
    int x = 1;
    int& ref = x;  // ref borrows from x
    x = 2;  // VIOLATION: cannot assign to x while it is borrowed
    ref = 3;  // Using the borrow
}

// @safe
void test_valid_usage() {
    int x = 42;
    {
        int& ref = x;  // ref borrows from x
        ref = 100;  // OK: x is still in scope
    }  // ref goes out of scope, borrow ends
    x = 200;  // OK: no active borrows
}

int main() {
    test_valid_usage();
    return 0;
}
