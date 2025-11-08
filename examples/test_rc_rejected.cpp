// This file should FAIL to compile
// Demonstrates that Rc<T> is correctly rejected

#include "rusty/sync/mpsc.hpp"
#include "rusty/rc.hpp"

int main() {
    // This should cause a compile error:
    // "Channel type T must be Send (marked explicitly)"
    auto [tx, rx] = rusty::sync::mpsc::channel<rusty::Rc<int>>();

    return 0;
}
