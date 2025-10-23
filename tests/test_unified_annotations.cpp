// Test unified external annotations with safety + lifetime
#include "../include/unified_external_annotations.hpp"
#include <cstring>
#include <memory>
#include <vector>

// Mock third-party library with complex lifetime relationships
namespace third_party {
    // Function that returns a reference with same lifetime as input
    const char* find_substring(const char* haystack, const char* needle) {
        return strstr(haystack, needle);
    }
    
    // Function that transfers ownership
    char* clone_string(const char* src) {
        return strdup(src);
    }
    
    // Function that borrows and returns borrowed reference
    const int& get_element(const std::vector<int>& vec, size_t index) {
        return vec[index];
    }
    
    // Function with lifetime constraints
    const char* select_longer(const char* a, const char* b) {
        return strlen(a) > strlen(b) ? a : b;
    }
}

// Unified annotations for the mock library
// @external: {
//   third_party::find_substring: [safe, (const char* haystack, const char* needle) -> const char* where haystack: 'a, return: 'a]
//   third_party::clone_string: [unsafe, (const char* src) -> owned char*]
//   third_party::get_element: [safe, (const vector<int>& vec, size_t index) -> const int& where vec: 'a, return: 'a]
//   third_party::select_longer: [safe, (const char* a, const char* b) -> const char* where a: 'a, b: 'b, return: 'a, 'a: 'b]
// }

// @safe
namespace test_basic_unified {
    void test_safe_with_lifetime() {
        const char* text = "Hello, world!";
        const char* pattern = "world";
        
        // Safe function with lifetime relationship
        const char* found = third_party::find_substring(text, pattern);
        // 'found' has lifetime of 'text'
        
        if (found) {
            printf("Found: %s\n", found);  // OK - printf is safe
        }
    }
    
    // @unsafe
    void test_unsafe_with_ownership() {
        const char* original = "test string";
        
        // Unsafe function that transfers ownership
        char* cloned = third_party::clone_string(original);  // OK in unsafe
        
        // We own the cloned string and must free it
        printf("Cloned: %s\n", cloned);
        free(cloned);  // OK in unsafe - matches ownership
    }
    
    void test_lifetime_constraint() {
        std::vector<int> numbers = {1, 2, 3, 4, 5};
        
        // Safe function with lifetime constraint
        const int& elem = third_party::get_element(numbers, 2);
        // 'elem' has lifetime of 'numbers'
        
        // Cannot modify 'numbers' while 'elem' exists
        // numbers.push_back(6);  // ERROR: would invalidate elem
        
        int value = elem;  // OK - copy value
        numbers.push_back(6);  // OK after elem no longer used
    }
}

// Test C standard library with unified annotations
// @safe
namespace test_c_stdlib_unified {
    void test_string_functions() {
        const char* str = "Hello, world!";
        
        // strchr returns pointer with same lifetime as input
        const char* ch = strchr(str, 'o');  // Safe, lifetime: str: 'a, return: 'a
        
        if (ch) {
            // 'ch' is valid as long as 'str' is valid
            printf("Found: %c\n", *ch);
        }
    }
    
    // @unsafe
    void test_memory_functions() {
        // malloc returns owned pointer
        void* buffer = malloc(100);  // Unsafe, lifetime: owned
        
        // memset returns same pointer with same lifetime
        void* initialized = memset(buffer, 0, 100);  // Unsafe, lifetime: buffer: 'a, return: 'a
        
        // free consumes owned pointer
        free(initialized);  // Unsafe, consumes ownership
    }
    
    // @unsafe
    void test_string_copy() {
        char dest[100];
        const char* src = "source";
        
        // strcpy returns dest with same lifetime
        char* result = strcpy(dest, src);  // Unsafe, lifetime: dest: 'a, return: 'a
        
        // 'result' and 'dest' are the same, with same lifetime
        printf("Copied: %s\n", result);
    }
}

// Test database functions with unified annotations
// @external: {
//   sqlite3_column_text: [safe, (sqlite3_stmt* stmt, int col) -> const unsigned char* where stmt: 'a, return: 'a]
//   sqlite3_errmsg: [safe, (sqlite3* db) -> const char* where db: 'a, return: 'a]
// }

// @safe
namespace test_database_unified {
    struct MockStmt { int dummy; };
    struct MockDB { int dummy; };
    
    // Mock functions for testing
    const unsigned char* sqlite3_column_text(MockStmt* stmt, int col) {
        static unsigned char text[] = "result";
        return text;
    }
    
    const char* sqlite3_errmsg(MockDB* db) {
        return "No error";
    }
    
    void test_sqlite_lifetimes() {
        MockStmt stmt;
        MockDB db;
        
        // Column text has lifetime of statement
        const unsigned char* text = sqlite3_column_text(&stmt, 0);
        // 'text' valid as long as 'stmt' exists
        
        // Error message has lifetime of database
        const char* error = sqlite3_errmsg(&db);
        // 'error' valid as long as 'db' exists
        
        printf("Text: %s, Error: %s\n", text, error);
    }
}

// Test complex lifetime relationships
// @external_function: complex_lifetime {
//   safety: safe
//   lifetime: (const T& container, const K& key) -> const V&
//   where: container: 'a, key: 'b, return: 'a, 'b: 'a
// }

// @safe
namespace test_complex_lifetimes {
    template<typename K, typename V>
    struct Container {
        std::vector<std::pair<K, V>> items;
        
        const V& lookup(const K& key) const {
            for (const auto& [k, v] : items) {
                if (k == key) return v;
            }
            static V default_value{};
            return default_value;
        }
    };
    
    void test_container_lifetime() {
        Container<int, std::string> cont;
        cont.items = {{1, "one"}, {2, "two"}};
        
        int key = 1;
        const std::string& value = cont.lookup(key);
        // 'value' has lifetime of 'cont', not 'key'
        // This matches the annotation: return: 'a where container: 'a
        
        // Can destroy key, value still valid
        key = 2;  // OK
        
        // Cannot destroy cont while value exists
        // cont = Container<int, std::string>();  // ERROR: would invalidate value
        
        printf("Value: %s\n", value.c_str());
    }
}

// Test JSON library with complete annotations
// @external: {
//   json::parse: [safe, (const string& s) -> owned json]
//   json::operator[]: [safe, (const string& key) -> json& where this: 'a, return: 'a]
//   json::get_ref: [safe, template<T>() -> T& where this: 'a, return: 'a]
// }

// @safe
namespace test_json_unified {
    // Mock JSON class
    class json {
    public:
        static json parse(const std::string& s) { return json(); }
        json& operator[](const std::string& key) { return *this; }
        template<typename T> T& get_ref() { static T t; return t; }
    };
    
    void test_json_lifetimes() {
        std::string json_str = "{\"key\": \"value\"}";
        
        // parse returns owned object
        json obj = json::parse(json_str);  // Safe, owned
        
        // operator[] returns reference with object's lifetime
        json& field = obj["key"];  // Safe, lifetime tied to obj
        
        // get_ref returns reference with object's lifetime
        std::string& str_ref = obj.get_ref<std::string>();  // Safe, lifetime tied to obj
        
        // Cannot destroy obj while references exist
        // obj = json();  // ERROR: would invalidate field and str_ref
    }
}

int main() {
    // Test basic unified annotations
    test_basic_unified::test_safe_with_lifetime();
    test_basic_unified::test_lifetime_constraint();
    
    // Test C stdlib unified
    test_c_stdlib_unified::test_string_functions();
    
    // Test database unified
    test_database_unified::test_sqlite_lifetimes();
    
    // Test complex lifetimes
    test_complex_lifetimes::test_container_lifetime();
    
    // Test JSON unified
    test_json_unified::test_json_lifetimes();
    
    return 0;
}