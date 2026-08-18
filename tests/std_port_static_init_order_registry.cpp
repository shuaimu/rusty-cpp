// Module-side half of tests/std_port_static_init_order_test.cpp.
//
// THIS is the TU that `import std_port;`. The driver TU deliberately does not,
// so its own namespace-scope initializers are NOT ordered after the hashbrown /
// std_port module initializers — which is exactly the shape that crashed in
// mako (`src/deptran/procedure.cc`'s `__cxx_global_var_init` reaching a
// hashbrown-backed registry through a plain function declaration).
//
// See the driver for the full description of the bug.

import std_port;

#include <cstddef>

template <typename K, typename V>
using HMap = ::std_port::collections::hash::map::HashMap<K, V>;

template <typename T>
using HSet = ::std_port::collections::hash::set::HashSet<T>;

// Function-local statics: the containers themselves are immune to
// initialization order. Only the transpiled table's own statics are at issue.
static HMap<int, int>& registry() {
    static HMap<int, int> m = HMap<int, int>::new_();
    return m;
}

static HSet<int>& seen() {
    static HSet<int> s = HSet<int>::new_();
    return s;
}

void repro_register(int key, int value) {
    registry().insert(key, value);
    seen().insert(key);
}

std::size_t repro_len() { return registry().len(); }

std::size_t repro_seen_len() { return seen().len(); }

bool repro_lookup(int key, int* out) {
    auto got = registry().get(key);
    if (got.is_none()) {
        return false;
    }
    *out = got.unwrap();
    return true;
}

bool repro_seen_contains(int key) { return seen().contains(key); }
