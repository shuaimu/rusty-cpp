#ifndef SAFE_MATH_H
#define SAFE_MATH_H

// This header demonstrates how safety annotations propagate from headers to implementations

// @safe
namespace SafeMath {
    // All functions in this namespace are safe by default
    int add(int a, int b);
    int subtract(int a, int b);
    int multiply(int a, int b);
    
    // @unsafe
    int divide(int a, int b);  // Unsafe due to potential division by zero
}

// Functions outside the safe namespace

// @safe
class SafeCalculator {
public:
    void set_value(int val);
    int get_value() const;
    void increment();
    
    // @unsafe
    void raw_pointer_operation();
    
private:
    int value;
};

// Individual function annotations

// @safe
int safe_factorial(int n);

// @unsafe  
void unsafe_memory_operation(void* ptr);

// No annotation - defaults to unsafe
void regular_function();

#endif // SAFE_MATH_H