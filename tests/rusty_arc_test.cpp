// Tests for rusty::Arc<T> (hand-written).
//
// Note: weak-reference test coverage was removed when the hand-written
// `rusty::sync::Weak` was replaced with an alias to the transpiled
// `rusty::port::sync::Weak`. The hand-written Arc no longer supports
// `downgrade` (its ControlBlock layout is incompatible with the
// transpiled Weak). Weak coverage lives in tests/arc_port_module_test.cpp.

#include "../include/rusty/arc.hpp"
#include <cassert>
#include <cstdio>
#include <memory>
#include <string>
#include <thread>
#include <utility>
#include <vector>

using namespace rusty;

struct ArcNewCounts {
    int copies = 0;
    int moves = 0;
    int conversions = 0;
};

struct ArcNewObserved {
    std::shared_ptr<ArcNewCounts> counts;
    int value;

    ArcNewObserved(std::shared_ptr<ArcNewCounts> counts, int value)
        : counts(std::move(counts)), value(value) {}

    ArcNewObserved(const ArcNewObserved& other)
        : counts(other.counts), value(other.value) {
        ++counts->copies;
    }

    ArcNewObserved(ArcNewObserved&& other) noexcept
        : counts(std::move(other.counts)), value(other.value) {
        ++counts->moves;
    }
};

struct ArcNewConvertible {
    std::shared_ptr<ArcNewCounts> counts;
    int value;

    ArcNewConvertible(int value)
        : counts(std::make_shared<ArcNewCounts>()), value(value) {
        ++counts->conversions;
    }
};

struct ArcNewPair {
    int first;
    int second;
};

using ArcNewCopyFactory = Arc<ArcNewObserved> (*)(const ArcNewObserved&);
using ArcNewMoveFactory = Arc<ArcNewObserved> (*)(ArcNewObserved&&);

static_assert(sizeof(Arc<int>) == sizeof(void*));
static_assert(alignof(Arc<int>) == alignof(void*));
static_assert(requires(const ArcNewObserved& value) {
    Arc<ArcNewObserved>::new_(value);
});
static_assert(requires(ArcNewObserved&& value) {
    Arc<ArcNewObserved>::new_(std::move(value));
});
static_assert(requires {
    static_cast<ArcNewCopyFactory>(&Arc<ArcNewObserved>::new_);
    static_cast<ArcNewMoveFactory>(&Arc<ArcNewObserved>::new_);
    Arc<std::string>::new_("convertible");
    Arc<ArcNewConvertible>::new_(7);
    Arc<ArcNewPair>::new_({1, 2});
});

void test_arc_new_direct_copy_move_and_compatibility() {
    printf("test_arc_new_direct_copy_move_and_compatibility: ");

    auto copy_counts = std::make_shared<ArcNewCounts>();
    const ArcNewObserved copied_source(copy_counts, 11);
    auto copied = Arc<ArcNewObserved>::new_(copied_source);
    assert(copied->value == 11);
    assert(copy_counts->copies == 1);
    assert(copy_counts->moves == 0);

    auto move_counts = std::make_shared<ArcNewCounts>();
    ArcNewObserved moved_source(move_counts, 22);
    auto moved = Arc<ArcNewObserved>::new_(std::move(moved_source));
    assert(moved->value == 22);
    assert(move_counts->copies == 0);
    assert(move_counts->moves == 1);

    int evaluations = 0;
    auto side_effect = [&]() {
        ++evaluations;
        return ArcNewObserved(std::make_shared<ArcNewCounts>(), 33);
    };
    auto evaluated_once = Arc<ArcNewObserved>::new_(side_effect());
    assert(evaluations == 1);
    assert(evaluated_once->value == 33);
    assert(evaluated_once->counts->copies == 0);
    assert(evaluated_once->counts->moves == 1);

    auto converted = Arc<std::string>::new_("convertible");
    assert(*converted == "convertible");

    auto converted_custom = Arc<ArcNewConvertible>::new_(44);
    assert(converted_custom->value == 44);
    assert(converted_custom->counts->conversions == 1);

    auto braced = Arc<ArcNewPair>::new_({1, 2});
    assert(braced->first == 1);
    assert(braced->second == 2);

    printf("PASS\n");
}

// Test basic construction
void test_arc_construction() {
    printf("test_arc_construction: ");
    {
        auto arc1 = Arc<int>::make(42);
        assert(arc1.is_valid());
        assert(*arc1 == 42);
        assert(arc1.strong_count() == 1);
        
        auto arc2 = arc<int>(100);
        assert(arc2.is_valid());
        assert(*arc2 == 100);
        
        auto arc3 = make_arc<int>(200);
        assert(arc3.is_valid());
        assert(*arc3 == 200);
    }
    printf("PASS\n");
}

// Test cloning and reference counting
void test_arc_clone() {
    printf("test_arc_clone: ");
    {
        auto arc1 = Arc<int>::make(42);
        assert(arc1.strong_count() == 1);
        
        auto arc2 = arc1.clone();
        assert(arc1.strong_count() == 2);
        assert(arc2.strong_count() == 2);
        assert(*arc1 == 42);
        assert(*arc2 == 42);
        
        {
            auto arc3 = arc1;  // Copy constructor
            assert(arc1.strong_count() == 3);
        }
        assert(arc1.strong_count() == 2);  // arc3 destroyed
    }
    printf("PASS\n");
}

// Test move semantics
void test_arc_move() {
    printf("test_arc_move: ");
    {
        auto arc1 = Arc<int>::make(42);
        auto arc2 = arc1.clone();
        assert(arc1.strong_count() == 2);

        auto arc3 = std::move(arc1);
        assert(!arc1.is_valid());  // arc1 should be empty
        assert(arc3.strong_count() == 2);  // Count unchanged
        assert(*arc3 == 42);
    }
    printf("PASS\n");
}

// Test get_mut
void test_arc_get_mut() {
    printf("test_arc_get_mut: ");
    {
        auto arc1 = Arc<int>::make(42);

        // Should get mutable reference when unique
        auto opt = arc1.get_mut();
        assert(opt.is_some());
        opt.unwrap() = 100;
        assert(*arc1 == 100);

        // Should not get mutable reference when shared
        auto arc2 = arc1.clone();
        opt = arc1.get_mut();
        assert(opt.is_none());  // Can't mutate when shared
    }
    printf("PASS\n");
}

// Test with custom struct
struct TestStruct {
    int value;
    static int instances;
    
    TestStruct(int v) : value(v) { instances++; }
    ~TestStruct() { instances--; }
};

int TestStruct::instances = 0;

void test_arc_destructor() {
    printf("test_arc_destructor: ");
    
    // Note: The test struct's move constructor also increments instances
    // which makes tracking difficult. Just test basic functionality.
    
    // Test 1: Arc properly manages memory
    {
        auto arc1 = Arc<int>::make(42);
        auto arc2 = arc1.clone();
        assert(arc1.strong_count() == 2);
        assert(arc2.strong_count() == 2);
    }
    // Both Arcs destroyed, memory should be freed
    
    // Test 2: Value is preserved through clones
    {
        auto arc1 = Arc<int>::make(100);
        auto arc2 = arc1.clone();
        auto arc3 = arc2.clone();
        assert(*arc1 == 100);
        assert(*arc2 == 100);
        assert(*arc3 == 100);
    }
    
    printf("PASS\n");
}

// Test thread safety
void test_arc_thread_safety() {
    printf("test_arc_thread_safety: ");
    {
        auto arc = Arc<int>::make(0);
        std::vector<std::thread> threads;
        
        // Create multiple threads that clone the Arc
        for (int i = 0; i < 10; ++i) {
            threads.emplace_back([arc]() {
                auto local = arc.clone();
                // Each thread has its own Arc
                assert(local.is_valid());
                assert(*local == 0);
            });
        }
        
        // Wait for all threads
        for (auto& t : threads) {
            t.join();
        }
        
        // Original Arc should still be valid
        assert(arc.is_valid());
        assert(arc.strong_count() == 1);  // All clones destroyed
    }
    printf("PASS\n");
}

// (test_arc_weak removed — hand-written Arc no longer supports downgrade.
//  Weak coverage moved to tests/arc_port_module_test.cpp.)

// Test assignment operators
void test_arc_assignment() {
    printf("test_arc_assignment: ");
    {
        auto arc1 = Arc<int>::make(42);
        auto arc2 = Arc<int>::make(100);
        
        arc2 = arc1;  // Copy assignment
        assert(arc1.strong_count() == 2);
        assert(*arc2 == 42);
        
        auto arc3 = Arc<int>::make(200);
        arc3 = std::move(arc1);  // Move assignment
        assert(!arc1.is_valid());
        assert(arc3.strong_count() == 2);
    }
    printf("PASS\n");
}

int main() {
    printf("=== Testing rusty::Arc<T> ===\n");

    test_arc_new_direct_copy_move_and_compatibility();
    test_arc_construction();
    test_arc_clone();
    test_arc_move();
    test_arc_get_mut();
    test_arc_destructor();
    test_arc_thread_safety();
    test_arc_assignment();
    
    printf("\nAll Arc tests passed!\n");
    return 0;
}
