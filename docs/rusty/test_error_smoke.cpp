// Instantiation + behavior smoke test for the std::error slice of module
// `rusty`. NOTE: rusty::to_string and the Formatter shim live in the module's
// GLOBAL FRAGMENT (unnameable from importers), so Display output cannot be
// asserted here — the explicit instantiation below gates that all Report<E>
// members (fmt/fmt_singleline/fmt_multiline/backtrace) at least compile.
import rusty;
#include <cassert>
#include <cstdio>
#include <string>
#include <string_view>

struct MyErr {
    std::string to_string() const { return "MyErr is here!"; }
};

// Full-surface compile gate: instantiates every non-template member,
// including the fmt paths (needs patches e2/e4/e5/e6).
template struct error::Report<MyErr>;

int main() {
    // Report::new_ -> From::from; builder setters return by value
    auto rep = error::Report<MyErr>::new_(MyErr{});
    assert(!rep.show_backtrace_field && !rep.pretty_field);
    auto rep2 = error::Report<MyErr>::new_(MyErr{}).pretty(true).show_backtrace(true);
    assert(rep2.show_backtrace_field && rep2.pretty_field);

    // Error trait default methods via UFCS dispatchers
    MyErr e;
    auto src = Error_::source(e);
    assert(src.is_none());
    auto d = Error_::description(e);
    assert(std::string_view(d) == "description() is deprecated; use Display");
    auto c = Error_::cause(e);
    assert(c.is_none());

    // Stubbed nightly Request/provide surface: always "nothing provided"
    auto rr = error::request_ref<error::Backtrace>(e);
    assert(rr.is_none());
    auto rv = error::request_value<int>(e);
    assert(rv.is_none());

    // Report::backtrace() rides the stub -> None
    assert(rep2.backtrace().is_none());

    std::printf("rusty (std) error-slice smoke OK: Report instantiation + Error defaults + request stubs\n");
    return 0;
}
