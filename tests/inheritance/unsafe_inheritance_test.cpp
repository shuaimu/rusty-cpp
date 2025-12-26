// Test: Unsafe inheritance patterns
// @safe classes CANNOT inherit from regular (non-@interface) classes

class RegularBase {
public:
    virtual ~RegularBase() = default;
    virtual void foo() {}  // Has implementation, not @interface
};

class AnotherBase {
public:
    int data;  // Has data member, not @interface
    virtual void bar() = 0;
};

// ERROR: @safe class inheriting from non-@interface class
// @safe
class BadDerived1 : public RegularBase {
public:
    void foo() override {}
};

// ERROR: @safe class inheriting from class with data
// @safe
class BadDerived2 : public AnotherBase {
public:
    void bar() override {}
};

// OK: @unsafe class can inherit from anything
// @unsafe
class OkDerived : public RegularBase {
public:
    void foo() override {}
};

// @interface
class IValid {
public:
    virtual ~IValid() = default;
    virtual void process() = 0;
};

// ERROR: @safe class inheriting from both @interface and non-@interface
// @safe
class MixedInheritance : public IValid, public RegularBase {
public:
    void process() override {}
    void foo() override {}
};
