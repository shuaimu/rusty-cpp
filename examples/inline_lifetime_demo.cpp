// Demonstration of in-place lifetime annotations in RustyCpp
//
// This example shows how to annotate your C++ code directly with
// lifetime information without using external annotation files.

#include <string>
#include <vector>
#include <memory>
#include <iostream>

// =============================================================================
// Basic Lifetime Annotations
// =============================================================================

// @lifetime: (&'a) -> &'a
const std::string& identity(const std::string& s) {
    // Return has same lifetime as parameter
    return s;
}

// @lifetime: owned
std::string createString(const char* text) {
    // Returns a new owned string
    return std::string(text);
}

// @lifetime: (&'a, &'b) -> &'a where 'a: 'b
const std::string& selectFirst(const std::string& first, const std::string& second) {
    // Return has lifetime of 'first', which must outlive 'second'
    return first;
}

// =============================================================================
// Safety Annotations with Lifetimes
// =============================================================================

// @safe
namespace safe_operations {
    
    // @lifetime: (&'a) -> size_t
    size_t getLength(const std::string& s) {
        // Safe function that borrows and returns primitive
        return s.length();
    }
    
    // @lifetime: (&'a mut) -> void
    void clearString(std::string& s) {
        // Safe function that mutably borrows
        s.clear();
    }
    
    // @lifetime: (&'a, size_t) -> &'a
    const char& getChar(const std::string& s, size_t index) {
        // Returns reference with same lifetime as string
        return s[index];
    }
}

// @unsafe
namespace unsafe_operations {
    
    // @lifetime: (size_t) -> owned void*
    void* allocateMemory(size_t size) {
        // Unsafe allocation with ownership transfer
        return malloc(size);
    }
    
    // @lifetime: (void*) -> void
    void freeMemory(void* ptr) {
        // Unsafe deallocation
        free(ptr);
    }
    
    // @lifetime: (void*, const void*, size_t) -> void*
    void* copyMemory(void* dest, const void* src, size_t n) {
        return memcpy(dest, src, n);
    }
}

// =============================================================================
// Class with Member Function Annotations
// =============================================================================

// @safe
class DataContainer {
private:
    std::vector<std::string> data;
    mutable std::string cache;
    
public:
    // @lifetime: (&'a mut, string) -> void
    void add(std::string item) {
        data.push_back(std::move(item));
        cache.clear();  // Invalidate cache
    }
    
    // @lifetime: (&'a, size_t) -> &'a
    const std::string& get(size_t index) const {
        return data.at(index);
    }
    
    // @lifetime: (&'a mut, size_t) -> &'a mut
    std::string& getMutable(size_t index) {
        cache.clear();  // Invalidate cache on mutable access
        return data.at(index);
    }
    
    // @lifetime: (&'a) -> size_t
    size_t size() const {
        return data.size();
    }
    
    // @lifetime: (&'a) -> &'a
    const std::string& getCached() const {
        if (cache.empty() && !data.empty()) {
            // Build cache from all data
            for (const auto& s : data) {
                cache += s + " ";
            }
        }
        return cache;
    }
    
    // @lifetime: (&'a) -> owned
    std::vector<std::string> clone() const {
        // Returns owned copy
        return data;
    }
    
    // @lifetime: (&'a mut) -> owned
    std::string remove(size_t index) {
        std::string result = std::move(data[index]);
        data.erase(data.begin() + index);
        cache.clear();
        return result;  // Transfer ownership
    }
};

// =============================================================================
// Template Functions with Lifetime Annotations
// =============================================================================

// @lifetime: (&'a, &'b) -> &'a where 'a: 'b
template<typename T>
const T& selectLarger(const T& a, const T& b) {
    return (a > b) ? a : b;
}

// @lifetime: (&'a) -> owned
template<typename T>
std::vector<T> duplicate(const std::vector<T>& vec) {
    std::vector<T> result = vec;
    result.insert(result.end(), vec.begin(), vec.end());
    return result;  // Return owned vector
}

// @lifetime: (&'a mut, const T&) -> void
template<typename T>
void appendToVector(std::vector<T>& vec, const T& item) {
    vec.push_back(item);
}

// =============================================================================
// Complex Lifetime Relationships
// =============================================================================

struct Node {
    std::string value;
    std::vector<Node*> children;
    
    // @lifetime: (&'a) -> &'a
    const std::string& getValue() const {
        return value;
    }
    
    // @lifetime: (&'a, size_t) -> *const
    const Node* getChild(size_t index) const {
        // Returns raw pointer - caller must ensure lifetime
        return (index < children.size()) ? children[index] : nullptr;
    }
};

// @safe
class Tree {
private:
    std::unique_ptr<Node> root;
    
public:
    // @lifetime: (&'a) -> *const
    const Node* getRoot() const {
        return root.get();
    }
    
    // @lifetime: (&'a, const Node*) -> &'a
    const std::string& getNodeValue(const Node* node) const {
        // Assumes node is valid and owned by this tree
        return node->getValue();
    }
    
    // @lifetime: owned
    static Tree create() {
        Tree t;
        t.root = std::make_unique<Node>();
        return t;
    }
};

// =============================================================================
// Real-World Example: String Buffer Manager
// =============================================================================

// @safe
class StringBufferManager {
private:
    struct Buffer {
        std::string data;
        bool in_use;
    };
    
    std::vector<Buffer> buffers;
    
public:
    // @lifetime: (&'a mut) -> size_t
    size_t allocateBuffer() {
        // Find unused buffer or create new one
        for (size_t i = 0; i < buffers.size(); ++i) {
            if (!buffers[i].in_use) {
                buffers[i].in_use = true;
                buffers[i].data.clear();
                return i;
            }
        }
        
        buffers.push_back({std::string(), true});
        return buffers.size() - 1;
    }
    
    // @lifetime: (&'a mut, size_t) -> void
    void releaseBuffer(size_t handle) {
        if (handle < buffers.size()) {
            buffers[handle].in_use = false;
        }
    }
    
    // @lifetime: (&'a, size_t) -> &'a
    const std::string& getBuffer(size_t handle) const {
        return buffers.at(handle).data;
    }
    
    // @lifetime: (&'a mut, size_t) -> &'a mut
    std::string& getMutableBuffer(size_t handle) {
        return buffers.at(handle).data;
    }
    
    // @lifetime: (&'a mut, size_t, const string&) -> void
    void writeToBuffer(size_t handle, const std::string& data) {
        buffers.at(handle).data = data;
    }
    
    // @lifetime: (&'a, size_t) -> owned
    std::string copyBuffer(size_t handle) const {
        return buffers.at(handle).data;  // Return owned copy
    }
};

// =============================================================================
// Example Usage with Lifetime Checking
// =============================================================================

// @safe
void demonstrateLifetimes() {
    // Basic lifetime relationships
    std::string s1 = "Hello";
    std::string s2 = "World";
    
    // identity: borrowed reference with same lifetime
    const std::string& ref = identity(s1);
    // ref has lifetime of s1
    
    // createString: returns owned value
    std::string owned = createString("New String");
    // owned is independent
    
    // selectFirst: complex lifetime relationship
    const std::string& selected = selectFirst(s1, s2);
    // selected has lifetime of s1, s1 must outlive s2 in this context
    
    // Using the container
    DataContainer container;
    container.add("First");
    container.add("Second");
    
    // Borrowing from container
    const std::string& item = container.get(0);
    // item has lifetime of container
    
    // Cannot modify container while reference exists
    // container.add("Third");  // ERROR: would invalidate item
    
    // Can use the reference
    std::cout << "Item: " << item << std::endl;
    
    // After reference goes out of scope, can modify again
    {
        const std::string& temp = container.get(1);
        std::cout << "Temp: " << temp << std::endl;
    }  // temp out of scope
    
    container.add("Third");  // OK now
    
    // Clone returns owned data
    std::vector<std::string> cloned = container.clone();
    // cloned is independent of container
    
    // Remove transfers ownership
    std::string removed = container.remove(0);
    // removed owns the data that was in container
}

// @safe
void demonstrateTemplates() {
    std::vector<int> vec1 = {1, 2, 3};
    std::vector<int> vec2 = {4, 5, 6};
    
    // Template with lifetime annotation
    const std::vector<int>& larger = selectLarger(vec1, vec2);
    // larger has lifetime tied to vec1 or vec2
    
    // duplicate returns owned copy
    std::vector<int> doubled = duplicate(vec1);
    // doubled is independent
    
    // Mutable borrow for append
    appendToVector(vec1, 7);
    // vec1 is mutably borrowed during call
}

// @safe
void demonstrateBufferManager() {
    StringBufferManager manager;
    
    // Allocate buffers
    size_t handle1 = manager.allocateBuffer();
    size_t handle2 = manager.allocateBuffer();
    
    // Write to buffers
    manager.writeToBuffer(handle1, "Buffer 1 data");
    manager.writeToBuffer(handle2, "Buffer 2 data");
    
    // Borrow buffer content
    const std::string& content = manager.getBuffer(handle1);
    // content has lifetime of manager
    
    // Cannot modify manager's structure while reference exists
    // manager.releaseBuffer(handle1);  // ERROR: would invalidate content
    
    // Can read the content
    std::cout << "Content: " << content << std::endl;
    
    // Copy returns owned string
    std::string copy = manager.copyBuffer(handle2);
    // copy is independent
    
    // Now can release buffers
    manager.releaseBuffer(handle1);
    manager.releaseBuffer(handle2);
}

int main() {
    std::cout << "=== In-Place Lifetime Annotations Demo ===" << std::endl;
    
    demonstrateLifetimes();
    demonstrateTemplates();
    demonstrateBufferManager();
    
    std::cout << "=== Demo Complete ===" << std::endl;
    return 0;
}