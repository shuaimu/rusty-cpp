
void test() {
    int value = 42;
    int x = 0;
    
    if (x == 0) {
        int& ref1 = value;
        if (x == 1) {
            const int& ref2 = value;  // Error: already mutably borrowed
        }
    }
}
