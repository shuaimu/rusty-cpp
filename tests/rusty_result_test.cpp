// Tests for rusty::Result<T, E>
#include "../include/rusty/result.hpp"
#include "../include/rusty/panic.hpp"
#include <cassert>
#include <cstdio>
#include <string>

using namespace rusty;

// Helper function for testing
Result<int, const char*> divide(int a, int b) {
    if (b == 0) {
        return Result<int, const char*>::Err("Division by zero");
    }
    return Result<int, const char*>::Ok(a / b);
}

// Test Ok and Err construction
void test_result_construction() {
    printf("test_result_construction: ");
    {
        auto ok = Ok<int, const char*>(42);
        assert(ok.is_ok());
        assert(!ok.is_err());
        assert(ok.unwrap() == 42);
        
        auto err = Err<int, const char*>("Error message");
        assert(!err.is_ok());
        assert(err.is_err());
        assert(std::string(err.unwrap_err()) == "Error message");
        
        // Using static methods
        auto ok2 = Result<int, std::string>::Ok(100);
        assert(ok2.is_ok());
        
        auto err2 = Result<int, std::string>::Err("Failed");
        assert(err2.is_err());
    }
    printf("PASS\n");
}

// Test function returning Result
void test_result_function() {
    printf("test_result_function: ");
    {
        auto ok_result = divide(10, 2);
        assert(ok_result.is_ok());
        assert(ok_result.unwrap() == 5);
        
        auto err_result = divide(10, 0);
        assert(err_result.is_err());
        assert(std::string(err_result.unwrap_err()) == "Division by zero");
    }
    printf("PASS\n");
}

// Test unwrap_or
void test_result_unwrap_or() {
    printf("test_result_unwrap_or: ");
    {
        auto ok = Ok<int, const char*>(42);
        assert(ok.unwrap_or(0) == 42);
        
        auto err = Err<int, const char*>("Error");
        assert(err.unwrap_or(100) == 100);
    }
    printf("PASS\n");
}

// Test unwrap edge cases
void test_result_unwrap_edge() {
    printf("test_result_unwrap_edge: ");
    {
        auto ok = Ok<int, const char*>(42);
        assert(ok.unwrap() == 42);
        
        auto err = Err<int, const char*>("Error");
        assert(std::string(err.unwrap_err()) == "Error");
        
        // Test unwrap_or
        auto err2 = Err<int, const char*>("Another error");
        assert(err2.unwrap_or(100) == 100);
    }
    printf("PASS\n");
}

// Test map
void test_result_map() {
    printf("test_result_map: ");
    {
        auto ok = divide(20, 2);
        auto doubled = ok.map([](int x) { return x * 2; });
        assert(doubled.is_ok());
        assert(doubled.unwrap() == 20);  // (20/2) * 2
        
        auto err = divide(20, 0);
        auto err_mapped = err.map([](int x) { return x * 2; });
        assert(err_mapped.is_err());
        assert(std::string(err_mapped.unwrap_err()) == "Division by zero");
    }
    printf("PASS\n");
}

// Test map_err
void test_result_map_err() {
    printf("test_result_map_err: ");
    {
        auto ok = divide(20, 2);
        auto ok_mapped = ok.map_err([](const char* e) { 
            return std::string("Error: ") + e; 
        });
        assert(ok_mapped.is_ok());
        assert(ok_mapped.unwrap() == 10);
        
        auto err = divide(20, 0);
        auto err_mapped = err.map_err([](const char* e) { 
            return std::string("Error: ") + e; 
        });
        assert(err_mapped.is_err());
        assert(err_mapped.unwrap_err() == "Error: Division by zero");
    }
    printf("PASS\n");
}

// Test and_then (chaining operations)
void test_result_and_then() {
    printf("test_result_and_then: ");
    {
        auto result = divide(100, 10)
            .and_then([](int x) { return divide(x, 2); });
        assert(result.is_ok());
        assert(result.unwrap() == 5);  // (100/10)/2 = 5
        
        auto err_chain = divide(100, 0)
            .and_then([](int x) { return divide(x, 2); });
        assert(err_chain.is_err());
        
        auto err_later = divide(100, 10)
            .and_then([](int x) { return divide(x, 0); });
        assert(err_later.is_err());
    }
    printf("PASS\n");
}

// Test or_else
void test_result_or_else() {
    printf("test_result_or_else: ");
    {
        auto ok = divide(10, 2);
        auto ok_result = ok.or_else([](const char*) { 
            return Result<int, const char*>::Ok(0); 
        });
        assert(ok_result.is_ok());
        assert(ok_result.unwrap() == 5);  // Original value
        
        auto err = divide(10, 0);
        auto err_result = err.or_else([](const char*) { 
            return Result<int, const char*>::Ok(-1); 
        });
        assert(err_result.is_ok());
        assert(err_result.unwrap() == -1);  // Alternative value
    }
    printf("PASS\n");
}

// Test move semantics
void test_result_move() {
    printf("test_result_move: ");
    {
        auto res1 = Ok<int, const char*>(42);
        auto res2 = std::move(res1);
        // After move, res1 is unspecified but valid
        assert(res2.is_ok());
        assert(res2.unwrap() == 42);
    }
    printf("PASS\n");
}

// Test with custom types
struct CustomError {
    int code;
    std::string message;
};

void test_result_custom_types() {
    printf("test_result_custom_types: ");
    {
        auto ok = Result<std::string, CustomError>::Ok("Success");
        assert(ok.is_ok());
        assert(ok.unwrap() == "Success");
        
        CustomError err{404, "Not found"};
        auto err_result = Result<std::string, CustomError>::Err(err);
        assert(err_result.is_err());
        auto e = err_result.unwrap_err();
        assert(e.code == 404);
        assert(e.message == "Not found");
    }
    printf("PASS\n");
}

// Test bool conversion
void test_result_bool() {
    printf("test_result_bool: ");
    {
        auto ok = Ok<int, const char*>(42);
        if (ok) {
            assert(true);  // Should execute
        } else {
            assert(false);
        }
        
        auto err = Err<int, const char*>("Error");
        if (!err) {
            assert(true);  // Should execute
        } else {
            assert(false);
        }
    }
    printf("PASS\n");
}

// Test complex chaining
void test_result_complex_chain() {
    printf("test_result_complex_chain: ");
    {
        // Chain multiple operations
        auto result = divide(1000, 10)   // Ok(100)
            .map([](int x) { return x + 50; })  // Ok(150)
            .and_then([](int x) { return divide(x, 3); })  // Ok(50)
            .map([](int x) { return x * 2; });  // Ok(100)
        
        assert(result.is_ok());
        assert(result.unwrap() == 100);
        
        // Chain with early error
        auto err_result = divide(1000, 0)   // Err
            .map([](int x) { return x + 50; })
            .and_then([](int x) { return divide(x, 3); })
            .map([](int x) { return x * 2; });
        
        assert(err_result.is_err());
    }
    printf("PASS\n");
}

// Test Result<void, E> pattern
void test_result_void() {
    printf("test_result_void: ");
    {
        using VoidResult = Result<void, const char*>;
        
        auto ok = VoidResult::Ok();
        assert(ok.is_ok());
        
        auto err = VoidResult::Err("Failed");
        assert(err.is_err());
        assert(std::string(err.unwrap_err()) == "Failed");
    }
    printf("PASS\n");
}

void test_result_equality() {
    printf("test_result_equality: ");
    {
        auto ok_a = Result<int, int>::Ok(7);
        auto ok_b = Result<int, int>::Ok(7);
        auto ok_c = Result<int, int>::Ok(9);
        auto err_a = Result<int, int>::Err(7);
        auto err_b = Result<int, int>::Err(7);
        auto err_c = Result<int, int>::Err(9);

        assert(ok_a == ok_b);
        assert(ok_a != ok_c);
        assert(err_a == err_b);
        assert(err_a != err_c);
        assert(ok_a != err_a);
    }
    printf("PASS\n");
}

void test_result_void_equality() {
    printf("test_result_void_equality: ");
    {
        using VoidResult = Result<void, int>;
        auto ok_a = VoidResult::Ok();
        auto ok_b = VoidResult::Ok();
        auto err_a = VoidResult::Err(3);
        auto err_b = VoidResult::Err(3);
        auto err_c = VoidResult::Err(4);

        assert(ok_a == ok_b);
        assert(err_a == err_b);
        assert(err_a != err_c);
        assert(ok_a != err_a);
    }
    printf("PASS\n");
}

void test_result_as_ref_as_mut() {
    printf("test_result_as_ref_as_mut: ");
    {
        auto ok = Result<int, int>::Ok(7);
        auto ok_ref = ok.as_ref();
        assert(ok_ref.is_ok());
        assert(*ok_ref.unwrap() == 7);

        auto ok_mut = ok.as_mut();
        assert(ok_mut.is_ok());
        auto* ok_ptr = ok_mut.unwrap();
        *ok_ptr = 11;
        auto ok_after = ok.as_ref();
        assert(*ok_after.unwrap() == 11);

        auto err = Result<int, int>::Err(9);
        auto err_ref = err.as_ref();
        assert(err_ref.is_err());
        assert(*err_ref.unwrap_err() == 9);

        auto err_mut = err.as_mut();
        assert(err_mut.is_err());
        auto* err_ptr = err_mut.unwrap_err();
        *err_ptr = 15;
        auto err_after = err.as_ref();
        assert(*err_after.unwrap_err() == 15);

        using VoidResult = Result<void, int>;
        auto void_err = VoidResult::Err(3);
        auto void_err_ref = void_err.as_ref();
        assert(void_err_ref.is_err());
        assert(*void_err_ref.unwrap_err() == 3);

        auto void_err_mut = void_err.as_mut();
        assert(void_err_mut.is_err());
        auto* void_err_ptr = void_err_mut.unwrap_err();
        *void_err_ptr = 8;
        auto void_err_after = void_err.as_ref();
        assert(*void_err_after.unwrap_err() == 8);

        auto void_ok = VoidResult::Ok();
        auto void_ok_ref = void_ok.as_ref();
        assert(void_ok_ref.is_ok());
        auto void_ok_mut = void_ok.as_mut();
        assert(void_ok_mut.is_ok());
    }
    printf("PASS\n");
}

void test_result_ok_err_helpers() {
    printf("test_result_ok_err_helpers: ");
    {
        auto ok = Result<int, const char*>::Ok(12);
        auto ok_value = ok.ok();
        assert(ok_value.is_some());
        assert(ok_value.unwrap() == 12);

        auto ok_err = ok.err();
        assert(ok_err.is_none());

        auto err = Result<int, const char*>::Err("oops");
        auto err_value = err.err();
        assert(err_value.is_some());
        assert(std::string(err_value.unwrap()) == "oops");

        auto err_ok = err.ok();
        assert(err_ok.is_none());

        using VoidResult = Result<void, int>;
        auto void_ok = VoidResult::Ok();
        auto void_ok_value = void_ok.ok();
        assert(void_ok_value.is_some());
        assert(void_ok_value.unwrap() == std::tuple<>{});

        auto void_err = VoidResult::Err(9);
        auto void_err_value = void_err.err();
        assert(void_err_value.is_some());
        assert(void_err_value.unwrap() == 9);

        auto void_err_ok = void_err.ok();
        assert(void_err_ok.is_none());
    }
    printf("PASS\n");
}

void test_result_const_unwrap_helpers() {
    printf("test_result_const_unwrap_helpers: ");
    {
        const auto ok = Result<int, int>::Ok(21);
        assert(ok.unwrap() == 21);

        const auto err = Result<int, int>::Err(34);
        assert(err.unwrap_err() == 34);

        using VoidResult = Result<void, int>;
        const auto void_ok = VoidResult::Ok();
        void_ok.unwrap();

        const auto void_err = VoidResult::Err(55);
        assert(void_err.unwrap_err() == 55);
    }
    printf("PASS\n");
}

void test_result_unwrap_or_else_void_callable_compiles() {
    printf("test_result_unwrap_or_else_void_callable_compiles: ");
    {
        auto ok = Result<int, int>::Ok(9);
        int v = ok.unwrap_or_else([](int) { std::abort(); });
        assert(v == 9);
    }
    printf("PASS\n");
}

void test_panic_catch_unwind_handles_begin_panic() {
    printf("test_panic_catch_unwind_handles_begin_panic: ");
    {
        auto result = rusty::panic::catch_unwind(rusty::panic::AssertUnwindSafe([]() {
            rusty::panic::begin_panic("boom");
        }));
        assert(result.is_err());
    }
    printf("PASS\n");
}

void test_panic_resume_unwind_rethrows_payload() {
    printf("test_panic_resume_unwind_rethrows_payload: ");
    {
        auto result = rusty::panic::catch_unwind(rusty::panic::AssertUnwindSafe([]() {
            rusty::panic::begin_panic("boom");
        }));
        assert(result.is_err());
        bool rethrown = false;
        try {
            rusty::panic::resume_unwind(result.unwrap_err());
        } catch (const std::runtime_error& e) {
            rethrown = true;
            assert(std::string(e.what()) == "boom");
        }
        assert(rethrown);
    }
    printf("PASS\n");
}

int main() {
    printf("=== Testing rusty::Result<T, E> ===\n");
    
    test_result_construction();
    test_result_function();
    test_result_unwrap_or();
    test_result_unwrap_edge();
    test_result_map();
    test_result_map_err();
    test_result_and_then();
    test_result_or_else();
    test_result_move();
    test_result_custom_types();
    test_result_bool();
    test_result_complex_chain();
    test_result_void();
    test_result_equality();
    test_result_void_equality();
    test_result_as_ref_as_mut();
    test_result_ok_err_helpers();
    test_result_const_unwrap_helpers();
    test_result_unwrap_or_else_void_callable_compiles();
    test_panic_catch_unwind_handles_begin_panic();
    test_panic_resume_unwind_rethrows_payload();
    
    printf("\nAll Result tests passed!\n");
    return 0;
}
