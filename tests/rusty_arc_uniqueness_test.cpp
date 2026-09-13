#ifdef NDEBUG
#undef NDEBUG
#endif

// Arc uniqueness must include Weak owners and concurrent downgrade/upgrade.
#include <rusty/arc.hpp>
#include <rusty/sync/weak.hpp>
#include <atomic>
#include <cassert>
#include <cstdio>
#include <optional>
#include <thread>
#include <utility>

static_assert(sizeof(rusty::Arc<int>) == sizeof(void*));
static_assert(alignof(rusty::Arc<int>) == alignof(void*));
static_assert(sizeof(rusty::sync::Weak<int>) == sizeof(void*));

static void weak_owners_exclude_mutation() {
    auto owner = rusty::Arc<int>::new_(7);
    assert(owner.get_mut().is_some());
    auto weak = rusty::downgrade(owner);
    assert(owner.strong_count() == 1);
    assert(owner.weak_count() == 1);
    assert(owner.get_mut().is_none());
    auto copy = weak.clone();
    weak.reset();
    assert(owner.get_mut().is_none());
    {
        auto upgraded = copy.upgrade().unwrap();
        copy.reset();
        assert(owner.weak_count() == 0);
        assert(owner.get_mut().is_none());
        assert(*upgraded == 7);
    }
    assert(owner.get_mut().is_some());
    owner.get_mut().unwrap() = 11;
    assert(*owner == 11);
}

struct Cyclic {
    rusty::sync::Weak<Cyclic> self;
    int value;
};

static void cyclic_and_expired_weak_owners() {
    rusty::sync::Weak<Cyclic> observer;
    {
        auto owner = rusty::Arc<Cyclic>::new_cyclic([](const auto& weak) {
            assert(weak.upgrade().is_none());
            return Cyclic{weak.clone(), 17};
        });
        assert(owner.strong_count() == 1);
        assert(owner.weak_count() == 1);
        assert(owner.get_mut().is_none());
        assert(owner->self.upgrade().unwrap()->value == 17);
        observer = rusty::downgrade(owner);
    }
    assert(observer.expired());
    assert(observer.upgrade().is_none());
}

static void weak_assignment_preserves_counts() {
    auto first = rusty::Arc<int>::new_(1);
    auto second = rusty::Arc<int>::new_(2);
    rusty::sync::Weak<int> weak;
    weak = first;
    assert(first.get_mut().is_none());
    weak = second;
    assert(first.get_mut().is_some());
    assert(second.get_mut().is_none());
    rusty::sync::Weak<int> copy;
    copy = weak;
    weak.reset();
    assert(second.get_mut().is_none());
    weak = std::move(copy);
    assert(second.weak_count() == 1);
    weak.reset();
    assert(second.get_mut().is_some());
}

static void concurrent_downgrade_upgrade_never_grants_mutation() {
    auto owner = rusty::Arc<int>::new_(29);
    std::atomic<bool> started{false};
    std::atomic<bool> finished{false};
    std::atomic<bool> release{false};
    std::thread worker([held = owner.clone(), &started, &finished, &release]() mutable {
        std::optional<rusty::Arc<int>> strong(std::move(held));
        started.store(true, std::memory_order_release);
        // At every point the worker owns a strong or weak reference. There
        // is no legal mutation window, including the handoff between kinds.
        for (int i = 0; i < 200000; ++i) {
            assert(strong->weak_count() <= 1);
            auto weak = rusty::downgrade(*strong);
            strong.reset();
            assert(weak.strong_count() >= 1);
            auto upgraded = weak.upgrade();
            assert(upgraded.is_some());
            strong.emplace(std::move(upgraded).unwrap());
            weak.reset();
            assert(**strong == 29);
        }
        finished.store(true, std::memory_order_release);
        // Retain the final strong owner until the caller has stopped checking.
        while (!release.load(std::memory_order_acquire)) {
            std::this_thread::yield();
        }
    });
    while (!started.load(std::memory_order_acquire)) {
        std::this_thread::yield();
    }
    size_t checked = 0;
    do {
        assert(owner.get_mut().is_none());
        assert(owner.weak_count() <= 1);
        ++checked;
    } while (!finished.load(std::memory_order_acquire));
    release.store(true, std::memory_order_release);
    worker.join();
    assert(checked > 0);
    assert(owner.get_mut().is_some());
    owner.get_mut().unwrap() = 31;
    assert(*owner == 31);
}

static void weak_count_requires_a_live_strong_owner() {
    rusty::sync::Weak<int> first;
    rusty::sync::Weak<int> second;
    assert(first.weak_count() == 0);
    {
        auto owner = rusty::Arc<int>::new_(13);
        first = rusty::downgrade(owner);
        second = first.clone();
        assert(first.weak_count() == 2);
        assert(second.weak_count() == 2);
        assert(owner.weak_count() == 2);
    }
    assert(first.weak_count() == 0);
    assert(second.weak_count() == 0);
    second.reset();
    assert(first.weak_count() == 0);
    auto cyclic = rusty::Arc<Cyclic>::new_cyclic([](const auto& weak) {
        assert(weak.weak_count() == 0);
        auto copy = weak.clone();
        assert(copy.weak_count() == 0);
        return Cyclic{std::move(copy), 19};
    });
    assert(cyclic.weak_count() == 1);
    assert(cyclic->self.weak_count() == 1);
}

struct ConstructionFailure {};

static void cyclic_callback_failure_releases_implicit_weak() {
    rusty::sync::Weak<int> escaped;
    int callbacks = 0;
    try {
        (void)rusty::Arc<int>::new_cyclic([&](const auto& weak) -> int {
            ++callbacks;
            escaped = weak.clone();
            throw ConstructionFailure{};
        });
        assert(false);
    } catch (const ConstructionFailure&) {
        assert(callbacks == 1);
    }
    assert(escaped.expired());
    assert(escaped.strong_count() == 0);
    assert(escaped.upgrade().is_none());
    escaped.reset();
    // Also cover failure without an escaped owner: the implicit weak is the
    // only remaining owner after the borrowed callback handle is destroyed.
    try {
        (void)rusty::Arc<int>::new_cyclic([](const auto&) -> int {
            throw ConstructionFailure{};
        });
        assert(false);
    } catch (const ConstructionFailure&) {
    }
}

struct LiveMember {
    int* live;
    explicit LiveMember(int* count) : live(count) { ++*live; }
    ~LiveMember() { --*live; }
};

struct ThrowingPayload {
    LiveMember member;
    ThrowingPayload(const rusty::sync::Weak<ThrowingPayload>& weak,
                    rusty::sync::Weak<ThrowingPayload>& escaped, int* live)
        : member(live) {
        escaped = weak.clone();
        throw ConstructionFailure{};
    }
};

static void cyclic_payload_failure_destroys_members_and_allocation() {
    rusty::sync::Weak<ThrowingPayload> escaped;
    int live = 0;
    try {
        (void)rusty::Arc<ThrowingPayload>::new_cyclic([&](const auto& weak) {
            return ThrowingPayload(weak, escaped, &live);
        });
        assert(false);
    } catch (const ConstructionFailure&) {
    }
    assert(live == 0);
    assert(escaped.expired());
    assert(escaped.strong_count() == 0);
    assert(escaped.upgrade().is_none());
    escaped.reset();
}

int main() {
    weak_owners_exclude_mutation();
    cyclic_and_expired_weak_owners();
    weak_assignment_preserves_counts();
    concurrent_downgrade_upgrade_never_grants_mutation();
    weak_count_requires_a_live_strong_owner();
    cyclic_callback_failure_releases_implicit_weak();
    cyclic_payload_failure_destroys_members_and_allocation();
    std::puts("Arc uniqueness: weak, cyclic, assignment, concurrent handoff and exception cleanup passed");
    return 0;
}
