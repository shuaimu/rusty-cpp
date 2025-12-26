// Test: Valid @interface class definitions
// These should pass all validation checks

// @interface
class IDrawable {
public:
    virtual ~IDrawable() = default;
    virtual void draw() const = 0;
    virtual void resize(int width, int height) = 0;
};

// @interface
class ISerializable {
public:
    virtual ~ISerializable() {}
    virtual void serialize() const = 0;
    virtual void deserialize() = 0;
};

// @interface with multiple pure virtual methods
class IDataProcessor {
public:
    virtual ~IDataProcessor() = default;
    virtual void process() = 0;
    virtual bool validate() const = 0;
    virtual void reset() = 0;
    virtual int getStatus() const = 0;
};

// @safe class implementing @interface (should be allowed)
// @safe
class Circle : public IDrawable {
public:
    void draw() const override {
        // Safe implementation
    }
    void resize(int width, int height) override {
        // Safe implementation
    }
};

// Multiple interface inheritance (should be allowed in @safe)
// @safe
class Document : public IDrawable, public ISerializable {
public:
    void draw() const override {}
    void resize(int, int) override {}
    void serialize() const override {}
    void deserialize() override {}
};

// @safe
void use_interfaces() {
    Circle c;
    c.draw();

    Document d;
    d.serialize();
}
