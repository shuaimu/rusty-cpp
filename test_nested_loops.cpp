// @safe
void test() {
    int value = 42;
    
    for (int i = 0; i < 2; i++) {
        int& ref1 = value;
        for (int j = 0; j < 2; j++) {
            const int& ref2 = value;  // Should error - mutable borrow exists
        }
    }
}
