// Test: Lambda Capture Escaping Scope
// Status: PARTIALLY DETECTED (capture safety exists, but not escape analysis)
//
// When a lambda captures by reference, the lambda must not outlive
// the captured variables.

#include <functional>
#include <vector>
#include <memory>

// =============================================================================
// NEGATIVE TESTS - Should produce errors after implementation
// =============================================================================

// @safe
std::function<int()> bad_lambda_escape_ref() {
    int x = 42;
    auto lambda = [&x]() { return x; };  // Captures x by reference
    return lambda;  // ERROR: lambda escapes, but x will be destroyed
}

// @safe
std::function<int()> bad_lambda_escape_ref_all() {
    int x = 42;
    int y = 100;
    auto lambda = [&]() { return x + y; };  // Captures all by reference
    return lambda;  // ERROR: both x and y will be destroyed
}

// @safe
void bad_lambda_stored_escapes() {
    std::function<int()> stored;
    {
        int x = 42;
        stored = [&x]() { return x; };
    }  // x destroyed

    // ERROR: stored lambda has dangling reference
    int val = stored();
}

// @safe
void bad_lambda_in_vector() {
    std::vector<std::function<int()>> funcs;
    {
        int x = 42;
        funcs.push_back([&x]() { return x; });
    }  // x destroyed

    // ERROR: funcs[0] has dangling capture
    int val = funcs[0]();
}

// @safe
std::function<int()> bad_lambda_captures_local_obj() {
    std::string s = "hello";
    auto lambda = [&s]() { return s.length(); };
    return lambda;  // ERROR: s will be destroyed
}

// @safe
void bad_nested_lambda_escape() {
    std::function<std::function<int()>()> outer;
    {
        int x = 42;
        outer = [&x]() {
            return [&x]() { return x; };  // Inner also captures x
        };
    }  // x destroyed

    // ERROR: both outer and inner lambdas have dangling refs
    auto inner = outer();
    int val = inner();
}

// @safe - Currently caught by lambda capture check, but escape not caught
std::function<void()> bad_lambda_captures_this() {
    struct Local {
        int value = 42;
        std::function<void()> get_lambda() {
            return [this]() { value++; };  // Captures this
        }
    };

    Local obj;
    return obj.get_lambda();  // ERROR: obj destroyed, this dangling
}

// =============================================================================
// POSITIVE TESTS - Should NOT produce errors
// =============================================================================

// @safe
std::function<int()> good_lambda_capture_copy() {
    int x = 42;
    auto lambda = [x]() { return x; };  // Captures x by COPY
    return lambda;  // OK: lambda owns its own copy
}

// @safe
std::function<int()> good_lambda_capture_all_copy() {
    int x = 42;
    int y = 100;
    auto lambda = [=]() { return x + y; };  // Captures all by COPY
    return lambda;  // OK: lambda owns copies
}

// @safe
std::function<int()> good_lambda_capture_move() {
    auto ptr = std::make_unique<int>(42);
    auto lambda = [p = std::move(ptr)]() { return *p; };
    return lambda;  // OK: lambda owns the unique_ptr
}

// @safe
void good_lambda_used_in_scope() {
    int x = 42;
    auto lambda = [&x]() { return x; };
    int val = lambda();  // OK: x still alive
}

// @safe
void good_lambda_passed_down() {
    int x = 42;
    auto lambda = [&x]() { return x; };

    // Passing to function that uses it immediately
    auto use_lambda = [](std::function<int()>& f) { return f(); };
    int val = use_lambda(lambda);  // OK: x still alive during call
}

// @safe
void good_lambda_in_algorithm() {
    std::vector<int> v = {1, 2, 3, 4, 5};
    int sum = 0;

    // Lambda with reference capture used immediately
    std::for_each(v.begin(), v.end(), [&sum](int x) { sum += x; });
    // OK: sum is alive for entire for_each call
}

// @safe
std::function<int(int)> good_lambda_no_capture() {
    auto lambda = [](int x) { return x * 2; };
    return lambda;  // OK: no captures, nothing can dangle
}

// @safe
void good_lambda_captures_param(int& x) {
    auto lambda = [&x]() { return x; };
    int val = lambda();  // OK: x is owned by caller, presumed to outlive us
}
