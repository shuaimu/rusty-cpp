// Demonstrates compositional safety:
// struct { Rc<T> } is automatically rejected (no marker)

#include "rusty/sync/mpsc.hpp"
#include "rusty/rc.hpp"

struct ContainsRc {
    rusty::Rc<int> data;
};

int main() {
    // This should FAIL to compile:
    // ContainsRc is NOT marked as Send (lacks static constexpr bool is_send = true)
    // Even though it's technically movable
    auto [tx, rx] = rusty::sync::mpsc::channel<ContainsRc>();

    return 0;
}
