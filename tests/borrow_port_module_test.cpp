// Smoke test for the transpiled borrow_port. Phase B/C level —
// proves the library links and Cow<int>'s default factory works.
// The actual Cow<str>/Cow<[u8]> instantiations require core::str /
// core::slice impls of ToOwned which live in other ports — for
// borrow_port alone, the int instantiation is the only end-to-end
// path that exercises the Phase 3a/3b trait machinery.

import borrow_port;

#include <cassert>
#include <cstdint>
#include <cstdio>

int main() {
    using ::borrow_port::Cow;
    using ::borrow_port::Cow_Borrowed;
    using ::borrow_port::Cow_Owned;

    int x = 42;
    Cow<int> borrowed = Cow<int>::Borrowed(x);
    Cow<int> owned = Cow<int>::Owned(99);

    assert(Cow<int>::is_borrowed(borrowed));
    assert(Cow<int>::is_owned(owned));
    assert(!Cow<int>::is_owned(borrowed));
    assert(!Cow<int>::is_borrowed(owned));

    std::printf("borrow_port module smoke OK: Cow<int>::Borrowed/Owned + is_borrowed/is_owned\n");
    return 0;
}
