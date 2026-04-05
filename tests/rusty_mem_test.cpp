// Tests for rusty::mem helpers used by transpiled std::mem lowering
#include "../include/rusty/mem.hpp"

#include <array>
#include <cassert>
#include <cstdio>
#include <type_traits>
#include <utility>

namespace {

struct DropCounter {
    int* drops;

    explicit DropCounter(int* drops_in) : drops(drops_in) {}
    DropCounter(const DropCounter&) = delete;
    DropCounter& operator=(const DropCounter&) = delete;

    DropCounter(DropCounter&& other) noexcept : drops(other.drops) {
        other.drops = nullptr;
    }

    DropCounter& operator=(DropCounter&& other) = delete;

    ~DropCounter() {
        if (drops != nullptr) {
            ++(*drops);
        }
    }
};

void test_manually_drop_new_pointer_access_shape() {
    std::printf("test_manually_drop_new_pointer_access_shape: ");
    auto wrapped = rusty::mem::manually_drop_new(std::array<int, 3>{1, 2, 3});
    static_assert(
        std::is_same_v<decltype(wrapped), rusty::mem::ManuallyDrop<std::array<int, 3>>>);
    assert((*wrapped)[0] == 1);
    (*wrapped)[1] = 42;
    assert(wrapped.as_ptr()->at(1) == 42);
    assert(wrapped.as_mut_ptr()->at(2) == 3);
    std::printf("PASS\n");
}

void test_manually_drop_suppresses_inner_drop() {
    std::printf("test_manually_drop_suppresses_inner_drop: ");
    int drops = 0;
    {
        DropCounter value(&drops);
        auto wrapped = rusty::mem::ManuallyDrop<DropCounter>::new_(std::move(value));
        (void)wrapped.as_ptr();
    }
    assert(drops == 0);
    std::printf("PASS\n");
}

} // namespace

int main() {
    test_manually_drop_new_pointer_access_shape();
    test_manually_drop_suppresses_inner_drop();
    return 0;
}
