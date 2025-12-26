// Test: Method safety contracts on interfaces
// Interface methods can be marked @safe or @unsafe
// Implementations must honor the same safety level

// @interface
class IDataHandler {
public:
    virtual ~IDataHandler() = default;

    // @safe
    virtual void process_safe() const = 0;

    // @unsafe
    virtual void process_unsafe() const = 0;

    // No annotation - inherits from class context
    virtual void process_default() const = 0;
};

// OK: Implementation honors @safe contract
// @safe
class SafeHandler : public IDataHandler {
public:
    // OK: @safe matches interface
    // @safe
    void process_safe() const override {
        int x = 42;  // Safe operations only
    }

    // OK: @unsafe matches interface (using unsafe block to call from @safe class)
    // @unsafe
    void process_unsafe() const override {
        int* ptr = nullptr;  // Can use unsafe operations
    }

    void process_default() const override {
        // Inherits safety from class
    }
};

// For validation: explicit annotation on implementation
// @safe
class ExplicitSafeHandler : public IDataHandler {
public:
    // @safe - explicitly marked, should match interface
    void process_safe() const override {}

    // @unsafe - explicitly marked
    void process_unsafe() const override {}

    void process_default() const override {}
};
