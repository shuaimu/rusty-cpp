// Test: Interface composition and inheritance chains
// @interface classes can only inherit from other @interface classes

// @interface
class IBase {
public:
    virtual ~IBase() = default;
    virtual void base_method() = 0;
};

// OK: @interface inheriting from @interface
// @interface
class IDerived : public IBase {
public:
    virtual void derived_method() = 0;
};

// OK: @interface inheriting from multiple @interface classes
// @interface
class IBase2 {
public:
    virtual ~IBase2() = default;
    virtual void other_method() = 0;
};

// @interface
class IMultiDerived : public IBase, public IBase2 {
public:
    virtual void multi_method() = 0;
};

// ERROR: @interface inheriting from non-@interface
class ConcreteBase {
public:
    virtual ~ConcreteBase() = default;
    virtual void concrete() {}  // Has implementation
};

// @interface - ERROR: should not inherit from non-@interface
// @interface
class IBadInterface : public ConcreteBase {
public:
    virtual void pure_method() = 0;
};

// @safe class can implement entire interface hierarchy
// @safe
class Implementation : public IDerived {
public:
    void base_method() override {}
    void derived_method() override {}
};

// @safe class implementing multi-interface hierarchy
// @safe
class MultiImpl : public IMultiDerived {
public:
    void base_method() override {}
    void other_method() override {}
    void multi_method() override {}
};
