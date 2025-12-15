// Test: Double Free and Use After Free with Raw Pointers
// Status: NOT DETECTED (requires RAII tracking Phase 6)
//
// While modern C++ prefers smart pointers, raw new/delete is still
// common. We should detect double-free and use-after-free.

#include <cstdlib>

// =============================================================================
// NEGATIVE TESTS - Should produce errors after implementation
// =============================================================================

// @unsafe
void bad_double_delete() {
    int* ptr = new int(42);
    delete ptr;
    delete ptr;  // ERROR: double free
}

// @unsafe
void bad_use_after_delete() {
    int* ptr = new int(42);
    delete ptr;
    *ptr = 10;  // ERROR: use after free
}

// @unsafe
void bad_use_after_delete_read() {
    int* ptr = new int(42);
    delete ptr;
    int x = *ptr;  // ERROR: read after free
}

// @unsafe
void bad_delete_then_return() {
    int* ptr = new int(42);
    delete ptr;
    // ERROR: returning deleted pointer
    // (caller might try to use it)
}

// @unsafe
void bad_conditional_double_free(bool condition) {
    int* ptr = new int(42);
    if (condition) {
        delete ptr;
    }
    delete ptr;  // ERROR: might be double free
}

// @unsafe
void bad_loop_double_free() {
    int* ptr = new int(42);
    for (int i = 0; i < 2; i++) {
        delete ptr;  // ERROR: second iteration is double free
    }
}

// @unsafe
void bad_delete_stack_variable() {
    int x = 42;
    int* ptr = &x;
    delete ptr;  // ERROR: deleting stack memory
}

// @unsafe
void bad_delete_static() {
    static int x = 42;
    delete &x;  // ERROR: deleting static memory
}

// Array delete mismatch
// @unsafe
void bad_array_delete_mismatch() {
    int* ptr = new int[10];
    delete ptr;  // ERROR: should be delete[]
}

// @unsafe
void bad_scalar_delete_array() {
    int* ptr = new int(42);
    delete[] ptr;  // ERROR: should be delete (not delete[])
}

// malloc/free with new/delete
// @unsafe
void bad_malloc_delete() {
    int* ptr = (int*)malloc(sizeof(int));
    delete ptr;  // ERROR: malloc'd memory should use free()
}

// @unsafe
void bad_new_free() {
    int* ptr = new int(42);
    free(ptr);  // ERROR: new'd memory should use delete
}

// Use after free through alias
// @unsafe
void bad_alias_use_after_free() {
    int* ptr1 = new int(42);
    int* ptr2 = ptr1;  // ptr2 aliases ptr1
    delete ptr1;
    *ptr2 = 10;  // ERROR: use after free through alias
}

// =============================================================================
// POSITIVE TESTS - Should NOT produce errors
// =============================================================================

// @unsafe
void good_new_delete_pair() {
    int* ptr = new int(42);
    *ptr = 100;  // OK: ptr is valid
    delete ptr;
    // ptr not used after delete - OK
}

// @unsafe
void good_array_new_delete() {
    int* arr = new int[10];
    arr[0] = 1;
    delete[] arr;  // Correct: array delete for array new
}

// @unsafe
void good_malloc_free_pair() {
    int* ptr = (int*)malloc(sizeof(int));
    *ptr = 42;
    free(ptr);  // Correct: free for malloc
}

// @unsafe
void good_conditional_delete(bool condition) {
    int* ptr = new int(42);
    if (condition) {
        delete ptr;
        ptr = nullptr;  // Good practice: null after delete
    }
    if (ptr) {
        delete ptr;  // Only deletes if not already deleted
    }
}

// @unsafe
void good_reassign_after_delete() {
    int* ptr = new int(42);
    delete ptr;
    ptr = new int(100);  // Reassign to new allocation - OK
    *ptr = 200;  // Valid
    delete ptr;
}

// @unsafe
int* good_return_new() {
    return new int(42);  // OK: caller takes ownership
}

// @unsafe
void good_take_ownership(int* ptr) {
    // Assuming we take ownership
    *ptr = 100;
    delete ptr;  // OK if we own it
}
