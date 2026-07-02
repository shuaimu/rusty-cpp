#include <rusty/option.hpp>

// @safe
int main() {
    rusty::Option<int> opt = rusty::Some<int>(42);

    // Mutable borrow
    auto mut_ref = opt.as_mut();

    // Immutable borrow - should ERROR
    auto ref_opt = opt.as_ref();

    return 0;
}