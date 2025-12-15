// Test: User-Defined RAII Types
// Status: NOT DETECTED (requires RAII tracking Phase 2)
//
// Any class with a destructor is an RAII type. Resources obtained from
// such objects are only valid while the object is alive.

#include <cstdio>
#include <mutex>

// User-defined RAII type: File handle
class FileHandle {
    FILE* fp;
public:
    FileHandle(const char* path) : fp(fopen(path, "r")) {}
    ~FileHandle() { if (fp) fclose(fp); }

    FILE* get() { return fp; }
    bool is_valid() const { return fp != nullptr; }

    // Non-copyable
    FileHandle(const FileHandle&) = delete;
    FileHandle& operator=(const FileHandle&) = delete;
};

// User-defined RAII type: Lock guard
class MyLockGuard {
    std::mutex& mtx;
public:
    MyLockGuard(std::mutex& m) : mtx(m) { mtx.lock(); }
    ~MyLockGuard() { mtx.unlock(); }

    MyLockGuard(const MyLockGuard&) = delete;
    MyLockGuard& operator=(const MyLockGuard&) = delete;
};

// User-defined RAII type: Memory buffer
class Buffer {
    char* data;
    size_t size;
public:
    Buffer(size_t n) : data(new char[n]), size(n) {}
    ~Buffer() { delete[] data; }

    char* get() { return data; }
    size_t length() const { return size; }

    Buffer(const Buffer&) = delete;
    Buffer& operator=(const Buffer&) = delete;
};

// User-defined RAII type: Database connection (simulated)
class DbConnection {
    int handle;
    bool connected;
public:
    DbConnection(const char* connstr) : handle(42), connected(true) {}
    ~DbConnection() { if (connected) { /* disconnect */ connected = false; } }

    int get_handle() { return handle; }
    bool is_connected() const { return connected; }
};

// =============================================================================
// NEGATIVE TESTS - Should produce errors after implementation
// =============================================================================

// @safe
FILE* bad_return_from_file_handle() {
    FileHandle fh("/tmp/test.txt");
    return fh.get();  // ERROR: fp will be closed when fh destroyed
}

// @safe
void bad_store_file_ptr() {
    FILE* ptr;
    {
        FileHandle fh("/tmp/test.txt");
        ptr = fh.get();
    }  // fh destroyed, file closed

    // ERROR: ptr is invalid (file closed)
    // @unsafe
    fread(nullptr, 1, 1, ptr);
}

// @safe
char* bad_return_buffer_ptr() {
    Buffer buf(1024);
    return buf.get();  // ERROR: memory freed when buf destroyed
}

// @safe
void bad_store_buffer_ptr() {
    char* ptr;
    {
        Buffer buf(1024);
        ptr = buf.get();
    }  // buf destroyed, memory freed

    // ERROR: ptr is dangling
    // @unsafe
    ptr[0] = 'x';
}

// @safe
int bad_return_db_handle() {
    DbConnection conn("localhost");
    return conn.get_handle();  // ERROR (conceptually): handle invalid after disconnect
    // Note: This returns an int copy, so it's "safe" in terms of memory,
    // but semantically the handle is invalid. A more sophisticated checker
    // could track this.
}

// Lock guard scope issue
std::mutex global_mutex;

// @safe
void bad_lock_guard_scope() {
    // @unsafe - this is problematic pattern
    {
        MyLockGuard guard(global_mutex);
    }  // guard destroyed here, mutex unlocked

    // Code here runs WITHOUT the lock held
    // This is a logic bug, not memory safety, but related to RAII
}

// Nested RAII
// @safe
char* bad_nested_raii() {
    struct Wrapper {
        Buffer buf;
        Wrapper() : buf(100) {}
        char* get() { return buf.get(); }
    };

    Wrapper w;
    return w.get();  // ERROR: w.buf's memory freed when w destroyed
}

// =============================================================================
// POSITIVE TESTS - Should NOT produce errors
// =============================================================================

// @safe
void good_use_file_in_scope() {
    FileHandle fh("/tmp/test.txt");
    if (fh.is_valid()) {
        FILE* fp = fh.get();
        // Use fp while fh is alive - OK
        // @unsafe
        fread(nullptr, 1, 1, fp);
    }
}  // fh destroyed here, after all uses

// @safe
void good_use_buffer_in_scope() {
    Buffer buf(1024);
    char* ptr = buf.get();
    ptr[0] = 'H';
    ptr[1] = 'i';
    // OK: ptr used while buf is alive
}

// @safe
void good_lock_guard_protects_work() {
    MyLockGuard guard(global_mutex);
    // All work here is protected by the lock
    int x = 42;
    // guard destroyed at end of function, unlocking mutex
}

// @safe
void good_pass_to_function(FileHandle& fh) {
    // Caller owns fh, so its resources are valid
    FILE* fp = fh.get();
    // @unsafe
    fread(nullptr, 1, 1, fp);
}

// @safe
size_t good_copy_value_from_raii() {
    Buffer buf(1024);
    return buf.length();  // Returns copy of size_t, not pointer
}
