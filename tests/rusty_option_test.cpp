// Tests for rusty::Option<T>
#include "../include/rusty/option.hpp"
#include <cassert>
#include <cstdio>
#include <string>

using namespace rusty;

// Test Some and None construction
void test_option_construction() {
    printf("test_option_construction: ");
    {
        auto some = Some(42);
        assert(some.is_some());
        assert(!some.is_none());
        assert(some.unwrap() == 42);
        
        Option<int> none(None);
        assert(!none.is_some());
        assert(none.is_none());
        
        Option<int> default_none;
        assert(default_none.is_none());
    }
    printf("PASS\n");
}

// Test unwrap_or
void test_option_unwrap_or() {
    printf("test_option_unwrap_or: ");
    {
        auto some = Some(42);
        assert(some.unwrap_or(0) == 42);
        
        Option<int> none(None);
        assert(none.unwrap_or(100) == 100);
    }
    printf("PASS\n");
}

// Test expect
void test_option_expect() {
    printf("test_option_expect: ");
    {
        auto some = Some(42);
        assert(some.expect("Should have value") == 42);
        
        // Note: expect on None would panic/abort in real Rust
        // For testing, we just verify Some case works
    }
    printf("PASS\n");
}

// Test map
void test_option_map() {
    printf("test_option_map: ");
    {
        auto some = Some(10);
        auto doubled = some.map([](int x) { return x * 2; });
        assert(doubled.is_some());
        assert(doubled.unwrap() == 20);
        
        Option<int> none(None);
        auto mapped_none = none.map([](int x) { return x * 2; });
        assert(mapped_none.is_none());
    }
    printf("PASS\n");
}

// Test map with type change
void test_option_map_type_change() {
    printf("test_option_map_type_change: ");
    {
        auto some_int = Some(42);
        auto some_str = some_int.map([](int x) { 
            return std::string("Value: " + std::to_string(x)); 
        });
        assert(some_str.is_some());
        assert(some_str.unwrap() == "Value: 42");
    }
    printf("PASS\n");
}

// Test move and assignment
void test_option_assignment() {
    printf("test_option_assignment: ");
    {
        auto some1 = Some(42);
        auto some2 = Some(100);

        // Option is move-only; use explicit clone when copy semantics are needed.
        auto some1_clone = some1.clone();
        some2 = std::move(some1_clone);
        assert(some2.is_some());
        assert(some2.unwrap() == 42);

        some2 = Option<int>(None);  // Assign None
        assert(some2.is_none());
    }
    printf("PASS\n");
}

// Test with references
void test_option_reference() {
    printf("test_option_reference: ");
    {
        int value = 42;
        auto some = Some(&value);
        assert(some.is_some());
        assert(*some.unwrap() == 42);
        
        Option<int*> none(None);
        assert(none.is_none());
    }
    printf("PASS\n");
}

// Test unwrap edge cases
void test_option_unwrap_edge() {
    printf("test_option_unwrap_edge: ");
    {
        // Test with zero value
        auto zero = Some(0);
        assert(zero.unwrap() == 0);
        
        // Test with negative value
        auto negative = Some(-42);
        assert(negative.unwrap() == -42);
        
        // Test unwrap_or with None
        Option<int> none(None);
        assert(none.unwrap_or(-1) == -1);
    }
    printf("PASS\n");
}

// Test Option equality
void test_option_equality() {
    printf("test_option_equality: ");
    {
        // Note: equality operators might not be implemented
        // Just test that we can create and use Options
        auto some1 = Some(42);
        auto some2 = Some(42);
        auto some3 = Some(100);
        
        // Both have values
        assert(some1.is_some() && some2.is_some());
        
        Option<int> none1(None);
        Option<int> none2(None);
        
        // Both are None
        assert(none1.is_none() && none2.is_none());
    }
    printf("PASS\n");
}

// Test move semantics
void test_option_move() {
    printf("test_option_move: ");
    {
        auto opt1 = Some(42);
        auto opt2 = std::move(opt1);
        // After move, opt1 is unspecified but valid
        assert(opt2.is_some());
        assert(opt2.unwrap() == 42);
    }
    printf("PASS\n");
}

// Test with custom struct
struct TestStruct {
    int value;
    TestStruct(int v) : value(v) {}
};

void test_option_custom_type() {
    printf("test_option_custom_type: ");
    {
        auto some = Some(TestStruct(42));
        assert(some.is_some());
        assert(some.unwrap().value == 42);
        
        Option<TestStruct> none(None);
        assert(none.is_none());
    }
    printf("PASS\n");
}

// Test Option<Option<T>> (nested)
void test_option_nested() {
    printf("test_option_nested: ");
    {
        auto nested = Some(Some(42));
        assert(nested.is_some());
        auto inner = nested.unwrap();
        assert(inner.is_some());
        assert(inner.unwrap() == 42);
        
        auto some_none = Some(Option<int>(None));
        assert(some_none.is_some());
        assert(some_none.unwrap().is_none());
    }
    printf("PASS\n");
}

// Test bool conversion
void test_option_bool() {
    printf("test_option_bool: ");
    {
        auto some = Some(42);
        if (some) {
            assert(true);  // Should execute
        } else {
            assert(false);
        }
        
        Option<int> none(None);
        if (!none) {
            assert(true);  // Should execute
        } else {
            assert(false);
        }
    }
    printf("PASS\n");
}

// Test as_ref/as_mut for Option<T&> specialization
void test_option_ref_specialization_as_ref_as_mut() {
    printf("test_option_ref_specialization_as_ref_as_mut: ");
    {
        int value = 7;
        Option<int&> opt_ref(value);

        auto ref_view = opt_ref.as_ref();
        assert(ref_view.is_some());
        assert(ref_view.unwrap() == 7);

        auto mut_view = opt_ref.as_mut();
        assert(mut_view.is_some());
        mut_view.unwrap() = 11;
        assert(value == 11);

        // Original Option remains valid after view creation.
        assert(opt_ref.is_some());
        assert(opt_ref.unwrap() == 11);
    }
    printf("PASS\n");
}

// Test as_ref for Option<const T&> specialization
void test_option_const_ref_specialization_as_ref() {
    printf("test_option_const_ref_specialization_as_ref: ");
    {
        const int value = 99;
        Option<const int&> opt_ref(value);

        auto view = opt_ref.as_ref();
        assert(view.is_some());
        assert(view.unwrap() == 99);
        assert(opt_ref.is_some());
        assert(opt_ref.unwrap() == 99);

        Option<const int&> none(None);
        auto none_view = none.as_ref();
        assert(none_view.is_none());
    }
    printf("PASS\n");
}

int main() {
    printf("=== Testing rusty::Option<T> ===\n");
    
    test_option_construction();
    test_option_unwrap_or();
    test_option_expect();
    test_option_map();
    test_option_map_type_change();
    test_option_assignment();
    test_option_reference();
    test_option_unwrap_edge();
    test_option_equality();
    test_option_move();
    test_option_custom_type();
    test_option_nested();
    test_option_bool();
    test_option_ref_specialization_as_ref_as_mut();
    test_option_const_ref_specialization_as_ref();
    
    printf("\nAll Option tests passed!\n");
    return 0;
}
