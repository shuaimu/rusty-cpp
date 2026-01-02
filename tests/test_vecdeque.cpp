// Comprehensive test file for rusty::VecDeque
// Compile with: g++ -std=c++17 -I../include -o test_vecdeque test_vecdeque.cpp

#include <rusty/vecdeque.hpp>
#include <cassert>
#include <string>
#include <iostream>
#include <vector>
#include <memory>

// ============================================================================
// Helper class for tracking construction/destruction
// ============================================================================

static int g_construct_count = 0;
static int g_destruct_count = 0;
static int g_move_count = 0;
static int g_copy_count = 0;

struct Tracker {
    int value;
    bool valid;

    Tracker() : value(0), valid(true) { ++g_construct_count; }
    explicit Tracker(int v) : value(v), valid(true) { ++g_construct_count; }

    Tracker(const Tracker& other) : value(other.value), valid(true) {
        ++g_construct_count;
        ++g_copy_count;
    }

    Tracker(Tracker&& other) noexcept : value(other.value), valid(true) {
        other.valid = false;
        ++g_construct_count;
        ++g_move_count;
    }

    Tracker& operator=(const Tracker& other) {
        value = other.value;
        valid = true;
        ++g_copy_count;
        return *this;
    }

    Tracker& operator=(Tracker&& other) noexcept {
        value = other.value;
        valid = true;
        other.valid = false;
        ++g_move_count;
        return *this;
    }

    ~Tracker() { ++g_destruct_count; }

    bool operator==(const Tracker& other) const { return value == other.value; }
};

void reset_tracker_counts() {
    g_construct_count = 0;
    g_destruct_count = 0;
    g_move_count = 0;
    g_copy_count = 0;
}

// ============================================================================
// Test Categories
// ============================================================================

// ----------------------------------------------------------------------------
// 1. Construction and Factory Methods
// ----------------------------------------------------------------------------

void test_default_constructor() {
    std::cout << "Testing default constructor..." << std::endl;

    rusty::VecDeque<int> dq;
    assert(dq.is_empty());
    assert(dq.len() == 0);
    assert(dq.size() == 0);
    assert(dq.capacity() == 0);

    std::cout << "  Default constructor passed!" << std::endl;
}

void test_capacity_constructor() {
    std::cout << "Testing capacity constructor..." << std::endl;

    rusty::VecDeque<int> dq(100);
    assert(dq.is_empty());
    assert(dq.capacity() >= 100);

    // Zero capacity
    rusty::VecDeque<int> dq_zero(0);
    assert(dq_zero.is_empty());
    assert(dq_zero.capacity() == 0);

    std::cout << "  Capacity constructor passed!" << std::endl;
}

void test_initializer_list_constructor() {
    std::cout << "Testing initializer list constructor..." << std::endl;

    rusty::VecDeque<int> dq = {10, 20, 30, 40, 50};
    assert(dq.len() == 5);
    assert(dq[0] == 10);
    assert(dq[4] == 50);

    // Empty initializer list
    rusty::VecDeque<int> dq_empty = {};
    assert(dq_empty.is_empty());

    // Single element
    rusty::VecDeque<int> dq_single = {42};
    assert(dq_single.len() == 1);
    assert(dq_single[0] == 42);

    std::cout << "  Initializer list constructor passed!" << std::endl;
}

void test_factory_methods() {
    std::cout << "Testing factory methods..." << std::endl;

    // make()
    auto dq1 = rusty::VecDeque<int>::make();
    assert(dq1.is_empty());

    // with_capacity()
    auto dq2 = rusty::VecDeque<int>::with_capacity(50);
    assert(dq2.is_empty());
    assert(dq2.capacity() >= 50);

    // with_capacity(0)
    auto dq3 = rusty::VecDeque<int>::with_capacity(0);
    assert(dq3.is_empty());
    assert(dq3.capacity() == 0);

    // vecdeque_of helper
    auto dq4 = rusty::vecdeque_of({1, 2, 3});
    assert(dq4.len() == 3);

    std::cout << "  Factory methods passed!" << std::endl;
}

// ----------------------------------------------------------------------------
// 2. Move Semantics
// ----------------------------------------------------------------------------

void test_move_constructor() {
    std::cout << "Testing move constructor..." << std::endl;

    rusty::VecDeque<std::string> dq1;
    dq1.push_back("hello");
    dq1.push_back("world");
    dq1.push_front("prefix");

    rusty::VecDeque<std::string> dq2(std::move(dq1));

    assert(dq2.len() == 3);
    assert(dq2[0] == "prefix");
    assert(dq2[1] == "hello");
    assert(dq2[2] == "world");
    assert(dq1.is_empty());
    assert(dq1.capacity() == 0);

    std::cout << "  Move constructor passed!" << std::endl;
}

void test_move_assignment() {
    std::cout << "Testing move assignment..." << std::endl;

    rusty::VecDeque<std::string> dq1;
    dq1.push_back("a");
    dq1.push_back("b");

    rusty::VecDeque<std::string> dq2;
    dq2.push_back("x");
    dq2.push_back("y");
    dq2.push_back("z");

    dq2 = std::move(dq1);

    assert(dq2.len() == 2);
    assert(dq2[0] == "a");
    assert(dq2[1] == "b");
    assert(dq1.is_empty());

    // Self-assignment check (should be no-op)
    dq2 = std::move(dq2);
    assert(dq2.len() == 2);

    std::cout << "  Move assignment passed!" << std::endl;
}

void test_move_semantics_with_tracker() {
    std::cout << "Testing move semantics with tracker..." << std::endl;
    reset_tracker_counts();

    {
        rusty::VecDeque<Tracker> dq1;
        dq1.push_back(Tracker(1));
        dq1.push_back(Tracker(2));

        int initial_constructs = g_construct_count;
        rusty::VecDeque<Tracker> dq2(std::move(dq1));

        // Move constructor should not copy or move elements
        assert(g_construct_count == initial_constructs);
        assert(dq2.len() == 2);
        assert(dq1.is_empty());
    }

    // All elements should be destructed
    assert(g_construct_count == g_destruct_count);

    std::cout << "  Move semantics with tracker passed!" << std::endl;
}

// ----------------------------------------------------------------------------
// 3. Push Operations
// ----------------------------------------------------------------------------

void test_push_back() {
    std::cout << "Testing push_back..." << std::endl;

    rusty::VecDeque<int> dq;

    for (int i = 0; i < 100; ++i) {
        dq.push_back(i);
        assert(dq.len() == static_cast<size_t>(i + 1));
        assert(dq.back() == i);
        assert(dq.front() == 0);
    }

    for (int i = 0; i < 100; ++i) {
        assert(dq[i] == i);
    }

    std::cout << "  push_back passed!" << std::endl;
}

void test_push_front() {
    std::cout << "Testing push_front..." << std::endl;

    rusty::VecDeque<int> dq;

    for (int i = 0; i < 100; ++i) {
        dq.push_front(i);
        assert(dq.len() == static_cast<size_t>(i + 1));
        assert(dq.front() == i);
        assert(dq.back() == 0);
    }

    for (int i = 0; i < 100; ++i) {
        assert(dq[i] == 99 - i);
    }

    std::cout << "  push_front passed!" << std::endl;
}

void test_alternating_push() {
    std::cout << "Testing alternating push..." << std::endl;

    rusty::VecDeque<int> dq;

    // Push alternating front/back
    for (int i = 0; i < 50; ++i) {
        dq.push_back(i);
        dq.push_front(-i - 1);
    }

    assert(dq.len() == 100);
    assert(dq.front() == -50);
    assert(dq.back() == 49);

    // Verify order: -50, -49, ..., -1, 0, 1, ..., 49
    for (int i = 0; i < 50; ++i) {
        assert(dq[i] == -50 + i);
    }
    for (int i = 0; i < 50; ++i) {
        assert(dq[50 + i] == i);
    }

    std::cout << "  Alternating push passed!" << std::endl;
}

// ----------------------------------------------------------------------------
// 4. Pop Operations
// ----------------------------------------------------------------------------

void test_pop_back() {
    std::cout << "Testing pop_back..." << std::endl;

    rusty::VecDeque<int> dq = {1, 2, 3, 4, 5};

    assert(dq.pop_back() == 5);
    assert(dq.len() == 4);
    assert(dq.pop_back() == 4);
    assert(dq.pop_back() == 3);
    assert(dq.pop_back() == 2);
    assert(dq.pop_back() == 1);
    assert(dq.is_empty());

    std::cout << "  pop_back passed!" << std::endl;
}

void test_pop_front() {
    std::cout << "Testing pop_front..." << std::endl;

    rusty::VecDeque<int> dq = {1, 2, 3, 4, 5};

    assert(dq.pop_front() == 1);
    assert(dq.len() == 4);
    assert(dq.pop_front() == 2);
    assert(dq.pop_front() == 3);
    assert(dq.pop_front() == 4);
    assert(dq.pop_front() == 5);
    assert(dq.is_empty());

    std::cout << "  pop_front passed!" << std::endl;
}

void test_mixed_pop() {
    std::cout << "Testing mixed pop..." << std::endl;

    rusty::VecDeque<int> dq = {1, 2, 3, 4, 5, 6};

    assert(dq.pop_front() == 1);
    assert(dq.pop_back() == 6);
    assert(dq.pop_front() == 2);
    assert(dq.pop_back() == 5);
    assert(dq.len() == 2);
    assert(dq.front() == 3);
    assert(dq.back() == 4);

    std::cout << "  Mixed pop passed!" << std::endl;
}

// ----------------------------------------------------------------------------
// 5. Element Access
// ----------------------------------------------------------------------------

void test_operator_bracket() {
    std::cout << "Testing operator[]..." << std::endl;

    rusty::VecDeque<int> dq = {10, 20, 30, 40, 50};

    // Read access
    assert(dq[0] == 10);
    assert(dq[2] == 30);
    assert(dq[4] == 50);

    // Write access
    dq[1] = 25;
    dq[3] = 45;
    assert(dq[1] == 25);
    assert(dq[3] == 45);

    // Const access
    const rusty::VecDeque<int>& cdq = dq;
    assert(cdq[0] == 10);
    assert(cdq[1] == 25);

    std::cout << "  operator[] passed!" << std::endl;
}

void test_get_method() {
    std::cout << "Testing get method..." << std::endl;

    rusty::VecDeque<int> dq = {5, 10, 15};

    assert(dq.get(0) == 5);
    assert(dq.get(1) == 10);
    assert(dq.get(2) == 15);

    dq.get(1) = 100;
    assert(dq.get(1) == 100);

    std::cout << "  get method passed!" << std::endl;
}

void test_front_back() {
    std::cout << "Testing front/back..." << std::endl;

    rusty::VecDeque<int> dq = {1, 2, 3};

    assert(dq.front() == 1);
    assert(dq.back() == 3);

    dq.front() = 10;
    dq.back() = 30;
    assert(dq.front() == 10);
    assert(dq.back() == 30);

    // Const access
    const rusty::VecDeque<int>& cdq = dq;
    assert(cdq.front() == 10);
    assert(cdq.back() == 30);

    // Single element
    rusty::VecDeque<int> dq_single = {42};
    assert(dq_single.front() == 42);
    assert(dq_single.back() == 42);
    assert(&dq_single.front() == &dq_single.back());

    std::cout << "  front/back passed!" << std::endl;
}

// ----------------------------------------------------------------------------
// 6. Ring Buffer Behavior
// ----------------------------------------------------------------------------

void test_ring_buffer_wrap_around() {
    std::cout << "Testing ring buffer wrap-around..." << std::endl;

    rusty::VecDeque<int> dq(4);  // Small capacity to force wrap

    dq.push_back(1);
    dq.push_back(2);
    dq.push_back(3);
    dq.push_back(4);

    // Remove from front, causing head to advance
    dq.pop_front();  // Remove 1
    dq.pop_front();  // Remove 2

    // Add to back, causing wrap-around
    dq.push_back(5);
    dq.push_back(6);

    assert(dq.len() == 4);
    assert(dq[0] == 3);
    assert(dq[1] == 4);
    assert(dq[2] == 5);
    assert(dq[3] == 6);

    std::cout << "  Ring buffer wrap-around passed!" << std::endl;
}

void test_ring_buffer_extensive() {
    std::cout << "Testing extensive ring buffer operations..." << std::endl;

    rusty::VecDeque<int> dq(8);

    // Simulate queue operations that stress the ring buffer
    for (int round = 0; round < 10; ++round) {
        for (int i = 0; i < 5; ++i) {
            dq.push_back(round * 10 + i);
        }
        for (int i = 0; i < 3; ++i) {
            dq.pop_front();
        }
    }

    // Should have 10 rounds * (5-3) = 20 elements
    assert(dq.len() == 20);

    std::cout << "  Extensive ring buffer passed!" << std::endl;
}

void test_is_contiguous() {
    std::cout << "Testing is_contiguous..." << std::endl;

    rusty::VecDeque<int> dq = {1, 2, 3, 4};
    assert(dq.is_contiguous());

    // Force wrap-around
    dq.pop_front();
    dq.pop_front();
    dq.push_back(5);
    dq.push_back(6);

    // May or may not be contiguous depending on capacity

    // After make_contiguous, should be contiguous
    dq.make_contiguous();
    assert(dq.is_contiguous());

    std::cout << "  is_contiguous passed!" << std::endl;
}

// ----------------------------------------------------------------------------
// 7. Capacity Operations
// ----------------------------------------------------------------------------

void test_reserve() {
    std::cout << "Testing reserve..." << std::endl;

    rusty::VecDeque<int> dq;
    assert(dq.capacity() == 0);

    dq.reserve(100);
    assert(dq.capacity() >= 100);
    assert(dq.is_empty());

    // Adding elements shouldn't increase capacity if under reserve
    for (int i = 0; i < 50; ++i) {
        dq.push_back(i);
    }
    assert(dq.capacity() >= 100);

    // Reserve less than current capacity (should be no-op)
    size_t old_cap = dq.capacity();
    dq.reserve(10);
    assert(dq.capacity() == old_cap);

    std::cout << "  reserve passed!" << std::endl;
}

void test_shrink_to_fit() {
    std::cout << "Testing shrink_to_fit..." << std::endl;

    rusty::VecDeque<int> dq;
    dq.reserve(100);
    assert(dq.capacity() >= 100);

    for (int i = 0; i < 10; ++i) {
        dq.push_back(i);
    }

    dq.shrink_to_fit();
    assert(dq.capacity() == 10);
    assert(dq.len() == 10);

    // Verify data integrity
    for (int i = 0; i < 10; ++i) {
        assert(dq[i] == i);
    }

    // Shrink empty deque
    rusty::VecDeque<int> dq_empty;
    dq_empty.reserve(50);
    dq_empty.shrink_to_fit();
    assert(dq_empty.capacity() == 0);

    std::cout << "  shrink_to_fit passed!" << std::endl;
}

void test_len_size_capacity() {
    std::cout << "Testing len/size/capacity..." << std::endl;

    rusty::VecDeque<int> dq;

    assert(dq.len() == 0);
    assert(dq.size() == 0);
    assert(dq.is_empty());

    dq.push_back(1);
    assert(dq.len() == 1);
    assert(dq.size() == 1);
    assert(!dq.is_empty());
    assert(dq.capacity() >= 1);

    dq.reserve(100);
    assert(dq.len() == 1);
    assert(dq.capacity() >= 100);

    std::cout << "  len/size/capacity passed!" << std::endl;
}

// ----------------------------------------------------------------------------
// 8. Modification Operations
// ----------------------------------------------------------------------------

void test_clear() {
    std::cout << "Testing clear..." << std::endl;
    reset_tracker_counts();

    {
        rusty::VecDeque<Tracker> dq;
        dq.push_back(Tracker(1));
        dq.push_back(Tracker(2));
        dq.push_back(Tracker(3));

        int before_clear = g_destruct_count;
        dq.clear();
        int destructs = g_destruct_count - before_clear;

        assert(dq.is_empty());
        assert(destructs == 3);  // Should destruct all elements

        // Can still use after clear
        dq.push_back(Tracker(10));
        assert(dq.len() == 1);
    }

    std::cout << "  clear passed!" << std::endl;
}

void test_swap_elements() {
    std::cout << "Testing swap elements..." << std::endl;

    rusty::VecDeque<int> dq = {1, 2, 3, 4, 5};

    dq.swap(0, 4);
    assert(dq[0] == 5);
    assert(dq[4] == 1);

    dq.swap(1, 3);
    assert(dq[1] == 4);
    assert(dq[3] == 2);

    // Swap same index (should be no-op)
    dq.swap(2, 2);
    assert(dq[2] == 3);

    std::cout << "  swap elements passed!" << std::endl;
}

void test_rotate_left() {
    std::cout << "Testing rotate_left..." << std::endl;

    rusty::VecDeque<int> dq = {1, 2, 3, 4, 5};

    dq.rotate_left(2);
    assert(dq[0] == 3);
    assert(dq[1] == 4);
    assert(dq[2] == 5);
    assert(dq[3] == 1);
    assert(dq[4] == 2);

    // Rotate by 0 (no-op)
    rusty::VecDeque<int> dq2 = {1, 2, 3};
    dq2.rotate_left(0);
    assert(dq2[0] == 1);

    // Rotate by size (no-op)
    dq2.rotate_left(3);
    assert(dq2[0] == 1);

    // Rotate by more than size (no-op)
    dq2.rotate_left(10);
    assert(dq2[0] == 1);

    std::cout << "  rotate_left passed!" << std::endl;
}

void test_rotate_right() {
    std::cout << "Testing rotate_right..." << std::endl;

    rusty::VecDeque<int> dq = {1, 2, 3, 4, 5};

    dq.rotate_right(2);
    assert(dq[0] == 4);
    assert(dq[1] == 5);
    assert(dq[2] == 1);
    assert(dq[3] == 2);
    assert(dq[4] == 3);

    // Rotate back
    dq.rotate_left(2);
    assert(dq[0] == 1);

    std::cout << "  rotate_right passed!" << std::endl;
}

void test_make_contiguous() {
    std::cout << "Testing make_contiguous..." << std::endl;

    rusty::VecDeque<int> dq(4);

    // Create a wrapped state
    dq.push_back(1);
    dq.push_back(2);
    dq.push_back(3);
    dq.push_back(4);
    dq.pop_front();
    dq.pop_front();
    dq.push_back(5);
    dq.push_back(6);

    // Should have [3, 4, 5, 6] wrapped around

    int* ptr = dq.make_contiguous();
    assert(ptr != nullptr);
    assert(dq.is_contiguous());

    // Verify order is preserved
    assert(dq[0] == 3);
    assert(dq[1] == 4);
    assert(dq[2] == 5);
    assert(dq[3] == 6);

    // Empty deque
    rusty::VecDeque<int> dq_empty;
    assert(dq_empty.make_contiguous() == nullptr);

    std::cout << "  make_contiguous passed!" << std::endl;
}

// ----------------------------------------------------------------------------
// 9. Iterator Support
// ----------------------------------------------------------------------------

void test_iterator_basic() {
    std::cout << "Testing iterator basic..." << std::endl;

    rusty::VecDeque<int> dq = {1, 2, 3, 4, 5};

    int sum = 0;
    for (int x : dq) {
        sum += x;
    }
    assert(sum == 15);

    std::cout << "  Iterator basic passed!" << std::endl;
}

void test_iterator_operations() {
    std::cout << "Testing iterator operations..." << std::endl;

    rusty::VecDeque<int> dq = {10, 20, 30, 40, 50};

    auto it = dq.begin();
    assert(*it == 10);

    ++it;
    assert(*it == 20);

    it++;
    assert(*it == 30);

    it += 2;
    assert(*it == 50);

    --it;
    assert(*it == 40);

    it -= 2;
    assert(*it == 20);

    assert(it[0] == 20);
    assert(it[1] == 30);

    auto it2 = dq.begin() + 3;
    assert(it2 - it == 2);

    std::cout << "  Iterator operations passed!" << std::endl;
}

void test_iterator_comparison() {
    std::cout << "Testing iterator comparison..." << std::endl;

    rusty::VecDeque<int> dq = {1, 2, 3};

    auto it1 = dq.begin();
    auto it2 = dq.begin();
    auto it3 = dq.begin() + 1;

    assert(it1 == it2);
    assert(it1 != it3);
    assert(it1 < it3);
    assert(it3 > it1);
    assert(it1 <= it2);
    assert(it1 <= it3);
    assert(it3 >= it1);
    assert(it1 >= it2);

    std::cout << "  Iterator comparison passed!" << std::endl;
}

void test_const_iterator() {
    std::cout << "Testing const iterator..." << std::endl;

    rusty::VecDeque<int> dq = {1, 2, 3, 4, 5};
    const rusty::VecDeque<int>& cdq = dq;

    int sum = 0;
    for (auto it = cdq.begin(); it != cdq.end(); ++it) {
        sum += *it;
    }
    assert(sum == 15);

    // cbegin/cend
    sum = 0;
    for (auto it = dq.cbegin(); it != dq.cend(); ++it) {
        sum += *it;
    }
    assert(sum == 15);

    std::cout << "  Const iterator passed!" << std::endl;
}

void test_iterator_modify() {
    std::cout << "Testing iterator modify..." << std::endl;

    rusty::VecDeque<int> dq = {1, 2, 3, 4, 5};

    for (auto& x : dq) {
        x *= 2;
    }

    assert(dq[0] == 2);
    assert(dq[1] == 4);
    assert(dq[2] == 6);
    assert(dq[3] == 8);
    assert(dq[4] == 10);

    std::cout << "  Iterator modify passed!" << std::endl;
}

// ----------------------------------------------------------------------------
// 10. Utility Operations
// ----------------------------------------------------------------------------

void test_clone() {
    std::cout << "Testing clone..." << std::endl;

    rusty::VecDeque<int> dq1 = {1, 2, 3, 4, 5};
    rusty::VecDeque<int> dq2 = dq1.clone();

    assert(dq1 == dq2);
    assert(dq2.len() == 5);

    // Modifications to clone don't affect original
    dq2.push_back(6);
    dq2[0] = 100;
    assert(dq1.len() == 5);
    assert(dq1[0] == 1);

    // Clone empty
    rusty::VecDeque<int> dq_empty;
    auto cloned_empty = dq_empty.clone();
    assert(cloned_empty.is_empty());

    std::cout << "  clone passed!" << std::endl;
}

void test_append() {
    std::cout << "Testing append..." << std::endl;

    rusty::VecDeque<int> dq1 = {1, 2, 3};
    rusty::VecDeque<int> dq2 = {4, 5, 6};

    dq1.append(std::move(dq2));

    assert(dq1.len() == 6);
    assert(dq1[0] == 1);
    assert(dq1[3] == 4);
    assert(dq1[5] == 6);
    assert(dq2.is_empty());

    // Append empty
    rusty::VecDeque<int> dq_empty;
    dq1.append(std::move(dq_empty));
    assert(dq1.len() == 6);

    // Append to empty
    rusty::VecDeque<int> dq3;
    rusty::VecDeque<int> dq4 = {7, 8, 9};
    dq3.append(std::move(dq4));
    assert(dq3.len() == 3);
    assert(dq3[0] == 7);

    std::cout << "  append passed!" << std::endl;
}

void test_equality() {
    std::cout << "Testing equality..." << std::endl;

    rusty::VecDeque<int> dq1 = {1, 2, 3};
    rusty::VecDeque<int> dq2 = {1, 2, 3};
    rusty::VecDeque<int> dq3 = {1, 2, 4};
    rusty::VecDeque<int> dq4 = {1, 2};
    rusty::VecDeque<int> dq5;

    assert(dq1 == dq2);
    assert(!(dq1 != dq2));

    assert(dq1 != dq3);
    assert(dq1 != dq4);
    assert(dq1 != dq5);

    // Empty equality
    rusty::VecDeque<int> dq6;
    assert(dq5 == dq6);

    std::cout << "  equality passed!" << std::endl;
}

// ----------------------------------------------------------------------------
// 11. Retain and Extract
// ----------------------------------------------------------------------------

void test_retain() {
    std::cout << "Testing retain..." << std::endl;

    rusty::VecDeque<int> dq = {1, 2, 3, 4, 5, 6, 7, 8, 9, 10};

    // Keep only even numbers
    dq.retain([](int x) { return x % 2 == 0; });

    assert(dq.len() == 5);
    assert(dq[0] == 2);
    assert(dq[1] == 4);
    assert(dq[2] == 6);
    assert(dq[3] == 8);
    assert(dq[4] == 10);

    // Retain all
    dq.retain([](int) { return true; });
    assert(dq.len() == 5);

    // Retain none
    dq.retain([](int) { return false; });
    assert(dq.is_empty());

    // Retain on empty
    rusty::VecDeque<int> dq_empty;
    dq_empty.retain([](int) { return true; });
    assert(dq_empty.is_empty());

    std::cout << "  retain passed!" << std::endl;
}

void test_retain_with_tracker() {
    std::cout << "Testing retain with tracker..." << std::endl;
    reset_tracker_counts();

    {
        rusty::VecDeque<Tracker> dq;
        for (int i = 0; i < 10; ++i) {
            dq.push_back(Tracker(i));
        }

        int before_retain = g_destruct_count;
        dq.retain([](const Tracker& t) { return t.value % 2 == 0; });
        int destructs = g_destruct_count - before_retain;

        assert(dq.len() == 5);
        assert(destructs >= 5);  // At least 5 odd elements destroyed
    }

    std::cout << "  retain with tracker passed!" << std::endl;
}

void test_extract_if() {
    std::cout << "Testing extract_if..." << std::endl;

    rusty::VecDeque<int> dq = {1, 2, 3, 4, 5, 6, 7, 8, 9, 10};

    // Extract odd numbers
    auto odds = dq.extract_if([](int x) { return x % 2 == 1; });

    // Original should have evens
    assert(dq.len() == 5);
    assert(dq[0] == 2);
    assert(dq[4] == 10);

    // Extracted should have odds
    assert(odds.len() == 5);
    assert(odds[0] == 1);
    assert(odds[4] == 9);

    // Extract none
    auto none = dq.extract_if([](int) { return false; });
    assert(none.is_empty());
    assert(dq.len() == 5);

    // Extract all
    auto all = dq.extract_if([](int) { return true; });
    assert(all.len() == 5);
    assert(dq.is_empty());

    // Extract from empty
    rusty::VecDeque<int> dq_empty;
    auto from_empty = dq_empty.extract_if([](int) { return true; });
    assert(from_empty.is_empty());

    std::cout << "  extract_if passed!" << std::endl;
}

// ----------------------------------------------------------------------------
// 12. Complex Type Tests
// ----------------------------------------------------------------------------

void test_string_type() {
    std::cout << "Testing with string type..." << std::endl;

    rusty::VecDeque<std::string> dq;

    dq.push_back("hello");
    dq.push_front("world");
    dq.push_back("foo");
    dq.push_front("bar");

    assert(dq.len() == 4);
    assert(dq[0] == "bar");
    assert(dq[1] == "world");
    assert(dq[2] == "hello");
    assert(dq[3] == "foo");

    // Pop
    assert(dq.pop_front() == "bar");
    assert(dq.pop_back() == "foo");

    std::cout << "  String type passed!" << std::endl;
}

void test_nested_vecdeque() {
    std::cout << "Testing nested VecDeque..." << std::endl;

    rusty::VecDeque<rusty::VecDeque<int>> outer;

    rusty::VecDeque<int> inner1 = {1, 2, 3};
    rusty::VecDeque<int> inner2 = {4, 5, 6};

    outer.push_back(std::move(inner1));
    outer.push_back(std::move(inner2));

    assert(outer.len() == 2);
    assert(outer[0].len() == 3);
    assert(outer[0][0] == 1);
    assert(outer[1][2] == 6);

    std::cout << "  Nested VecDeque passed!" << std::endl;
}

// ----------------------------------------------------------------------------
// 13. Stress Tests
// ----------------------------------------------------------------------------

void test_stress_push_pop() {
    std::cout << "Testing stress push/pop..." << std::endl;

    rusty::VecDeque<int> dq;

    // Many push/pop cycles
    for (int cycle = 0; cycle < 100; ++cycle) {
        for (int i = 0; i < 100; ++i) {
            if (i % 2 == 0) {
                dq.push_back(i);
            } else {
                dq.push_front(i);
            }
        }

        for (int i = 0; i < 50; ++i) {
            dq.pop_front();
            dq.pop_back();
        }
    }

    assert(dq.is_empty());

    std::cout << "  Stress push/pop passed!" << std::endl;
}

void test_stress_large_data() {
    std::cout << "Testing stress large data..." << std::endl;

    rusty::VecDeque<int> dq;

    const int N = 100000;
    for (int i = 0; i < N; ++i) {
        dq.push_back(i);
    }

    assert(dq.len() == N);
    assert(dq.front() == 0);
    assert(dq.back() == N - 1);

    // Random access check
    assert(dq[N / 2] == N / 2);

    // Pop all
    for (int i = 0; i < N / 2; ++i) {
        dq.pop_front();
        dq.pop_back();
    }

    assert(dq.is_empty());

    std::cout << "  Stress large data passed!" << std::endl;
}

void test_stress_mixed_operations() {
    std::cout << "Testing stress mixed operations..." << std::endl;

    rusty::VecDeque<int> dq;

    for (int i = 0; i < 1000; ++i) {
        int op = i % 6;
        switch (op) {
            case 0: dq.push_back(i); break;
            case 1: dq.push_front(i); break;
            case 2: if (!dq.is_empty()) dq.pop_back(); break;
            case 3: if (!dq.is_empty()) dq.pop_front(); break;
            case 4: if (dq.len() >= 2) dq.swap(0, dq.len() - 1); break;
            case 5: if (dq.len() >= 2) dq.rotate_left(1); break;
        }
    }

    // Just ensure no crashes
    std::cout << "  Stress mixed operations passed!" << std::endl;
}

// ----------------------------------------------------------------------------
// 14. Edge Cases
// ----------------------------------------------------------------------------

void test_single_element() {
    std::cout << "Testing single element..." << std::endl;

    rusty::VecDeque<int> dq;
    dq.push_back(42);

    assert(dq.len() == 1);
    assert(dq.front() == 42);
    assert(dq.back() == 42);
    assert(dq[0] == 42);
    assert(&dq.front() == &dq.back());

    assert(dq.pop_back() == 42);
    assert(dq.is_empty());

    dq.push_front(100);
    assert(dq.pop_front() == 100);
    assert(dq.is_empty());

    std::cout << "  Single element passed!" << std::endl;
}

void test_empty_operations() {
    std::cout << "Testing empty operations..." << std::endl;

    rusty::VecDeque<int> dq;

    assert(dq.is_empty());
    assert(dq.len() == 0);

    // Iterators on empty
    assert(dq.begin() == dq.end());
    assert(dq.cbegin() == dq.cend());

    // Clone empty
    auto cloned = dq.clone();
    assert(cloned.is_empty());

    // Clear empty
    dq.clear();
    assert(dq.is_empty());

    // Reserve on empty
    dq.reserve(100);
    assert(dq.is_empty());
    assert(dq.capacity() >= 100);

    // Shrink empty
    dq.shrink_to_fit();
    assert(dq.capacity() == 0);

    std::cout << "  Empty operations passed!" << std::endl;
}

void test_wrap_around_edge_case() {
    std::cout << "Testing wrap around edge case..." << std::endl;

    // Create a situation where head is at capacity - 1
    rusty::VecDeque<int> dq(4);

    dq.push_back(1);
    dq.push_back(2);
    dq.push_back(3);
    dq.push_back(4);

    // Remove all from front
    dq.pop_front();
    dq.pop_front();
    dq.pop_front();
    dq.pop_front();
    assert(dq.is_empty());

    // Now head should be at some position, add from front
    dq.push_front(10);
    assert(dq.front() == 10);
    assert(dq.back() == 10);

    dq.push_front(20);
    assert(dq.front() == 20);
    assert(dq.back() == 10);

    std::cout << "  Wrap around edge case passed!" << std::endl;
}

// ============================================================================
// Main
// ============================================================================

int main() {
    std::cout << "=== Comprehensive VecDeque Tests ===" << std::endl;
    std::cout << std::endl;

    // 1. Construction and Factory Methods
    std::cout << "--- 1. Construction and Factory Methods ---" << std::endl;
    test_default_constructor();
    test_capacity_constructor();
    test_initializer_list_constructor();
    test_factory_methods();

    // 2. Move Semantics
    std::cout << "\n--- 2. Move Semantics ---" << std::endl;
    test_move_constructor();
    test_move_assignment();
    test_move_semantics_with_tracker();

    // 3. Push Operations
    std::cout << "\n--- 3. Push Operations ---" << std::endl;
    test_push_back();
    test_push_front();
    test_alternating_push();

    // 4. Pop Operations
    std::cout << "\n--- 4. Pop Operations ---" << std::endl;
    test_pop_back();
    test_pop_front();
    test_mixed_pop();

    // 5. Element Access
    std::cout << "\n--- 5. Element Access ---" << std::endl;
    test_operator_bracket();
    test_get_method();
    test_front_back();

    // 6. Ring Buffer Behavior
    std::cout << "\n--- 6. Ring Buffer Behavior ---" << std::endl;
    test_ring_buffer_wrap_around();
    test_ring_buffer_extensive();
    test_is_contiguous();

    // 7. Capacity Operations
    std::cout << "\n--- 7. Capacity Operations ---" << std::endl;
    test_reserve();
    test_shrink_to_fit();
    test_len_size_capacity();

    // 8. Modification Operations
    std::cout << "\n--- 8. Modification Operations ---" << std::endl;
    test_clear();
    test_swap_elements();
    test_rotate_left();
    test_rotate_right();
    test_make_contiguous();

    // 9. Iterator Support
    std::cout << "\n--- 9. Iterator Support ---" << std::endl;
    test_iterator_basic();
    test_iterator_operations();
    test_iterator_comparison();
    test_const_iterator();
    test_iterator_modify();

    // 10. Utility Operations
    std::cout << "\n--- 10. Utility Operations ---" << std::endl;
    test_clone();
    test_append();
    test_equality();

    // 11. Retain and Extract
    std::cout << "\n--- 11. Retain and Extract ---" << std::endl;
    test_retain();
    test_retain_with_tracker();
    test_extract_if();

    // 12. Complex Type Tests
    std::cout << "\n--- 12. Complex Type Tests ---" << std::endl;
    test_string_type();
    test_nested_vecdeque();

    // 13. Stress Tests
    std::cout << "\n--- 13. Stress Tests ---" << std::endl;
    test_stress_push_pop();
    test_stress_large_data();
    test_stress_mixed_operations();

    // 14. Edge Cases
    std::cout << "\n--- 14. Edge Cases ---" << std::endl;
    test_single_element();
    test_empty_operations();
    test_wrap_around_edge_case();

    std::cout << "\n=== All 47 VecDeque tests passed! ===" << std::endl;
    return 0;
}
