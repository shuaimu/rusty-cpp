// Test external annotations for third-party functions
#include "../include/external_annotations.hpp"
#include <cstdio>
#include <cstring>
#include <memory>
#include <vector>
#include <fcntl.h>      // For open()
#include <sys/stat.h>   // For mkdir()
#include <unistd.h>     // For close()

// Mock third-party library functions
namespace third_party {
    int process_data(const void* data, size_t size) {
        // Simulated third-party function
        return 0;
    }
    
    void* allocate_buffer(size_t size) {
        return malloc(size);
    }
    
    void deallocate_buffer(void* ptr) {
        free(ptr);
    }
    
    const char* get_version() {
        return "1.0.0";
    }
}

namespace legacy {
    // Legacy C-style API
    char* unsafe_string_copy(char* dest, const char* src) {
        return strcpy(dest, src);  // Unsafe!
    }
    
    int safe_string_compare(const char* s1, const char* s2) {
        return strcmp(s1, s2);  // Safe
    }
}

// Add external annotations for our mock functions
// @external_safety: {
//   third_party::process_data: safe
//   third_party::allocate_buffer: unsafe
//   third_party::deallocate_buffer: unsafe
//   third_party::get_version: safe
//   legacy::unsafe_string_copy: unsafe
//   legacy::safe_string_compare: safe
// }

// @safe
namespace test_standard_library {
    void test_safe_c_functions() {
        // These C standard library functions are marked safe
        printf("Hello, world!\n");  // OK - printf is safe
        
        const char* str = "test";
        size_t len = strlen(str);  // OK - strlen is safe
        
        int value = atoi("42");  // OK - atoi is safe
        
        // String comparison is safe
        int result = strcmp("a", "b");  // OK - strcmp is safe
    }
    
    // @unsafe
    void test_unsafe_c_functions() {
        // These require unsafe context
        void* ptr = malloc(100);  // OK in unsafe - malloc is unsafe
        memcpy(ptr, "data", 4);   // OK in unsafe - memcpy is unsafe
        free(ptr);                // OK in unsafe - free is unsafe
        
        char buffer[100];
        strcpy(buffer, "test");   // OK in unsafe - strcpy is unsafe
    }
    
    void test_unsafe_without_annotation() {
        // These should error in safe context
        // void* ptr = malloc(100);  // ERROR: malloc is unsafe
        // free(ptr);                // ERROR: free is unsafe
        
        // char buffer[100];
        // strcpy(buffer, "test");   // ERROR: strcpy is unsafe
    }
}

// @safe
namespace test_third_party {
    void test_safe_third_party() {
        // Functions marked as safe in external annotations
        const char* version = third_party::get_version();  // OK - marked safe
        
        std::vector<char> data = {'a', 'b', 'c'};
        int result = third_party::process_data(data.data(), data.size());  // OK - marked safe
    }
    
    // @unsafe
    void test_unsafe_third_party() {
        // Functions marked as unsafe in external annotations
        void* buffer = third_party::allocate_buffer(1024);  // OK in unsafe
        // ... use buffer ...
        third_party::deallocate_buffer(buffer);  // OK in unsafe
    }
    
    void test_unsafe_third_party_without_annotation() {
        // These should error in safe context
        // void* buffer = third_party::allocate_buffer(1024);  // ERROR: unsafe
        // third_party::deallocate_buffer(buffer);             // ERROR: unsafe
    }
}

// @safe
namespace test_legacy {
    void test_legacy_api() {
        // Safe legacy functions
        int cmp = legacy::safe_string_compare("a", "b");  // OK - marked safe
        
        // Unsafe legacy functions should error
        // char dest[100];
        // legacy::unsafe_string_copy(dest, "source");  // ERROR: unsafe
    }
    
    // @unsafe
    void test_unsafe_legacy() {
        char dest[100];
        legacy::unsafe_string_copy(dest, "source");  // OK in unsafe context
    }
}

// @safe
namespace test_patterns {
    // Test pattern matching for external annotations
    
    void test_std_patterns() {
        // std::* pattern should make all std:: functions safe
        std::vector<int> vec;
        vec.push_back(42);  // OK - std::* is whitelisted
        
        std::unique_ptr<int> ptr = std::make_unique<int>(42);  // OK
    }
    
    void test_blacklist_patterns() {
        // These match blacklist patterns and should error
        // operator new is blacklisted
        // int* ptr = new int(42);  // ERROR: operator new is unsafe
        // delete ptr;              // ERROR: operator delete is unsafe
    }
}

// @safe
namespace test_posix {
    // @unsafe
    void test_posix_unsafe() {
        // POSIX functions marked as unsafe
        int fd = open("/tmp/test", 0);  // OK in unsafe - open is unsafe
        char buffer[100];
        read(fd, buffer, 100);  // OK in unsafe - read is unsafe
        write(fd, "data", 4);   // OK in unsafe - write is unsafe
        close(fd);              // OK in unsafe - close is unsafe
    }
    
    void test_posix_safe() {
        // Some POSIX functions are safe
        mkdir("/tmp/testdir", 0755);  // OK - mkdir is safe
        rmdir("/tmp/testdir");        // OK - rmdir is safe
        rename("/tmp/a", "/tmp/b");   // OK - rename is safe
    }
}

// Test with profiles
// @external_profile: myproject {
//   safe: [
//     "mylib::*",
//     "helper::*"
//   ]
//   unsafe: [
//     "*::internal_*",
//     "*::unsafe_*"
//   ]
// }

namespace mylib {
    void public_api() {}
    void internal_api() {}
}

namespace helper {
    void utility() {}
}

// @safe
namespace test_profiles {
    // Assuming 'myproject' profile is active
    
    void test_profile_safe() {
        mylib::public_api();  // OK - mylib::* is safe in profile
        helper::utility();     // OK - helper::* is safe in profile
    }
    
    void test_profile_unsafe() {
        // mylib::internal_api();  // ERROR: matches *::internal_* pattern
    }
}

int main() {
    // Test standard library annotations
    test_standard_library::test_safe_c_functions();
    
    // Test third-party annotations
    test_third_party::test_safe_third_party();
    
    // Test legacy API annotations
    test_legacy::test_legacy_api();
    
    // Test pattern matching
    test_patterns::test_std_patterns();
    
    // Test POSIX annotations
    test_posix::test_posix_safe();
    
    // Test profiles
    test_profiles::test_profile_safe();
    
    return 0;
}