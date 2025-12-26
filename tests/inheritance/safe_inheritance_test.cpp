// Test: Safe inheritance from @interface
// @safe classes can only inherit from @interface classes

// @interface
class IProcessor {
public:
    virtual ~IProcessor() = default;
    virtual void process() = 0;
};

// @interface
class IValidator {
public:
    virtual ~IValidator() = default;
    virtual bool validate() const = 0;
};

// OK: @safe class inherits from @interface
// @safe
class SafeProcessor : public IProcessor {
public:
    void process() override {
        int x = 42;
    }
};

// OK: @safe class inherits from multiple @interface classes
// @safe
class SafeValidatingProcessor : public IProcessor, public IValidator {
public:
    void process() override {
        int x = 42;
    }
    bool validate() const override {
        return true;
    }
};

// @safe
void test_safe_inheritance() {
    SafeProcessor sp;
    sp.process();

    SafeValidatingProcessor svp;
    svp.validate();
}
