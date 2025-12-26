// Test: Invalid @interface class definitions
// These should trigger validation errors

// ERROR: @interface with data member
// @interface
class BadInterface1 {
public:
    virtual ~BadInterface1() = default;
    virtual void foo() = 0;
private:
    int data;  // ERROR: interfaces cannot have data members
};

// ERROR: @interface without virtual destructor
// @interface
class BadInterface2 {
public:
    ~BadInterface2() {}  // ERROR: non-virtual destructor
    virtual void foo() = 0;
};

// ERROR: @interface with non-pure-virtual method
// @interface
class BadInterface3 {
public:
    virtual ~BadInterface3() = default;
    virtual void foo() = 0;
    virtual void bar() {}  // ERROR: has implementation (non-pure)
};

// ERROR: @interface with non-virtual method
// @interface
class BadInterface4 {
public:
    virtual ~BadInterface4() = default;
    virtual void foo() = 0;
    void helper() {}  // ERROR: non-virtual method
};
