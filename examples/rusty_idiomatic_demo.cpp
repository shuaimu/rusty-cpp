// Demo of Rust-idiomatic API for Rusty types
// Shows both Rust-style and C++-style usage

#include "rusty/rusty.hpp"
#include <cstdio>

extern "C" int printf(const char*, ...);

// @safe
namespace demo {
    
// Example 1: Box with Rust-idiomatic API
// @safe
void demo_box_rust_style() {
    printf("\n=== Box (Rust-style API) ===\n");
    
    // Rust style: Box::new()
    auto box1 = rusty::Box<int>::new_(42);
    printf("Box::new_(42): %d\n", *box1);
    
    // Another Box
    auto box2 = rusty::Box<int>::new_(100);
    printf("Box::new_(100): %d\n", *box2);
    
    // Short form: rusty::make_box()
    auto box3 = rusty::make_box<int>(200);
    printf("rusty::make_box(200): %d\n", *box3);
    
    // C++ style (still works)
    auto box4 = rusty::make_box<int>(300);
    printf("make_box(300): %d\n", *box4);
    
    // Rust-style method: into_raw()
    int* raw = box1.into_raw();
    printf("into_raw() returned: %d\n", *raw);
    delete raw;
}

// Example 2: Arc with Rust-idiomatic API
// @safe
void demo_arc_rust_style() {
    printf("\n=== Arc (Rust-style API) ===\n");
    
    // Rust style: Arc::new()
    auto arc1 = rusty::Arc<int>::new_(42);
    printf("Arc::new_(42): %d, count: %zu\n", *arc1, arc1.strong_count());
    
    // Another Arc
    auto arc2 = rusty::Arc<int>::new_(100);
    
    // Short form: rusty::arc()
    auto arc3 = rusty::arc<int>(200);
    
    // Clone (Rust style)
    auto arc4 = arc1.clone();
    printf("After clone, count: %zu\n", arc1.strong_count());
    
    // C++ style (still works)
    auto arc5 = rusty::make_arc<int>(300);
}

// Example 3: Rc with Rust-idiomatic API
// @safe
void demo_rc_rust_style() {
    printf("\n=== Rc (Rust-style API) ===\n");

    // Create Rc using make()
    auto rc1 = rusty::Rc<int>::make(42);
    printf("Rc::make(42): %d, count: %zu\n", *rc1, rc1.strong_count());

    // Short form: rusty::rc()
    auto rc2 = rusty::rc<int>(100);

    // Clone increases ref count
    auto rc3 = rc1.clone();
    printf("After clone, count: %zu\n", rc1.strong_count());

    // Try to get mutable (Rust pattern)
    auto rc4 = rusty::Rc<int>::make(200);
    if (auto* mut_ref = rc4.get_mut()) {
        *mut_ref = 250;
        printf("Modified unique Rc: %d\n", *rc4);
    }
}

// Example 4: Vec with Rust-idiomatic API
// @safe
void demo_vec_rust_style() {
    printf("\n=== Vec (Rust-style API) ===\n");
    
    // Rust style: Vec::new()
    auto vec1 = rusty::Vec<int>::make();
    vec1.push(10);
    vec1.push(20);
    printf("Vec::make() with pushes, len: %zu\n", vec1.len());
    
    // Rust style: Vec::with_capacity()
    auto vec2 = rusty::Vec<int>::with_capacity(100);
    printf("Vec::with_capacity(100), cap: %zu\n", vec2.cap());
    
    // Rust-style methods
    vec2.push(1);
    vec2.push(2);
    vec2.push(3);
    
    // Rust-style iteration (using begin/end)
    printf("Elements: ");
    for (const auto& elem : vec2) {
        printf("%d ", elem);
    }
    printf("\n");
    
    // is_empty() check (Rust style)
    if (!vec2.is_empty()) {
        printf("Vec is not empty, size: %zu\n", vec2.len());
    }
}

// Example 5: Option with Rust-idiomatic methods
// @safe
void demo_option_rust_style() {
    printf("\n=== Option (Rust-style API) ===\n");
    
    // Create Some and None
    auto some = rusty::Some(42);
    auto none = rusty::Option<int>(rusty::None);
    
    // Rust-style checks
    if (some.is_some()) {
        printf("is_some() = true\n");
    }
    if (none.is_none()) {
        printf("is_none() = true\n");
    }
    
    // expect() with custom message
    auto val = rusty::Some(100);
    int x = val.expect("Value should exist!");
    printf("expect() returned: %d\n", x);
    
    // unwrap_or() with default
    int y = none.unwrap_or(0);
    printf("unwrap_or(0) on None: %d\n", y);
    
    // map() operation
    auto doubled = rusty::Some(21).map([](int n) { return n * 2; });
    if (doubled.is_some()) {
        printf("map(x * 2) = %d\n", doubled.unwrap());
    }
}

// Example 6: Result with Rust-idiomatic methods
// @safe
rusty::Result<int, const char*> safe_divide(int a, int b) {
    if (b == 0) {
        return rusty::Result<int, const char*>::Err("Division by zero");
    }
    return rusty::Result<int, const char*>::Ok(a / b);
}

// @safe
void demo_result_rust_style() {
    printf("\n=== Result (Rust-style API) ===\n");
    
    auto ok_result = safe_divide(10, 2);
    auto err_result = safe_divide(10, 0);
    
    // Rust-style checks
    if (ok_result.is_ok()) {
        printf("is_ok() = true, value: %d\n", ok_result.unwrap());
    }
    
    if (err_result.is_err()) {
        printf("is_err() = true, error: %s\n", err_result.unwrap_err());
    }
    
    // map() on Result
    auto doubled = safe_divide(20, 4).map([](int x) { return x * 2; });
    if (doubled.is_ok()) {
        printf("map(x * 2) on Result: %d\n", doubled.unwrap());
    }
    
    // and_then() for chaining
    auto chained = safe_divide(100, 10)
        .and_then([](int x) { return safe_divide(x, 2); });
    if (chained.is_ok()) {
        printf("Chained operations: %d\n", chained.unwrap());
    }
    
    // unwrap_or() with default
    int safe_val = safe_divide(10, 0).unwrap_or(-1);
    printf("unwrap_or(-1) on error: %d\n", safe_val);
}

// Example 7: Combining types with Rust idioms
// @safe
void demo_combined_rust_style() {
    printf("\n=== Combined Types (Rust-style) ===\n");
    
    // Vec of Boxes
    auto vec = rusty::Vec<rusty::Box<int>>::make();
    vec.push(rusty::Box<int>::new_(1));
    vec.push(rusty::Box<int>::new_(2));
    vec.push(rusty::make_box<int>(3));
    
    printf("Vec<Box<int>> with %zu elements\n", vec.len());
    
    // Option<Arc<T>>
    auto maybe_shared = rusty::Some(rusty::Arc<int>::new_(42));
    if (maybe_shared.is_some()) {
        auto arc = maybe_shared.unwrap();
        printf("Option<Arc<int>>: %d\n", *arc);
    }
    
    // Result<Rc<T>, E>
    auto make_rc_result = [](int val) -> rusty::Result<rusty::Rc<int>, const char*> {
        if (val < 0) {
            return rusty::Result<rusty::Rc<int>, const char*>::Err("Negative value");
        }
        return rusty::Result<rusty::Rc<int>, const char*>::Ok(
            rusty::Rc<int>::make(val)
        );
    };
    
    auto result = make_rc_result(100);
    if (result.is_ok()) {
        auto rc = result.unwrap();
        printf("Result<Rc<int>>: %d\n", *rc);
    }
}

} // namespace demo

int main() {
    printf("Rusty C++ - Rust-idiomatic API Demo\n");
    printf("====================================\n");
    
    demo::demo_box_rust_style();
    demo::demo_arc_rust_style();
    demo::demo_rc_rust_style();
    demo::demo_vec_rust_style();
    demo::demo_option_rust_style();
    demo::demo_result_rust_style();
    demo::demo_combined_rust_style();
    
    printf("\n=== Demo Complete ===\n");
    printf("\nBoth Rust-style (Box::new_) and C++-style (make_box) APIs work!\n");
    return 0;
}