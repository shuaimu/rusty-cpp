// Bug: Const propagation incorrectly flags Cell::set() calls
// See: docs/bug_report_cell_const_propagation.md
//
// Cell::set() is @safe with an internal @unsafe block.
// Calling it from a const method should NOT require @unsafe at the call site.

#include <rusty/cell.hpp>

// @safe
class Counter {
private:
    rusty::Cell<int> count_{0};

public:
    // @safe - This should NOT trigger a const propagation violation
    // Cell::set() is @safe with internal @unsafe block that handles the mutation
    void increment() const {
        count_.set(count_.get() + 1);  // BUG: This line triggers false positive
    }

    // @safe
    int get() const {
        return count_.get();
    }
};

// @safe
int main() {
    Counter c;
    c.increment();
    c.increment();
    c.increment();
    return c.get();  // Should return 3
}
