
class UniquePtr {
public:
    UniquePtr(int* p) : ptr(p) {}
    UniquePtr(UniquePtr&& other) : ptr(other.ptr) { other.ptr = nullptr; }
    int* ptr;
};

UniquePtr&& move(UniquePtr& p) {
    return static_cast<UniquePtr&&>(p);
}

void test() {
    int* raw = new int(42);
    UniquePtr ptr(raw);
    
    if (raw != nullptr) {
        UniquePtr a = move(ptr);
    } else {
        UniquePtr b = move(ptr);
    }
    
    // ptr is moved in both branches
    UniquePtr c = move(ptr);  // Error: use after move
}
