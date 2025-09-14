#include <sys/time.h>

// @safe
void test_function() {
    struct timeval now;  // Line 5 - variable declaration
    timeval later;       // Line 6 - variable declaration without struct keyword
    gettimeofday(&now, nullptr);  // Line 7 - actual function call
}