// Demo of Rust-inspired safe types for C++
// These types work with the Rusty C++ Checker to ensure memory safety

#include "rusty/rusty.hpp"
#include <cstdio>

extern "C" int printf(const char*, ...);

// @safe
namespace demo {
    
// Example 1: Box - Single ownership
// @safe
void demo_box() {
    printf("\n=== Box Demo ===\n");
    
    // Create a Box (heap-allocated, single owner)
    auto box1 = rusty::make_box<int>(42);
    printf("Box value: %d\n", *box1);
    
    // Move ownership (box1 becomes invalid)
    auto box2 = std::move(box1);
    printf("After move, box2: %d\n", *box2);
    
    // This would be caught by the checker:
    // printf("box1: %d\n", *box1);  // Error: use after move
    
    // Box automatically deallocates when it goes out of scope
}

// Example 2: Arc - Thread-safe shared ownership
// @safe
void demo_arc() {
    printf("\n=== Arc Demo ===\n");
    
    // Create an Arc (atomically reference counted)
    auto arc1 = rusty::make_arc<int>(100);
    printf("Arc value: %d, ref count: %zu\n", *arc1, arc1.strong_count());
    
    // Clone increases reference count
    auto arc2 = arc1.clone();
    printf("After clone, ref count: %zu\n", arc1.strong_count());
    
    // Both can read the value (immutable access only)
    printf("arc1: %d, arc2: %d\n", *arc1, *arc2);
    
    // Moving doesn't change ref count
    auto arc3 = std::move(arc1);
    printf("After move, arc3 ref count: %zu\n", arc3.strong_count());
    
    // arc1 is now invalid (moved from)
    // printf("arc1: %d\n", *arc1);  // Error: use after move
}

// Example 3: Vec - Dynamic array with ownership
// @safe
void demo_vec() {
    printf("\n=== Vec Demo ===\n");
    
    // Create a Vec
    rusty::Vec<int> vec;
    vec.push(10);
    vec.push(20);
    vec.push(30);
    
    printf("Vec size: %zu\n", vec.len());
    printf("Elements: ");
    for (size_t i = 0; i < vec.len(); ++i) {
        printf("%d ", vec[i]);
    }
    printf("\n");
    
    // Pop from the back
    int last = vec.pop();
    printf("Popped: %d, new size: %zu\n", last, vec.len());
    
    // Move semantics
    auto vec2 = std::move(vec);
    printf("After move, vec2 size: %zu\n", vec2.len());
    
    // vec is now invalid
    // vec.push(40);  // Error: use after move
}

// Example 4: Option - Nullable values
// @safe
void demo_option() {
    printf("\n=== Option Demo ===\n");
    
    // Create Some and None variants
    rusty::Option<int> some_value = rusty::Some(42);
    rusty::Option<int> no_value = rusty::None;
    
    // Check if value exists
    if (some_value.is_some()) {
        printf("Has value: %d\n", some_value.as_ref().unwrap());
    }
    
    if (no_value.is_none()) {
        printf("No value present\n");
    }
    
    // Map over Option
    auto doubled = some_value.map_ref([](int x) { return x * 2; });
    printf("Doubled: %d\n", doubled.unwrap());
    
    // Unwrap with default
    int val = no_value.unwrap_or(0);
    printf("Default value: %d\n", val);
}

// Example 5: Result - Error handling
// @safe
rusty::Result<int, const char*> divide(int a, int b) {
    if (b == 0) {
        return rusty::Result<int, const char*>::Err("Division by zero");
    }
    return rusty::Result<int, const char*>::Ok(a / b);
}

// @safe
void demo_result() {
    printf("\n=== Result Demo ===\n");
    
    auto result1 = divide(10, 2);
    auto result2 = divide(10, 0);
    
    // Check and unwrap
    if (result1.is_ok()) {
        printf("10 / 2 = %d\n", result1.unwrap());
    }
    
    if (result2.is_err()) {
        printf("Error: %s\n", result2.unwrap_err());
    }
    
    // Map over Result
    auto doubled = divide(20, 4).map([](int x) { return x * 2; });
    if (doubled.is_ok()) {
        printf("(20 / 4) * 2 = %d\n", doubled.unwrap());
    }
    
    // Chain operations
    auto chained = divide(100, 5)
        .and_then([](int x) { return divide(x, 2); });
    if (chained.is_ok()) {
        printf("(100 / 5) / 2 = %d\n", chained.unwrap());
    }
}

// Example 6: Rc - Single-threaded reference counting
// @safe
void demo_rc() {
    printf("\n=== Rc Demo ===\n");
    
    // Create an Rc (non-atomic reference counting)
    auto rc1 = rusty::make_rc<int>(50);
    printf("Rc value: %d, ref count: %zu\n", *rc1, rc1.strong_count());
    
    // Clone increases reference count
    auto rc2 = rc1.clone();
    printf("After clone, ref count: %zu\n", rc1.strong_count());
    
    // Get mutable reference if unique
    auto rc3 = rusty::make_rc<int>(75);
    if (auto opt = rc3.get_mut(); opt.is_some()) {
        opt.unwrap() = 80;
        printf("Modified unique Rc: %d\n", *rc3);
    }

    // Can't modify when shared
    auto rc4 = rc3.clone();
    if (rc3.get_mut().is_none()) {
        printf("Cannot modify shared Rc\n");
    }
}

// Example 7: Combining types
struct Data {
    int value;
    Data(int v) : value(v) { printf("Data(%d) created\n", v); }
    ~Data() { printf("Data(%d) destroyed\n", value); }
};

// @safe
void demo_combined() {
    printf("\n=== Combined Types Demo ===\n");
    
    // Vec of Boxes
    rusty::Vec<rusty::Box<Data>> vec;
    vec.push(rusty::make_box<Data>(1));
    vec.push(rusty::make_box<Data>(2));
    vec.push(rusty::make_box<Data>(3));
    
    printf("Vec has %zu boxes\n", vec.len());
    
    // Option of Arc
    rusty::Option<rusty::Arc<Data>> maybe_shared = 
        rusty::Some(rusty::make_arc<Data>(100));
    
    if (maybe_shared.is_some()) {
        auto arc = maybe_shared.unwrap();
        printf("Shared data: %d\n", arc->value);
    }
    
    // Result with Box
    auto make_data = [](int v) -> rusty::Result<rusty::Box<Data>, const char*> {
        if (v < 0) {
            return rusty::Result<rusty::Box<Data>, const char*>::Err("Negative value");
        }
        return rusty::Result<rusty::Box<Data>, const char*>::Ok(
            rusty::make_box<Data>(v)
        );
    };
    
    auto result = make_data(42);
    if (result.is_ok()) {
        auto box = result.unwrap();
        printf("Created box with value: %d\n", box->value);
    }
}

} // namespace demo

int main() {
    printf("Rusty C++ Types Demo\n");
    printf("====================\n");
    
    demo::demo_box();
    demo::demo_arc();
    demo::demo_vec();
    demo::demo_option();
    demo::demo_result();
    demo::demo_rc();
    demo::demo_combined();
    
    printf("\n=== Demo Complete ===\n");
    return 0;
}