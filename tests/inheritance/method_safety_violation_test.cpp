// Test: Method safety contract violations
// Implementations that violate interface safety contracts

// @interface
class IProcessor {
public:
    virtual ~IProcessor() = default;

    // @safe
    virtual void must_be_safe() = 0;

    // @unsafe
    virtual void must_be_unsafe() = 0;
};

// ERROR: Implementation marks @safe method as @unsafe
class BadHandler1 : public IProcessor {
public:
    // ERROR: interface says @safe, implementation says @unsafe
    // @unsafe
    void must_be_safe() override {
        int* ptr = nullptr;
    }

    // @unsafe
    void must_be_unsafe() override {}
};

// ERROR: Implementation marks @unsafe method as @safe
class BadHandler2 : public IProcessor {
public:
    // @safe
    void must_be_safe() override {}

    // ERROR: interface says @unsafe, implementation says @safe
    // (this might actually be ok to strengthen, TBD)
    // @safe
    void must_be_unsafe() override {
        int x = 42;
    }
};

// @interface with all @safe methods
// @interface
class ISafeOnly {
public:
    virtual ~ISafeOnly() = default;

    // @safe
    virtual void op1() = 0;

    // @safe
    virtual void op2() = 0;
};

// ERROR: @safe class implementing @safe interface but using unsafe operations
// @safe
class BadSafeImpl : public ISafeOnly {
public:
    // Should be @safe but uses pointer operations
    void op1() override {
        int* ptr = nullptr;  // ERROR: unsafe in @safe context
    }

    void op2() override {}
};
