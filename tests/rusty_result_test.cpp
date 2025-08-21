// Tests for rusty::Result<T, E>
#include "../include/rusty/result.hpp"
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
    
    printf("\nAll Result tests passed!\n");
    return 0;
}