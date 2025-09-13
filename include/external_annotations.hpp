// External Annotations for Third-Party Code (Simplified)
//
// This header provides unified annotations for third-party functions
// combining both safety and lifetime information.
//
// Usage:
//   #include <external_annotations.hpp>
//   #include <third_party_library.h>
//   // Your code with safety and lifetime checking

#ifndef RUSTYCPP_EXTERNAL_ANNOTATIONS_HPP
#define RUSTYCPP_EXTERNAL_ANNOTATIONS_HPP

// ============================================================================
// Marking Entire Classes/Namespaces as Unsafe
// ============================================================================

// Mark entire namespaces or classes as unsafe (all functions within)
// @external_unsafe: legacy::*
// @external_unsafe: OldCStyleAPI::*
// @external_unsafe: vendor::internal::*

// Multiple scopes can be marked at once
// @external_unsafe: {
//   scopes: [
//     "legacy::*",
//     "vendor::unsafe::*",
//     "deprecated::*",
//     "LowLevelDriver::*"
//   ]
// }

// ============================================================================
// Unified Function Annotations (Safety + Lifetime)
// ============================================================================

// All external functions use unified syntax: [safety, lifetime_spec]
// @external: {
//   function_name: [safety, lifetime_specification]
// }

// ============================================================================
// C Standard Library
// ============================================================================

// @external: {
//   // String functions
//   strlen: [safe, (const char* str) -> size_t]
//   strcmp: [safe, (const char* s1, const char* s2) -> int]
//   strchr: [safe, (const char* str, int c) -> const char* where str: 'a, return: 'a]
//   strstr: [safe, (const char* str, const char* needle) -> const char* where str: 'a, return: 'a]
//   strcpy: [unsafe, (char* dest, const char* src) -> char* where dest: 'a, return: 'a]
//   strncpy: [safe, (char* dest, const char* src, size_t n) -> char* where dest: 'a, return: 'a]
//   strcat: [unsafe, (char* dest, const char* src) -> char* where dest: 'a, return: 'a]
//   strdup: [unsafe, (const char* str) -> owned char*]
//   
//   // Memory functions
//   malloc: [unsafe, (size_t size) -> owned void*]
//   calloc: [unsafe, (size_t n, size_t size) -> owned void*]
//   realloc: [unsafe, (void* ptr, size_t size) -> owned void*]
//   free: [unsafe, (void* ptr) -> void]
//   memcpy: [unsafe, (void* dest, const void* src, size_t n) -> void* where dest: 'a, return: 'a]
//   memmove: [unsafe, (void* dest, const void* src, size_t n) -> void* where dest: 'a, return: 'a]
//   memset: [unsafe, (void* ptr, int value, size_t n) -> void* where ptr: 'a, return: 'a]
//   memcmp: [safe, (const void* s1, const void* s2, size_t n) -> int]
//   
//   // I/O functions
//   printf: [safe, (const char* format, ...) -> int]
//   fprintf: [safe, (FILE* file, const char* format, ...) -> int]
//   sprintf: [unsafe, (char* buffer, const char* format, ...) -> int]
//   snprintf: [safe, (char* buffer, size_t size, const char* format, ...) -> int]
//   fopen: [unsafe, (const char* path, const char* mode) -> owned FILE*]
//   fclose: [unsafe, (FILE* file) -> int]
//   fread: [unsafe, (void* buffer, size_t size, size_t count, FILE* file) -> size_t]
//   fwrite: [unsafe, (const void* buffer, size_t size, size_t count, FILE* file) -> size_t]
//   fgets: [safe, (char* buffer, int size, FILE* file) -> char* where buffer: 'a, return: 'a]
//   fputs: [safe, (const char* str, FILE* file) -> int]
//   
//   // Other safe functions
//   atoi: [safe, (const char* str) -> int]
//   atof: [safe, (const char* str) -> double]
//   exit: [safe, (int status) -> void]
//   abort: [safe, () -> void]
// }

// ============================================================================
// POSIX Functions
// ============================================================================

// @external: {
//   // File operations
//   open: [unsafe, (const char* path, int flags, ...) -> int]
//   close: [unsafe, (int fd) -> int]
//   read: [unsafe, (int fd, void* buffer, size_t count) -> ssize_t]
//   write: [unsafe, (int fd, const void* buffer, size_t count) -> ssize_t]
//   lseek: [safe, (int fd, off_t offset, int whence) -> off_t]
//   
//   // Directory operations
//   mkdir: [safe, (const char* path, mode_t mode) -> int]
//   rmdir: [safe, (const char* path) -> int]
//   opendir: [unsafe, (const char* path) -> owned DIR*]
//   readdir: [unsafe, (DIR* dir) -> struct dirent* where dir: 'a, return: 'a]
//   closedir: [unsafe, (DIR* dir) -> int]
//   
//   // Process operations
//   fork: [unsafe, () -> pid_t]
//   wait: [safe, (int* status) -> pid_t]
//   waitpid: [safe, (pid_t pid, int* status, int options) -> pid_t]
//   getpid: [safe, () -> pid_t]
//   getppid: [safe, () -> pid_t]
// }

// ============================================================================
// Common Third-Party Libraries
// ============================================================================

// Boost
// @external: {
//   boost::lexical_cast: [safe, template<T, S>(const S& arg) -> owned T]
//   boost::format: [safe, (const string& fmt) -> owned format]
//   boost::split: [safe, (vector<string>& result, const string& input, Pred pred) -> void]
//   boost::filesystem::exists: [safe, (const path& p) -> bool]
//   boost::filesystem::canonical: [safe, (const path& p) -> owned path]
//   boost::shared_ptr::get: [safe, () -> T* where this: 'a, return: 'a]
//   boost::unique_ptr::get: [safe, () -> T* where this: 'a, return: 'a]
//   boost::unique_ptr::release: [unsafe, () -> owned T*]
// }

// SQLite3
// @external: {
//   sqlite3_open: [unsafe, (const char* filename, sqlite3** db) -> int]
//   sqlite3_close: [unsafe, (sqlite3* db) -> int]
//   sqlite3_prepare_v2: [safe, (sqlite3* db, const char* sql, int nbyte, sqlite3_stmt** stmt, const char** tail) -> int]
//   sqlite3_bind_text: [safe, (sqlite3_stmt* stmt, int idx, const char* text, int nbyte, void(*)(void*)) -> int]
//   sqlite3_bind_int: [safe, (sqlite3_stmt* stmt, int idx, int value) -> int]
//   sqlite3_step: [safe, (sqlite3_stmt* stmt) -> int]
//   sqlite3_column_text: [safe, (sqlite3_stmt* stmt, int col) -> const unsigned char* where stmt: 'a, return: 'a]
//   sqlite3_column_int: [safe, (sqlite3_stmt* stmt, int col) -> int]
//   sqlite3_finalize: [unsafe, (sqlite3_stmt* stmt) -> int]
//   sqlite3_errmsg: [safe, (sqlite3* db) -> const char* where db: 'a, return: 'a]
// }

// JSON (nlohmann/json)
// @external: {
//   nlohmann::json::parse: [safe, (const string& s) -> owned json]
//   nlohmann::json::dump: [safe, (int indent) -> owned string]
//   nlohmann::json::operator[]: [safe, (const string& key) -> json& where this: 'a, return: 'a]
//   nlohmann::json::at: [safe, (const string& key) -> json& where this: 'a, return: 'a]
//   nlohmann::json::get: [safe, template<T>() -> owned T]
//   nlohmann::json::get_ref: [safe, template<T>() -> T& where this: 'a, return: 'a]
// }

// OpenSSL
// @external: {
//   EVP_MD_CTX_new: [unsafe, () -> owned EVP_MD_CTX*]
//   EVP_MD_CTX_free: [unsafe, (EVP_MD_CTX* ctx) -> void]
//   SHA256: [safe, (const unsigned char* data, size_t count, unsigned char* md) -> unsigned char* where md: 'a, return: 'a]
//   SHA512: [safe, (const unsigned char* data, size_t count, unsigned char* md) -> unsigned char* where md: 'a, return: 'a]
// }

// ============================================================================
// Pattern-Based Annotations
// ============================================================================

// Whitelist safe patterns
// @external_whitelist: {
//   patterns: [
//     "std::*",           // STL handled separately
//     "rusty::*",         // Our own library
//     "*::size",          // Size getters
//     "*::length",        // Length getters
//     "*::empty",         // Empty checks
//     "*::capacity",      // Capacity getters
//   ]
// }

// Blacklist unsafe patterns
// @external_blacklist: {
//   patterns: [
//     "*::operator new*",    // Manual allocation
//     "*::operator delete*", // Manual deallocation
//     "*::malloc",           // C allocation
//     "*::calloc",           // C allocation
//     "*::realloc",          // C reallocation
//     "*::free",             // C deallocation
//     "asm*",                // Inline assembly
//     "__builtin_*",         // Compiler builtins
//   ]
// }

// ============================================================================
// Unsafe Scopes Examples
// ============================================================================

// Example: Mark entire legacy namespace as unsafe
// @external_unsafe: legacy::*

// Example: Mark old C-style class as unsafe
// @external_unsafe: OldFileHandler::*

// Example: Mark multiple scopes
// @external_unsafe: {
//   scopes: [
//     "deprecated::*",
//     "internal::lowlevel::*",
//     "vendor::proprietary::*"
//   ]
// }

// ============================================================================
// Library Profiles
// ============================================================================

// Qt Framework
// @external_profile: qt {
//   annotations: {
//     QObject::parent: [safe, () -> QObject* where this: 'a, return: 'a]
//     QObject::children: [safe, () -> const QObjectList& where this: 'a, return: 'a]
//     QObject::connect: [unsafe, (const QObject* sender, const char* signal, const QObject* receiver, const char* method) -> QMetaObject::Connection]
//     QString::c_str: [safe, () -> const char* where this: 'a, return: 'a]
//     QString::toStdString: [safe, () -> owned std::string]
//   }
// }

// Embedded Systems
// @external_profile: embedded {
//   annotations: {
//     read_register: [safe, (uint32_t addr) -> uint32_t]
//     write_register: [safe, (uint32_t addr, uint32_t value) -> void]
//     get_peripheral: [unsafe, (uint32_t base) -> Peripheral* where return: 'static]
//     dma_transfer: [unsafe, (void* src, void* dest, size_t size) -> int]
//   }
// }

// ============================================================================
// Usage in Your Code
// ============================================================================

/*
// Example 1: Using with unsafe scope
namespace myapp {
    // @safe
    void process() {
        // legacy::old_function();  // ERROR: legacy::* is marked unsafe
        
        // Can still use in unsafe context
        // @unsafe
        {
            legacy::old_function();  // OK in unsafe block
        }
    }
}

// Example 2: Using unified annotations
// @safe
void example() {
    const char* str = "hello";
    const char* found = strchr(str, 'e');  // OK: safe with lifetime
    
    // void* buf = malloc(100);  // ERROR: malloc is unsafe
    
    // @unsafe
    {
        void* buf = malloc(100);  // OK in unsafe
        free(buf);                // OK in unsafe
    }
}
*/

#endif // RUSTYCPP_EXTERNAL_ANNOTATIONS_HPP