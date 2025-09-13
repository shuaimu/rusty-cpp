# In-Place Lifetime Annotations

RustyCpp supports inline lifetime annotations that can be placed directly in your C++ source code. These annotations are written as special comments that the analyzer recognizes and uses to understand lifetime relationships.

## Overview

In-place annotations allow you to:
- Specify lifetime relationships for function signatures
- Mark functions as safe or unsafe
- Define lifetime constraints directly where functions are declared
- Gradually add safety to existing codebases

## Basic Syntax

### Function-Level Lifetime Annotations

Place lifetime annotations as comments directly above function declarations:

```cpp
// @lifetime: (&'a) -> &'a
const int& identity(const int& x) {
    return x;
}

// @lifetime: owned
std::unique_ptr<Resource> createResource() {
    return std::make_unique<Resource>();
}

// @lifetime: (&'a, &'b) -> &'a where 'a: 'b
const std::string& selectLonger(const std::string& a, const std::string& b) {
    return a.length() > b.length() ? a : b;
}
```

### Safety Annotations

Mark functions or namespaces as safe or unsafe:

```cpp
// @safe
void processData() {
    // This function is checked for safety
    std::vector<int> data = {1, 2, 3};
    int& ref = data[0];
    // data.push_back(4);  // ERROR: Would invalidate ref
}

// @unsafe
void lowLevelOperation() {
    // Unsafe operations allowed here
    void* buffer = malloc(100);
    memcpy(buffer, source, 100);
    free(buffer);
}
```

## Lifetime Annotation Components

### 1. Parameter Lifetimes

Specify how parameters relate to each other:

```cpp
// Single parameter with lifetime
// @lifetime: (&'a) -> T
int getValue(const Container& cont);

// Multiple parameters with different lifetimes
// @lifetime: (&'a, &'b) -> void
void process(const Input& in, Output& out);

// Mutable reference
// @lifetime: (&'a mut) -> void
void modify(Data& data);
```

### 2. Return Lifetimes

Specify how return values relate to parameters:

```cpp
// Return has same lifetime as parameter
// @lifetime: (&'a) -> &'a
const Element& getElement(const Container& c);

// Return is owned (new object)
// @lifetime: (&'a) -> owned
std::string toString(const Data& d);

// Return has static lifetime
// @lifetime: () -> &'static
const Config& getGlobalConfig();
```

### 3. Lifetime Constraints (Where Clauses)

Express complex relationships between lifetimes:

```cpp
// 'a must outlive 'b
// @lifetime: (&'a, &'b) -> &'a where 'a: 'b
const T& keepFirst(const T& long_lived, const T& short_lived);

// Multiple constraints
// @lifetime: (&'a, &'b, &'c) -> &'a where 'a: 'b, 'b: 'c
const T& complex(const T& a, const T& b, const T& c);
```

## Common Patterns

### Borrowing Pattern

Function returns a reference into its parameter:

```cpp
class Container {
    std::vector<Item> items;
    
    // @lifetime: (&'a, size_t) -> &'a
    const Item& operator[](size_t index) const {
        return items[index];
    }
    
    // @lifetime: (&'a mut, size_t) -> &'a mut
    Item& operator[](size_t index) {
        return items[index];
    }
};
```

### Factory Pattern

Function creates and returns a new object:

```cpp
// @lifetime: owned
std::unique_ptr<Widget> createWidget(int type) {
    switch(type) {
        case 1: return std::make_unique<ButtonWidget>();
        case 2: return std::make_unique<LabelWidget>();
        default: return std::make_unique<DefaultWidget>();
    }
}
```

### Transformation Pattern

Function transforms input without taking ownership:

```cpp
// @lifetime: (&'a) -> owned
std::string processString(const std::string& input) {
    std::string result = input;
    // Transform result...
    return result;  // Returns new owned string
}
```

### Selector Pattern

Function selects between multiple inputs:

```cpp
// @lifetime: (&'a, &'b, bool) -> &'a where 'a: 'b
const Data& selectData(const Data& primary, const Data& fallback, bool usePrimary) {
    return usePrimary ? primary : fallback;
}
```

## Class Member Functions

Annotate member functions with lifetime relationships:

```cpp
class DataStore {
private:
    std::map<std::string, Data> store;
    
public:
    // @lifetime: (&'a, const string&) -> &'a
    const Data& get(const std::string& key) const {
        return store.at(key);
    }
    
    // @lifetime: (&'a mut, const string&) -> &'a mut
    Data& get_mut(const std::string& key) {
        return store[key];
    }
    
    // @lifetime: (&'a mut, string, Data) -> void
    void insert(std::string key, Data value) {
        store[std::move(key)] = std::move(value);
    }
    
    // @lifetime: owned
    Data remove(const std::string& key) {
        Data result = std::move(store[key]);
        store.erase(key);
        return result;
    }
};
```

## Template Functions

Annotate template functions with generic lifetime relationships:

```cpp
// @lifetime: (&'a) -> &'a
template<typename T>
const T& identity(const T& x) {
    return x;
}

// @lifetime: (&'a, &'b) -> owned
template<typename T, typename U>
auto combine(const T& a, const U& b) {
    return std::make_pair(a, b);
}

// @lifetime: (&'a mut) -> void
template<typename Container>
void clearContainer(Container& c) {
    c.clear();
}
```

## Combining with Safety Annotations

Use both lifetime and safety annotations together:

```cpp
// @safe
// @lifetime: (&'a, &'b) -> &'a where 'a: 'b
const Buffer& processBuffer(const Buffer& input, const Config& config) {
    // Safe function with lifetime checking
    validateConfig(config);
    return input;  // Return has lifetime of input, not config
}

// @unsafe
// @lifetime: (size_t) -> owned
void* allocateBuffer(size_t size) {
    // Unsafe allocation but with clear ownership transfer
    return malloc(size);
}
```

## Namespace-Level Annotations

Apply annotations to entire namespaces:

```cpp
// @safe
namespace data_processing {
    // All functions in this namespace are checked
    
    // @lifetime: (&'a) -> owned
    Result process(const Input& in) {
        // ...
    }
    
    // @lifetime: (&'a, &'b) -> void
    void transform(const Source& src, Target& tgt) {
        // ...
    }
    
    // @unsafe  // Override namespace-level safety
    void unsafeOptimization() {
        // This specific function is unsafe
    }
}
```

## Practical Examples

### Example 1: String Processing

```cpp
// @safe
class StringProcessor {
    std::string buffer;
    
public:
    // @lifetime: (&'a) -> &'a
    const std::string& getBuffer() const {
        return buffer;
    }
    
    // @lifetime: (&'a mut, const string&) -> void
    void setBuffer(const std::string& s) {
        buffer = s;
    }
    
    // @lifetime: (&'a, const char*) -> size_t
    size_t findSubstring(const char* substr) const {
        return buffer.find(substr);
    }
    
    // @lifetime: (&'a) -> owned
    std::string extractUpper() const {
        std::string result;
        for (char c : buffer) {
            result += std::toupper(c);
        }
        return result;
    }
};
```

### Example 2: Container with Lifetime Relationships

```cpp
// @safe
template<typename T>
class SafeContainer {
    std::vector<T> items;
    T* cached_ptr = nullptr;
    
public:
    // @lifetime: (&'a mut, T) -> void
    void add(T item) {
        items.push_back(std::move(item));
        cached_ptr = nullptr;  // Invalidate cache
    }
    
    // @lifetime: (&'a) -> &'a
    const T& first() const {
        return items.front();
    }
    
    // @lifetime: (&'a) -> &'a
    const T& last() const {
        return items.back();
    }
    
    // @lifetime: (&'a mut) -> &'a mut
    T& getCached() {
        if (!cached_ptr && !items.empty()) {
            cached_ptr = &items[0];
        }
        return *cached_ptr;
    }
};
```

### Example 3: Database Connection

```cpp
// @safe
class DatabaseConnection {
    sqlite3* db;
    
public:
    // @lifetime: owned
    static DatabaseConnection open(const std::string& path) {
        DatabaseConnection conn;
        sqlite3_open(path.c_str(), &conn.db);
        return conn;
    }
    
    // @lifetime: (&'a, const string&) -> owned
    QueryResult execute(const std::string& query) {
        // Returns owned result
        return QueryResult(/* ... */);
    }
    
    // @lifetime: (&'a) -> &'a
    const sqlite3* getHandle() const {
        return db;
    }
    
    // @unsafe
    // @lifetime: (&'a mut) -> *mut
    sqlite3* getMutableHandle() {
        // Unsafe: returns raw pointer
        return db;
    }
};
```

## Best Practices

### 1. Start Simple

Begin with basic annotations and add complexity as needed:

```cpp
// Start with this:
// @lifetime: (&'a) -> &'a

// Later add constraints if needed:
// @lifetime: (&'a, &'b) -> &'a where 'a: 'b
```

### 2. Be Explicit About Ownership

Always clarify whether functions transfer ownership:

```cpp
// Clear ownership transfer
// @lifetime: owned
std::unique_ptr<T> create();

// Clear borrowing
// @lifetime: (&'a) -> &'a
const T& borrow(const Container& c);
```

### 3. Document Lifetime Relationships

Use meaningful lifetime names when relationships are complex:

```cpp
// @lifetime: (&'container, &'element) -> &'container where 'container: 'element
const Container& storeElement(const Container& c, const Element& e);
```

### 4. Combine with External Annotations

Use in-place annotations for your code and external annotations for third-party:

```cpp
// Your code with in-place annotation
// @safe
// @lifetime: (&'a) -> &'a
const Data& processData(const Data& input) {
    // Use third-party function (annotated externally)
    third_party::validate(input);  // Checked via external annotations
    return input;
}
```

## Limitations

1. **Parser Limitations**: Complex template syntax may not be fully supported
2. **Inference**: Not all lifetimes can be automatically inferred
3. **Macros**: Annotations in macros may not be recognized
4. **Virtual Functions**: Lifetime polymorphism is limited

## Troubleshooting

### Common Issues

1. **Annotation Not Recognized**
   - Ensure comment is directly above function
   - Check syntax is exactly as specified
   - No extra spaces in lifetime specification

2. **Lifetime Too Restrictive**
   - Consider if return really needs same lifetime as parameter
   - Use `owned` for new objects

3. **Missing Constraints**
   - Add where clauses for outlives relationships
   - Ensure transitive relationships are specified

## Future Enhancements

- Automatic lifetime inference for simple cases
- Support for lifetime elision rules
- Better template support
- IDE integration with quick fixes
- Lifetime visualization tools