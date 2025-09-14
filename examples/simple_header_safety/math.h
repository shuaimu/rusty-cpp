#ifndef MATH_H
#define MATH_H

// Example demonstrating header safety annotations propagating to implementations

// @safe
int safe_add(int a, int b);

// @unsafe
int unsafe_divide(int a, int b);

// No annotation - defaults to unsafe
int regular_multiply(int a, int b);

#endif