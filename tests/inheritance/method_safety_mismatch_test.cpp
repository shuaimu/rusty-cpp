// Test: Method safety contract mismatch detection
// When implementation has explicit annotation that conflicts with interface

// @interface
class IProcessor {
public:
    virtual ~IProcessor() = default;

    // @safe
    virtual int process(int x) = 0;

    // @unsafe
    virtual void* rawAlloc(size_t n) = 0;
};

// ERROR: Implementation marks @safe method as @unsafe
class MismatchHandler1 : public IProcessor {
public:
    // @unsafe - MISMATCH: interface says @safe!
    int process(int x) override {
        return x * 2;
    }

    // @unsafe - OK: matches interface
    void* rawAlloc(size_t n) override {
        return nullptr;
    }
};

// ERROR: Implementation marks @unsafe method as @safe
class MismatchHandler2 : public IProcessor {
public:
    // @safe - OK: matches interface
    int process(int x) override {
        return x * 2;
    }

    // @safe - MISMATCH: interface says @unsafe!
    void* rawAlloc(size_t n) override {
        return nullptr;
    }
};
