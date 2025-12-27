// Test: String literal safety in @safe code
// String literals are safe because they have static lifetime.
// Explicit char* variables are unsafe because their origin is unknown.

// @safe
void log(const char* msg);

// @safe
void printf_safe(const char* fmt, ...);

// Test 1: String literal directly passed - SAFE
// @safe
void test1_literal_direct() {
    log("hello");  // OK - string literal is safe
}

// Test 2: Multiple string literals - SAFE
// @safe
void test2_multiple_literals() {
    printf_safe("%s %s\n", "hello", "world");  // OK - all literals
}

// Test 3: char* variable declaration - UNSAFE
// @safe
void test3_char_ptr_variable() {
    const char* ptr = "hello";  // ERROR - char* variable in @safe
    log(ptr);
}

// Test 4: char* in @unsafe block - OK
// @safe
void test4_char_ptr_in_unsafe() {
    // @unsafe
    {
        const char* ptr = "hello";  // OK - in @unsafe block
        log(ptr);  // OK
    }
}

// Test 5: Safe wrapper pattern
// @safe
void safe_log(const char* msg) {
    // @unsafe
    {
        log(msg);  // OK - inside @unsafe
    }
}

// @safe
void test5_safe_wrapper() {
    safe_log("hello");  // OK - passing literal to safe wrapper
}

// Test 6: Wide string literals - SAFE
// @safe
void test6_wide_literal() {
    const wchar_t* wide = L"hello";  // ERROR - wchar_t* is still char*
}

// Test 7: Raw string literals (C++11) - SAFE
// @safe
void test7_raw_literal() {
    log(R"(hello "world")");  // OK - raw string literal is safe
}
