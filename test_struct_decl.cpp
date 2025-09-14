// @safe
void test_struct_declaration() {
    struct timeval tv;      // This should not be a function call
    timeval tv2;            // This should also not be a function call
    int x = 5;              // Regular variable declaration
}