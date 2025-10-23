// Test simplified external annotation system
// - Unified syntax only (no separate safety/lifetime)
// - Scope-level unsafe marking

#include "../include/external_annotations.hpp"
#include <cstring>
#include <memory>

// Mock legacy namespace (entire namespace marked unsafe)
namespace legacy {
    int old_function(int x) { return x * 2; }
    void* old_malloc(size_t size) { return malloc(size); }
    void old_free(void* ptr) { free(ptr); }
    char* old_strcpy(char* dest, const char* src) { return strcpy(dest, src); }
}

// Mock vendor library with internal unsafe parts
namespace vendor {
    namespace safe_api {
        int process(int x) { return x + 1; }
        bool validate(int x) { return x > 0; }
    }
    
    namespace internal {
        void* allocate(size_t size) { return malloc(size); }
        void deallocate(void* ptr) { free(ptr); }
        void low_level_operation() { }
    }
}

// Mock old C-style class
class OldFileHandler {
public:
    static FILE* open(const char* path) { return fopen(path, "r"); }
    static void close(FILE* f) { fclose(f); }
    static size_t read(void* buf, size_t size, FILE* f) { return fread(buf, 1, size, f); }
    static size_t write(const void* buf, size_t size, FILE* f) { return fwrite(buf, 1, size, f); }
};

// External annotations for this test

// Mark entire legacy namespace as unsafe
// @external_unsafe: legacy::*

// Mark vendor internal namespace as unsafe
// @external_unsafe: vendor::internal::*

// Mark old C-style class as unsafe
// @external_unsafe: OldFileHandler::*

// Unified annotations for specific functions
// @external: {
//   vendor::safe_api::process: [safe, (int x) -> int]
//   vendor::safe_api::validate: [safe, (int x) -> bool]
// }

// Standard library annotations (unified only)
// @external: {
//   strlen: [safe, (const char* str) -> size_t]
//   strcmp: [safe, (const char* s1, const char* s2) -> int]
//   strchr: [safe, (const char* str, int c) -> const char* where str: 'a, return: 'a]
//   strcpy: [unsafe, (char* dest, const char* src) -> char* where dest: 'a, return: 'a]
//   malloc: [unsafe, (size_t size) -> owned void*]
//   free: [unsafe, (void* ptr) -> void]
// }

// =============================================================================
// Test Cases
// =============================================================================

// @safe
namespace test_unsafe_scopes {
    
    void test_legacy_namespace() {
        // All legacy:: functions are unsafe (entire namespace marked)
        
        // ERROR: Cannot call any legacy function in safe context
        // int result = legacy::old_function(42);
        // void* ptr = legacy::old_malloc(100);
        // legacy::old_free(ptr);
        
        // Must use unsafe context
        // @unsafe
        {
            int result = legacy::old_function(42);  // OK in unsafe
            void* ptr = legacy::old_malloc(100);    // OK in unsafe
            legacy::old_free(ptr);                  // OK in unsafe
        }
    }
    
    void test_vendor_namespaces() {
        // vendor::safe_api functions are explicitly marked safe
        int result = vendor::safe_api::process(42);  // OK: explicitly safe
        bool valid = vendor::safe_api::validate(10);  // OK: explicitly safe
        
        // vendor::internal functions are all unsafe (namespace marked)
        // void* ptr = vendor::internal::allocate(100);  // ERROR: unsafe namespace
        // vendor::internal::low_level_operation();      // ERROR: unsafe namespace
        
        // @unsafe
        {
            void* ptr = vendor::internal::allocate(100);  // OK in unsafe
            vendor::internal::deallocate(ptr);            // OK in unsafe
            vendor::internal::low_level_operation();      // OK in unsafe
        }
    }
    
    void test_unsafe_class() {
        // OldFileHandler class is entirely unsafe
        
        // ERROR: All methods are unsafe
        // FILE* f = OldFileHandler::open("test.txt");
        // OldFileHandler::close(f);
        
        // @unsafe
        {
            FILE* f = OldFileHandler::open("test.txt");  // OK in unsafe
            if (f) {
                char buffer[100];
                size_t bytes = OldFileHandler::read(buffer, 100, f);  // OK in unsafe
                OldFileHandler::close(f);  // OK in unsafe
            }
        }
    }
}

// @safe
namespace test_unified_annotations {
    
    void test_safe_with_lifetime() {
        const char* text = "Hello, world!";
        
        // Safe functions with lifetime checking
        size_t len = strlen(text);  // OK: safe, no lifetime
        
        const char* found = strchr(text, 'o');  // OK: safe with lifetime
        // 'found' has lifetime tied to 'text'
        
        if (found) {
            printf("Found at position: %ld\n", found - text);
        }
    }
    
    void test_unsafe_functions() {
        // Unsafe functions require unsafe context
        
        // ERROR: malloc is unsafe
        // void* buffer = malloc(100);
        
        // ERROR: strcpy is unsafe
        // char dest[100];
        // strcpy(dest, "test");
        
        // @unsafe
        {
            void* buffer = malloc(100);  // OK in unsafe
            
            char dest[100];
            strcpy(dest, "test");  // OK in unsafe
            
            free(buffer);  // OK in unsafe
        }
    }
    
    void test_lifetime_relationships() {
        char buffer[100] = "original";
        const char* source = "modified";
        
        // @unsafe
        {
            // strcpy returns dest with same lifetime
            char* result = strcpy(buffer, source);
            // 'result' and 'buffer' have same address and lifetime
            
            printf("Result: %s\n", result);
        }
    }
}

// @safe
namespace test_pattern_matching {
    
    // Assuming these patterns are configured:
    // Whitelist: "*::size", "*::length", "*::empty"
    // Blacklist: "*::operator new*", "*::operator delete*"
    
    class Container {
    public:
        size_t size() const { return 10; }  // Would match *::size pattern
        size_t length() const { return 10; }  // Would match *::length pattern
        bool empty() const { return false; }  // Would match *::empty pattern
        
        void* operator new(size_t size) { return malloc(size); }  // Matches blacklist
        void operator delete(void* ptr) { free(ptr); }  // Matches blacklist
    };
    
    void test_patterns() {
        Container c;
        
        // These would be safe if patterns are active
        size_t s = c.size();    // OK: matches safe pattern
        size_t l = c.length();  // OK: matches safe pattern
        bool e = c.empty();     // OK: matches safe pattern
        
        // These would be unsafe due to blacklist
        // Container* ptr = new Container();  // ERROR: operator new is blacklisted
        // delete ptr;  // ERROR: operator delete is blacklisted
        
        // @unsafe
        {
            Container* ptr = new Container();  // OK in unsafe
            delete ptr;  // OK in unsafe
        }
    }
}

// @safe
namespace test_real_world {
    
    // Simulating a real scenario with mixed safe/unsafe code
    
    class DataProcessor {
        std::vector<int> data;
        legacy::OldFileHandler* file_handler;  // Using unsafe legacy code
        
    public:
        // Safe public API
        void add_data(int value) {
            data.push_back(value);
        }
        
        size_t get_size() const {
            return data.size();
        }
        
        // This method needs to interact with unsafe legacy code
        void save_to_file(const char* path) {
            // @unsafe
            {
                FILE* f = OldFileHandler::open(path);
                if (f) {
                    for (int val : data) {
                        OldFileHandler::write(&val, sizeof(int), f);
                    }
                    OldFileHandler::close(f);
                }
            }
        }
        
        // This method uses both safe and unsafe operations
        void process_with_legacy() {
            // Safe operations
            int count = vendor::safe_api::process(data.size());
            bool valid = vendor::safe_api::validate(count);
            
            if (valid) {
                // Unsafe operations need unsafe block
                // @unsafe
                {
                    int result = legacy::old_function(count);
                    void* temp = legacy::old_malloc(result * sizeof(int));
                    // ... do something with temp ...
                    legacy::old_free(temp);
                }
            }
        }
    };
}

int main() {
    // Test unsafe scopes
    test_unsafe_scopes::test_legacy_namespace();
    test_unsafe_scopes::test_vendor_namespaces();
    test_unsafe_scopes::test_unsafe_class();
    
    // Test unified annotations
    test_unified_annotations::test_safe_with_lifetime();
    test_unified_annotations::test_lifetime_relationships();
    
    // Test patterns
    test_pattern_matching::test_patterns();
    
    printf("All tests completed\n");
    return 0;
}