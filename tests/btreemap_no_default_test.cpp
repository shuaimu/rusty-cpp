#include "../include/rusty/btreemap.hpp"
#include "../include/rusty/btreeset.hpp"
#include <iostream>
#include <cassert>
#include <random>

using namespace rusty;

// Test type that deliberately has no default constructor
// This is the key use case we're fixing - types like Rc<T>, Box<T>, etc.
class NoDefault {
private:
    int value_;
    NoDefault() = delete;  // No default constructor!
public:
    explicit NoDefault(int v) : value_(v) {}
    NoDefault(const NoDefault& other) : value_(other.value_) {}
    NoDefault(NoDefault&& other) noexcept : value_(other.value_) {}
    NoDefault& operator=(const NoDefault& other) { value_ = other.value_; return *this; }
    NoDefault& operator=(NoDefault&& other) noexcept { value_ = other.value_; return *this; }

    int value() const { return value_; }
    bool operator<(const NoDefault& other) const { return value_ < other.value_; }
    bool operator==(const NoDefault& other) const { return value_ == other.value_; }
};


void test_btreemap_nodefault_basic() {
    std::cout << "Testing BTreeMap with NoDefault type..." << std::endl;

    BTreeMap<NoDefault, NoDefault> map;
    assert(map.is_empty());
    assert(map.len() == 0);

    // Insert
    map.insert(NoDefault(3), NoDefault(30));
    map.insert(NoDefault(1), NoDefault(10));
    map.insert(NoDefault(2), NoDefault(20));

    assert(!map.is_empty());
    assert(map.len() == 3);

    // Contains key
    assert(map.contains_key(NoDefault(1)));
    assert(map.contains_key(NoDefault(2)));
    assert(map.contains_key(NoDefault(3)));
    assert(!map.contains_key(NoDefault(4)));

    // Get values
    auto v1 = map.get(NoDefault(1));
    assert(v1.is_some());
    assert(v1.unwrap()->value() == 10);

    auto v2 = map.get(NoDefault(2));
    assert(v2.is_some());
    assert(v2.unwrap()->value() == 20);

    auto v3 = map.get(NoDefault(3));
    assert(v3.is_some());
    assert(v3.unwrap()->value() == 30);

    std::cout << "✓ BTreeMap NoDefault basic tests passed" << std::endl;
}

void test_btreemap_nodefault_stress() {
    std::cout << "Testing BTreeMap with NoDefault type (stress)..." << std::endl;

    BTreeMap<NoDefault, NoDefault> map;
    const int N = 1000;

    // Insert in reverse order to stress the tree
    for (int i = N - 1; i >= 0; --i) {
        map.insert(NoDefault(i), NoDefault(i * 10));
    }
    assert(map.len() == N);

    // Verify all present
    for (int i = 0; i < N; ++i) {
        if (!map.contains_key(NoDefault(i))) {
            std::cerr << "ERROR: key " << i << " not found!" << std::endl;
            assert(false);
        }
        auto val = map.get(NoDefault(i));
        assert(val.is_some());
        assert(val.unwrap()->value() == i * 10);
    }

    std::cout << "✓ BTreeMap NoDefault stress tests passed (" << N << " elements)" << std::endl;
}

void test_btreemap_nodefault_random() {
    std::cout << "Testing BTreeMap with NoDefault type (random order)..." << std::endl;

    BTreeMap<NoDefault, NoDefault> map;
    const int N = 500;

    // Create shuffled array
    std::vector<int> nums;
    for (int i = 0; i < N; ++i) nums.push_back(i);
    std::random_device rd;
    std::mt19937 gen(rd());
    std::shuffle(nums.begin(), nums.end(), gen);

    // Insert in random order
    for (int i : nums) {
        map.insert(NoDefault(i), NoDefault(i * 10));
    }
    assert(map.len() == N);

    // Verify all present
    for (int i = 0; i < N; ++i) {
        assert(map.contains_key(NoDefault(i)));
    }

    // Random lookups
    std::uniform_int_distribution<> dis(0, N - 1);
    for (int i = 0; i < 10000; ++i) {
        int key = dis(gen);
        assert(map.contains_key(NoDefault(key)));
    }

    std::cout << "✓ BTreeMap NoDefault random tests passed (" << N << " elements)" << std::endl;
}

void test_btreemap_nodefault_remove() {
    std::cout << "Testing BTreeMap with NoDefault type (remove)..." << std::endl;

    BTreeMap<NoDefault, NoDefault> map;
    for (int i = 0; i < 100; ++i) {
        map.insert(NoDefault(i), NoDefault(i * 10));
    }
    assert(map.len() == 100);

    // Remove every other element
    for (int i = 0; i < 100; i += 2) {
        auto removed = map.remove(NoDefault(i));
        assert(removed.is_some());
        assert(removed.unwrap().value() == i * 10);
    }
    assert(map.len() == 50);

    // Verify pattern
    for (int i = 0; i < 100; ++i) {
        if (i % 2 == 0) {
            assert(!map.contains_key(NoDefault(i)));
        } else {
            assert(map.contains_key(NoDefault(i)));
        }
    }

    std::cout << "✓ BTreeMap NoDefault remove tests passed" << std::endl;
}

void test_btreemap_nodefault_update() {
    std::cout << "Testing BTreeMap with NoDefault type (update)..." << std::endl;

    BTreeMap<NoDefault, NoDefault> map;
    map.insert(NoDefault(1), NoDefault(10));
    map.insert(NoDefault(2), NoDefault(20));

    // Update existing key
    auto old = map.insert(NoDefault(1), NoDefault(100));
    assert(old.is_some());
    assert(old.unwrap().value() == 10);
    assert(map.len() == 2);

    // Verify updated value
    auto val = map.get(NoDefault(1));
    assert(val.is_some());
    assert(val.unwrap()->value() == 100);

    std::cout << "✓ BTreeMap NoDefault update tests passed" << std::endl;
}

void test_btreemap_nodefault_iteration() {
    std::cout << "Testing BTreeMap with NoDefault type (iteration)..." << std::endl;

    BTreeMap<NoDefault, NoDefault> map;
    map.insert(NoDefault(3), NoDefault(30));
    map.insert(NoDefault(1), NoDefault(10));
    map.insert(NoDefault(2), NoDefault(20));

    // Iteration should be in sorted order
    int expected = 1;
    int count = 0;
    for (const auto& [key, val] : map) {
        assert(key.value() == expected);
        assert(val.value() == expected * 10);
        ++expected;
        ++count;
    }
    assert(count == 3);

    std::cout << "✓ BTreeMap NoDefault iteration tests passed" << std::endl;
}

// Note: MoveOnly types require additional work because median keys need
// to be copied during splits. This is a limitation shared with std::map.
// The primary goal was to support types without default constructors (like Rc<T>),
// which typically ARE copyable.

void test_btreeset_nodefault_basic() {
    std::cout << "Testing BTreeSet with NoDefault type..." << std::endl;

    BTreeSet<NoDefault> set;
    assert(set.is_empty());
    assert(set.len() == 0);

    // Insert
    assert(set.insert(NoDefault(3)) == true);
    assert(set.insert(NoDefault(1)) == true);
    assert(set.insert(NoDefault(2)) == true);
    assert(set.insert(NoDefault(2)) == false);  // Duplicate

    assert(!set.is_empty());
    assert(set.len() == 3);

    // Contains
    assert(set.contains(NoDefault(1)));
    assert(set.contains(NoDefault(2)));
    assert(set.contains(NoDefault(3)));
    assert(!set.contains(NoDefault(4)));

    std::cout << "✓ BTreeSet NoDefault basic tests passed" << std::endl;
}

void test_btreeset_nodefault_stress() {
    std::cout << "Testing BTreeSet with NoDefault type (stress)..." << std::endl;

    BTreeSet<NoDefault> set;
    const int N = 1000;

    // Insert in reverse order
    for (int i = N - 1; i >= 0; --i) {
        set.insert(NoDefault(i));
    }
    assert(set.len() == N);

    // Verify all present
    for (int i = 0; i < N; ++i) {
        if (!set.contains(NoDefault(i))) {
            std::cerr << "ERROR: value " << i << " not found!" << std::endl;
            assert(false);
        }
    }

    std::cout << "✓ BTreeSet NoDefault stress tests passed (" << N << " elements)" << std::endl;
}

void test_btreeset_nodefault_remove() {
    std::cout << "Testing BTreeSet with NoDefault type (remove)..." << std::endl;

    BTreeSet<NoDefault> set;
    for (int i = 0; i < 100; ++i) {
        set.insert(NoDefault(i));
    }
    assert(set.len() == 100);

    // Remove every other element
    for (int i = 0; i < 100; i += 2) {
        assert(set.remove(NoDefault(i)) == true);
    }
    assert(set.len() == 50);

    // Verify pattern
    for (int i = 0; i < 100; ++i) {
        if (i % 2 == 0) {
            assert(!set.contains(NoDefault(i)));
        } else {
            assert(set.contains(NoDefault(i)));
        }
    }

    std::cout << "✓ BTreeSet NoDefault remove tests passed" << std::endl;
}

void test_btreemap_keys_values() {
    std::cout << "Testing BTreeMap keys() and values()..." << std::endl;

    BTreeMap<NoDefault, NoDefault> map;
    map.insert(NoDefault(3), NoDefault(30));
    map.insert(NoDefault(1), NoDefault(10));
    map.insert(NoDefault(2), NoDefault(20));

    auto keys = map.keys();
    assert(keys.len() == 3);
    assert(keys[0].value() == 1);
    assert(keys[1].value() == 2);
    assert(keys[2].value() == 3);

    auto values = map.values();
    assert(values.len() == 3);
    assert(values[0].value() == 10);
    assert(values[1].value() == 20);
    assert(values[2].value() == 30);

    std::cout << "✓ BTreeMap keys/values tests passed" << std::endl;
}

void test_btreemap_move_semantics() {
    std::cout << "Testing BTreeMap move semantics..." << std::endl;

    BTreeMap<NoDefault, NoDefault> map1;
    map1.insert(NoDefault(1), NoDefault(10));
    map1.insert(NoDefault(2), NoDefault(20));

    // Move constructor
    BTreeMap<NoDefault, NoDefault> map2 = std::move(map1);
    assert(map2.len() == 2);
    assert(map2.contains_key(NoDefault(1)));
    assert(map2.contains_key(NoDefault(2)));

    // Move assignment
    BTreeMap<NoDefault, NoDefault> map3;
    map3 = std::move(map2);
    assert(map3.len() == 2);

    std::cout << "✓ BTreeMap move semantics tests passed" << std::endl;
}

void test_btreemap_clear() {
    std::cout << "Testing BTreeMap clear..." << std::endl;

    BTreeMap<NoDefault, NoDefault> map;
    for (int i = 0; i < 100; ++i) {
        map.insert(NoDefault(i), NoDefault(i * 10));
    }
    assert(map.len() == 100);

    map.clear();
    assert(map.is_empty());
    assert(map.len() == 0);

    // Re-insert after clear
    map.insert(NoDefault(1), NoDefault(10));
    assert(map.len() == 1);
    assert(map.contains_key(NoDefault(1)));

    std::cout << "✓ BTreeMap clear tests passed" << std::endl;
}

int main() {
    std::cout << "============================================" << std::endl;
    std::cout << "BTreeMap/BTreeSet No-Default-Constructor Tests" << std::endl;
    std::cout << "============================================" << std::endl;
    std::cout << std::endl;

    // BTreeMap tests
    test_btreemap_nodefault_basic();
    test_btreemap_nodefault_stress();
    test_btreemap_nodefault_random();
    test_btreemap_nodefault_remove();
    test_btreemap_nodefault_update();
    test_btreemap_nodefault_iteration();
    test_btreemap_keys_values();
    test_btreemap_move_semantics();
    test_btreemap_clear();

    std::cout << std::endl;

    // BTreeSet tests
    test_btreeset_nodefault_basic();
    test_btreeset_nodefault_stress();
    test_btreeset_nodefault_remove();

    std::cout << std::endl;
    std::cout << "============================================" << std::endl;
    std::cout << "✅ All No-Default-Constructor tests passed!" << std::endl;
    std::cout << "============================================" << std::endl;

    return 0;
}
