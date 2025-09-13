// Unified External Annotations for Safety and Lifetimes
//
// This header provides a unified way to annotate third-party functions
// with both safety information AND lifetime specifications.
//
// Usage:
//   #include <unified_external_annotations.hpp>
//   #include <third_party_library.h>
//   // Your code with full safety and lifetime checking
//
// The unified syntax allows you to specify everything about a function
// in one place, making the contract clear and maintainable.

#ifndef RUSTYCPP_UNIFIED_EXTERNAL_ANNOTATIONS_HPP
#define RUSTYCPP_UNIFIED_EXTERNAL_ANNOTATIONS_HPP

// ============================================================================
// Unified Annotation Syntax
// ============================================================================
//
// Functions can be annotated with both safety and lifetime information:
//
// @external_function: function_name {
//   safety: safe/unsafe
//   lifetime: (...) -> ...
//   where: lifetime constraints
// }
//
// Or using the compact syntax:
//
// @external: {
//   function_name: [safety, lifetime_spec]
// }

// ============================================================================
// C Standard Library - Complete Annotations
// ============================================================================

// String functions with lifetime relationships
// @external: {
//   strlen: [safe, (const char* str) -> owned]
//   strcpy: [unsafe, (char* dest, const char* src) -> char* where dest: 'a, return: 'a]
//   strncpy: [safe, (char* dest, const char* src, size_t n) -> char* where dest: 'a, return: 'a]
//   strcat: [unsafe, (char* dest, const char* src) -> char* where dest: 'a, return: 'a]
//   strcmp: [safe, (const char* s1, const char* s2) -> owned]
//   strncmp: [safe, (const char* s1, const char* s2, size_t n) -> owned]
//   strchr: [safe, (const char* str, int c) -> const char* where str: 'a, return: 'a]
//   strrchr: [safe, (const char* str, int c) -> const char* where str: 'a, return: 'a]
//   strstr: [safe, (const char* str, const char* needle) -> const char* where str: 'a, return: 'a]
//   strtok: [unsafe, (char* str, const char* delim) -> char* where str: 'a, return: 'a]
//   strdup: [unsafe, (const char* str) -> owned char*]
// }

// Memory functions
// @external: {
//   malloc: [unsafe, (size_t size) -> owned void*]
//   calloc: [unsafe, (size_t n, size_t size) -> owned void*]
//   realloc: [unsafe, (void* ptr, size_t size) -> owned void*]
//   free: [unsafe, (void* ptr) -> void]
//   memcpy: [unsafe, (void* dest, const void* src, size_t n) -> void* where dest: 'a, return: 'a]
//   memmove: [unsafe, (void* dest, const void* src, size_t n) -> void* where dest: 'a, return: 'a]
//   memset: [unsafe, (void* ptr, int value, size_t n) -> void* where ptr: 'a, return: 'a]
//   memcmp: [safe, (const void* s1, const void* s2, size_t n) -> owned]
// }

// I/O functions
// @external: {
//   fopen: [unsafe, (const char* path, const char* mode) -> owned FILE*]
//   fclose: [unsafe, (FILE* file) -> owned]
//   fread: [unsafe, (void* buffer, size_t size, size_t count, FILE* file) -> owned]
//   fwrite: [unsafe, (const void* buffer, size_t size, size_t count, FILE* file) -> owned]
//   fgets: [safe, (char* buffer, int size, FILE* file) -> char* where buffer: 'a, return: 'a]
//   fputs: [safe, (const char* str, FILE* file) -> owned]
//   fprintf: [safe, (FILE* file, const char* format, ...) -> owned]
//   fscanf: [unsafe, (FILE* file, const char* format, ...) -> owned]
//   printf: [safe, (const char* format, ...) -> owned]
//   sprintf: [unsafe, (char* buffer, const char* format, ...) -> owned]
//   snprintf: [safe, (char* buffer, size_t size, const char* format, ...) -> owned]
// }

// ============================================================================
// POSIX Functions with Lifetimes
// ============================================================================

// File operations
// @external: {
//   open: [unsafe, (const char* path, int flags, ...) -> owned int]
//   close: [unsafe, (int fd) -> owned]
//   read: [unsafe, (int fd, void* buffer, size_t count) -> owned]
//   write: [unsafe, (int fd, const void* buffer, size_t count) -> owned]
//   lseek: [safe, (int fd, off_t offset, int whence) -> owned]
//   dup: [unsafe, (int fd) -> owned int]
//   dup2: [unsafe, (int oldfd, int newfd) -> owned]
//   pipe: [unsafe, (int pipefd[2]) -> owned]
// }

// Directory operations
// @external: {
//   opendir: [unsafe, (const char* path) -> owned DIR*]
//   readdir: [unsafe, (DIR* dir) -> struct dirent* where dir: 'a, return: 'a]
//   closedir: [unsafe, (DIR* dir) -> owned]
//   getcwd: [safe, (char* buffer, size_t size) -> char* where buffer: 'a, return: 'a]
//   chdir: [safe, (const char* path) -> owned]
// }

// Process functions
// @external: {
//   fork: [unsafe, () -> owned pid_t]
//   exec*: [unsafe, (...) -> owned]
//   wait: [safe, (int* status) -> owned pid_t]
//   waitpid: [safe, (pid_t pid, int* status, int options) -> owned pid_t]
//   getpid: [safe, () -> owned pid_t]
//   getppid: [safe, () -> owned pid_t]
// }

// ============================================================================
// Common Third-Party Libraries with Full Annotations
// ============================================================================

// Boost Library
// @external: {
//   boost::lexical_cast: [safe, template<T, S>(const S& arg) -> owned T]
//   boost::format: [safe, (const string& fmt) -> owned format]
//   boost::split: [safe, (vector<string>& result, const string& input, Pred pred) -> void]
//   boost::filesystem::path: [safe, (const string& p) -> owned path]
//   boost::filesystem::exists: [safe, (const path& p) -> owned bool]
//   boost::filesystem::canonical: [safe, (const path& p) -> owned path]
//   boost::shared_ptr::get: [safe, () -> T* where this: 'a, return: 'a]
//   boost::unique_ptr::get: [safe, () -> T* where this: 'a, return: 'a]
//   boost::unique_ptr::release: [unsafe, () -> owned T*]
// }

// JSON Libraries (nlohmann/json)
// @external: {
//   nlohmann::json::parse: [safe, (const string& s) -> owned json]
//   nlohmann::json::dump: [safe, (int indent) -> owned string]
//   nlohmann::json::operator[]: [safe, (const string& key) -> json& where this: 'a, return: 'a]
//   nlohmann::json::at: [safe, (const string& key) -> json& where this: 'a, return: 'a]
//   nlohmann::json::get: [safe, template<T>() -> owned T]
//   nlohmann::json::get_ptr: [safe, template<T>() -> T* where this: 'a, return: 'a]
//   nlohmann::json::get_ref: [safe, template<T>() -> T& where this: 'a, return: 'a]
// }

// SQLite3
// @external: {
//   sqlite3_open: [unsafe, (const char* filename, sqlite3** db) -> owned int]
//   sqlite3_close: [unsafe, (sqlite3* db) -> owned int]
//   sqlite3_prepare_v2: [safe, (sqlite3* db, const char* sql, int nbyte, sqlite3_stmt** stmt, const char** tail) -> owned int]
//   sqlite3_bind_text: [safe, (sqlite3_stmt* stmt, int idx, const char* text, int nbyte, void(*)(void*)) -> owned int]
//   sqlite3_bind_int: [safe, (sqlite3_stmt* stmt, int idx, int value) -> owned int]
//   sqlite3_step: [safe, (sqlite3_stmt* stmt) -> owned int]
//   sqlite3_column_text: [safe, (sqlite3_stmt* stmt, int col) -> const unsigned char* where stmt: 'a, return: 'a]
//   sqlite3_column_int: [safe, (sqlite3_stmt* stmt, int col) -> owned int]
//   sqlite3_finalize: [unsafe, (sqlite3_stmt* stmt) -> owned int]
//   sqlite3_errmsg: [safe, (sqlite3* db) -> const char* where db: 'a, return: 'a]
// }

// OpenSSL
// @external: {
//   EVP_MD_CTX_new: [unsafe, () -> owned EVP_MD_CTX*]
//   EVP_MD_CTX_free: [unsafe, (EVP_MD_CTX* ctx) -> void]
//   EVP_DigestInit_ex: [unsafe, (EVP_MD_CTX* ctx, const EVP_MD* type, ENGINE* impl) -> owned int]
//   EVP_DigestUpdate: [unsafe, (EVP_MD_CTX* ctx, const void* data, size_t count) -> owned int]
//   EVP_DigestFinal_ex: [unsafe, (EVP_MD_CTX* ctx, unsigned char* md, unsigned int* size) -> owned int]
//   SHA256: [safe, (const unsigned char* data, size_t count, unsigned char* md) -> unsigned char* where md: 'a, return: 'a]
//   SHA512: [safe, (const unsigned char* data, size_t count, unsigned char* md) -> unsigned char* where md: 'a, return: 'a]
// }

// CURL
// @external: {
//   curl_easy_init: [unsafe, () -> owned CURL*]
//   curl_easy_cleanup: [unsafe, (CURL* curl) -> void]
//   curl_easy_setopt: [unsafe, (CURL* curl, CURLoption option, ...) -> owned CURLcode]
//   curl_easy_perform: [unsafe, (CURL* curl) -> owned CURLcode]
//   curl_easy_getinfo: [safe, (CURL* curl, CURLINFO info, ...) -> owned CURLcode]
//   curl_easy_strerror: [safe, (CURLcode code) -> const char* where return: 'static]
// }

// ============================================================================
// Extended Syntax for Complex Lifetime Relationships
// ============================================================================

// Functions that create relationships between parameters
// @external_function: container_insert {
//   safety: safe
//   lifetime: (Container<T>& cont, const T& value) -> void
//   where: cont: 'a, value: 'b, 'b: 'a  // value lifetime must outlive container
// }

// Functions that return interior references
// @external_function: get_field {
//   safety: safe
//   lifetime: (const Struct& s) -> const Field&
//   where: s: 'a, return: 'a
// }

// Functions with callback lifetimes
// @external_function: async_operation {
//   safety: unsafe
//   lifetime: (Callback cb, void* context) -> void
//   where: cb: 'static, context: 'a, 'a: 'static
// }

// ============================================================================
// Profiles with Complete Annotations
// ============================================================================

// Qt Framework Profile
// @external_profile: qt {
//   annotations: {
//     QObject::parent: [safe, () -> QObject* where this: 'a, return: 'a]
//     QObject::children: [safe, () -> const QObjectList& where this: 'a, return: 'a]
//     QObject::findChild: [safe, template<T>(const QString& name) -> T* where this: 'a, return: 'a]
//     QObject::connect: [unsafe, (const QObject* sender, const char* signal, const QObject* receiver, const char* method) -> owned QMetaObject::Connection]
//     QString::c_str: [safe, () -> const char* where this: 'a, return: 'a]
//     QString::data: [safe, () -> QChar* where this: 'a, return: 'a]
//     QString::toStdString: [safe, () -> owned std::string]
//     QVector::data: [safe, () -> T* where this: 'a, return: 'a]
//     QVector::at: [safe, (int i) -> const T& where this: 'a, return: 'a]
//     QSharedPointer::data: [safe, () -> T* where this: 'a, return: 'a]
//     QScopedPointer::data: [safe, () -> T* where this: 'a, return: 'a]
//     QScopedPointer::take: [unsafe, () -> owned T*]
//   }
// }

// Embedded Systems Profile
// @external_profile: embedded {
//   annotations: {
//     read_register: [safe, (uint32_t addr) -> owned uint32_t]
//     write_register: [safe, (uint32_t addr, uint32_t value) -> void]
//     get_peripheral: [unsafe, (uint32_t base) -> Peripheral* where return: 'static]
//     dma_transfer: [unsafe, (void* src, void* dest, size_t size) -> owned int]
//     interrupt_handler: [unsafe, (void(*handler)(void)) -> void where handler: 'static]
//   }
// }

// ============================================================================
// Macros for Inline Annotations
// ============================================================================

// Use these macros to annotate specific call sites
#define RUSTYCPP_EXTERNAL(func, safety, lifetime) func
#define RUSTYCPP_SAFE_WITH_LIFETIME(func, lifetime) func
#define RUSTYCPP_UNSAFE_WITH_LIFETIME(func, lifetime) func

// Example usage:
// RUSTYCPP_EXTERNAL(third_party_func(data), safe, (&'a) -> &'a);
// RUSTYCPP_SAFE_WITH_LIFETIME(get_string(), () -> &'static);
// RUSTYCPP_UNSAFE_WITH_LIFETIME(allocate(size), (size_t) -> owned void*);

// ============================================================================
// Helper Annotations for Common Patterns
// ============================================================================

// Pattern: Borrowing getter - returns reference with object lifetime
// @external_pattern: borrowing_getter {
//   matches: ["*::get*", "*::operator[]", "*::at", "*::front", "*::back"]
//   safety: safe
//   lifetime: generic (&'self) -> &'self
// }

// Pattern: Consuming operation - takes ownership
// @external_pattern: consuming_operation {
//   matches: ["*::take", "*::release", "*::move"]
//   safety: unsafe
//   lifetime: generic (&'self mut) -> owned T
// }

// Pattern: Factory function - creates new owned object
// @external_pattern: factory_function {
//   matches: ["*::create*", "*::make*", "*::new*"]
//   safety: varies  // Depends on whether it allocates
//   lifetime: generic (...) -> owned T
// }

#endif // RUSTYCPP_UNIFIED_EXTERNAL_ANNOTATIONS_HPP