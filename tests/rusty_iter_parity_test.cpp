// Parity test: rusty's hand-written iterator adapters must match Rust's own.
//
// Reads the checked-in golden fixture (tests/iter_parity/golden.txt, produced by
// tests/iter_parity/gen_golden.rs against the real Rust toolchain) and, for each
// case id, reproduces the value through rusty's iterators and compares. Rust is
// the oracle; any divergence — or a case present on only one side — fails.
//
// The emitted-by-the-transpiler rusty API is used directly: free functions
// `rusty::map/filter/chain/take/skip/rev/filter_map/fold` plus the `.step_by()`
// / `.sum()` / `.count()` methods, over `rusty::range_inclusive(a, b)` (`a..=b`).
#include "../include/rusty/rusty.hpp"

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <functional>
#include <map>
#include <sstream>
#include <string>

namespace {

// Comma-join an iterator's items without depending on Vec/Option iteration:
// fold the items into a string (rusty::fold is the same primitive the
// transpiler emits for `.fold(...)`).
template <class It>
std::string seq(It&& it) {
    return rusty::fold(std::forward<It>(it), std::string{},
                       [](std::string acc, auto&& x) {
                           if (!acc.empty()) acc.push_back(',');
                           acc += std::to_string(
                               static_cast<int64_t>(rusty::detail::deref_if_pointer_like(x)));
                           return acc;
                       });
}

auto r(int64_t a, int64_t b) { return rusty::range_inclusive(a, b); }

// case_id -> reproduced value (must equal the Rust golden line `<id>|<value>`).
std::map<std::string, std::function<std::string()>> cases() {
    std::map<std::string, std::function<std::string()>> m;
    m["map"] = [] { return seq(rusty::map(r(1, 5), [](auto&& x) { return x * 2; })); };
    m["filter"] = [] {
        return seq(rusty::filter(r(1, 10), [](auto&& x) { return x % 2 == 0; }));
    };
    m["chain"] = [] { return seq(rusty::chain(r(1, 3), r(4, 6))); };
    m["take"] = [] { return seq(rusty::take(r(1, 100), 4)); };
    m["skip"] = [] { return seq(rusty::skip(r(1, 10), 7)); };
    m["rev"] = [] { return seq(rusty::rev(r(1, 5))); };
    m["filter_map"] = [] {
        return seq(rusty::filter_map(r(1, 6), [](auto&& x) {
            return x % 2 == 0 ? rusty::Some<int64_t>(x * 10) : rusty::None;
        }));
    };
    m["map_filter_chain"] = [] {
        return seq(rusty::chain(rusty::map(r(1, 4), [](auto&& x) { return x * x; }),
                                rusty::filter(r(1, 3), [](auto&& x) { return x % 2 == 1; })));
    };
    m["fold"] = [] {
        return std::to_string(
            rusty::fold(r(1, 5), int64_t{0}, [](int64_t a, auto&& x) { return a + x; }));
    };
    m["count"] = [] {
        return std::to_string(static_cast<int64_t>(
            rusty::count(rusty::filter(r(1, 10), [](auto&& x) { return x % 3 == 0; }))));
    };
    return m;
}

}  // namespace

int main(int argc, char** argv) {
    if (argc < 2) {
        std::fprintf(stderr, "usage: %s <golden.txt>\n", argv[0]);
        return 2;
    }
    std::ifstream in(argv[1]);
    if (!in) {
        std::fprintf(stderr, "cannot open golden: %s\n", argv[1]);
        return 2;
    }
    auto repro = cases();
    int failures = 0;
    int checked = 0;
    std::map<std::string, bool> seen;
    std::string line;
    while (std::getline(in, line)) {
        if (line.empty()) continue;
        auto bar = line.find('|');
        if (bar == std::string::npos) continue;
        std::string id = line.substr(0, bar);
        std::string expected = line.substr(bar + 1);
        seen[id] = true;
        auto it = repro.find(id);
        if (it == repro.end()) {
            std::fprintf(stderr, "MISSING C++ repro for case '%s'\n", id.c_str());
            ++failures;
            continue;
        }
        std::string got = it->second();
        ++checked;
        if (got != expected) {
            std::fprintf(stderr, "MISMATCH %s: rust='%s' rusty='%s'\n", id.c_str(),
                         expected.c_str(), got.c_str());
            ++failures;
        }
    }
    for (auto& [id, _] : repro) {
        if (!seen.count(id)) {
            std::fprintf(stderr, "C++ case '%s' not in golden\n", id.c_str());
            ++failures;
        }
    }
    std::printf("iter parity: %d checked, %d failures\n", checked, failures);
    return failures == 0 ? 0 : 1;
}
