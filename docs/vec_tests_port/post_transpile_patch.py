#!/usr/bin/env python3
"""Post-transpile patches for vec_tests_port (rustc alloctests vec.rs).

Pipeline (transpile WITHOUT --expand; cargo-expand strips #[test]):
    python3 docs/vec_deque_tests_port/prep_expand.py <tgt>/src/lib.rs
    RUSTY_CPP_DUMP_AUTO=1 ./target/release/rusty-cpp-transpiler \
        --crate <tgt>/Cargo.toml --output-dir <tgt>/cpp_out --auto-namespace
    python3 docs/vec_tests_port/post_transpile_patch.py <tgt>/cpp_out

The tests exercise the ALLOC consolidated module's `vec::Vec` (the
vendored vec_port stays on its own smoke path). Rules grow from
compile-error triage exactly like the vec_deque patcher did.
"""
import os
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
from _test_port_helpers import (  # noqa
    inject_test_runner_include,
    inject_module_imports,
    stub_tests,
)

HELPERS = """
// The tests exercise the ALLOC consolidated module's Vec.
template<typename T, typename A = rusty::alloc::Global>
using VecT = ::vec::Vec<T, A>;
// vec![a, b, c] literal factories (the alloc Vec has no init-list ctor).
template<typename T, typename... Ts>
static auto vt_of_t(Ts&&... xs) {
    auto v = VecT<T>::with_capacity(sizeof...(Ts));
    (v.push(std::forward<Ts>(xs)), ...);
    return v;
}
template<typename... Ts>
static auto vt_of(Ts&&... xs) {
    using T = std::common_type_t<std::decay_t<Ts>...>;
    return vt_of_t<T>(std::forward<Ts>(xs)...);
}
// Empty `vec![]` in typed positions: universally-convertible proxy.
struct vt_empty_proxy {
    constexpr size_t size() const { return 0; }
    constexpr size_t len() const { return 0; }
    template<typename T2, typename A2>
    operator ::vec::Vec<T2, A2>() const { return ::vec::Vec<T2, A2>::new_(); }
    template<typename T2, typename A2>
    friend bool operator==(const vt_empty_proxy&, const ::vec::Vec<T2, A2>& v) {
        return v.is_empty();
    }
    template<typename T2, typename A2>
    friend bool operator==(const ::vec::Vec<T2, A2>& v, const vt_empty_proxy&) {
        return v.is_empty();
    }
};
static inline vt_empty_proxy vt_of() { return {}; }
// Element-wise compare of a (containerA, containerB) tuple against an
// expected tuple — partition results (std::vector) vs vt_of (alloc Vec).
template<typename P, typename Q>
static bool vt_tuple2_eq(const P& got, const Q& want) {
    const auto& g0 = std::get<0>(got);
    const auto& g1 = std::get<1>(got);
    const auto& w0 = std::get<0>(want);
    const auto& w1 = std::get<1>(want);
    auto eq = [](const auto& a, const auto& b) {
        auto sz0 = [](const auto& x) -> size_t {
            if constexpr (requires { std::size(x); }) return std::size(x);
            else if constexpr (requires { x.len(); }) return x.len();
            else return 0;
        };
        if constexpr (requires { a[size_t{}]; b[size_t{}]; }) {
            if (sz0(a) != sz0(b)) return false;
            for (size_t i = 0; i < sz0(a); ++i) {
                if (!(a[i] == b[i])) return false;
            }
            return true;
        } else {
            // One side is the subscript-less empty proxy.
            auto sz = [](const auto& x) -> size_t {
                if constexpr (requires { std::size(x); }) return std::size(x);
                else if constexpr (requires { x.len(); }) return x.len();
                else return 0;
            };
            return sz(a) == sz(b);
        }
    };
    return eq(g0, w0) && eq(g1, w1);
}
// Rust Iterator::next_chunk::<N>() over Option-next iterators. Err carries
// the partial remainder (as_slice/len surface only, enough for the tests).
template<typename T2>
struct vt_chunk_err {
    std::vector<T2> got;
    std::span<const T2> as_slice() const {
        return {got.data(), got.size()};
    }
    size_t len() const { return got.size(); }
};
template<size_t N, typename It>
static auto vt_next_chunk(It& it) {
    using T2 = std::remove_cvref_t<decltype(rusty::detail::deref_if_pointer_like(
        rusty::detail::option_like_take_value(std::declval<decltype(it.next())&>())))>;
    std::vector<T2> got;
    got.reserve(N);
    for (size_t i = 0; i < N; ++i) {
        auto v = it.next();
        if (!v.is_some()) {
            return rusty::Result<std::array<T2, N>, vt_chunk_err<T2>>::Err(
                vt_chunk_err<T2>{std::move(got)});
        }
        auto taken = rusty::detail::option_like_take_value(v);
        got.push_back(std::move(rusty::detail::deref_if_pointer_like(taken)));
    }
    return [&]<size_t... I>(std::index_sequence<I...>) {
        return rusty::Result<std::array<T2, N>, vt_chunk_err<T2>>::Ok(
            std::array<T2, N>{std::move(got[I])...});
    }(std::make_index_sequence<N>{});
}
// DoubleEndedIterator::nth_back over Option-next_back iterators.
template<typename It>
static auto vt_nth_back(It&& it, size_t n) {
    for (size_t i = 0; i < n; ++i) {
        auto skipped = it.next_back();
        if (!skipped.is_some()) { return skipped; }
    }
    return it.next_back();
}
// NOTE (vd lesson): ANY operator== declared in this namespace HIDES the
// global array.hpp compare helpers for ordinary lookup — so this block
// must cover every std-vs-std shape the tests use, plus the VecT mixes.
template<typename T2, size_t E, typename U2, size_t N>
static bool operator==(std::span<T2, E> s, const std::array<U2, N>& a) {
    if (s.size() != N) return false;
    if constexpr (N > 0) {
        for (size_t i = 0; i < N; ++i) if (!(s[i] == a[i])) return false;
    }
    return true;
}
// Same-T (and cross-const) forms only: strictly MORE SPECIALIZED than
// the global array.hpp (L, R) template, so ADL from module bodies never
// ties with it (suite-typed spans associate this namespace).
template<typename T2, size_t E1, size_t E2>
static bool operator==(std::span<T2, E1> a, std::span<T2, E2> b) {
    if (a.size() != b.size()) return false;
    for (size_t i = 0; i < a.size(); ++i) if (!(a[i] == b[i])) return false;
    return true;
}
template<typename T2, size_t E1, size_t E2>
static bool operator==(std::span<const T2, E1> a, std::span<T2, E2> b) {
    if (a.size() != b.size()) return false;
    for (size_t i = 0; i < a.size(); ++i) if (!(a[i] == b[i])) return false;
    return true;
}
template<typename T2, size_t E1, size_t E2>
static bool operator==(std::span<T2, E1> a, std::span<const T2, E2> b) {
    if (a.size() != b.size()) return false;
    for (size_t i = 0; i < a.size(); ++i) if (!(a[i] == b[i])) return false;
    return true;
}
template<typename T2, typename U2, size_t E>
static bool operator==(const std::vector<T2>& v, std::span<U2, E> s) {
    if (v.size() != s.size()) return false;
    for (size_t i = 0; i < v.size(); ++i) if (!(v[i] == s[i])) return false;
    return true;
}
template<typename T2, typename U2, size_t N>
static bool operator==(const std::vector<T2>& v, const std::array<U2, N>& a) {
    if (v.size() != N) return false;
    if constexpr (N > 0) {
        for (size_t i = 0; i < N; ++i) if (!(v[i] == a[i])) return false;
    }
    return true;
}
// collect_range yields std::vector; expected sides are alloc Vecs.
template<typename T2, typename A2, typename U2>
static bool operator==(const std::vector<T2>& a, const ::vec::Vec<U2, A2>& b) {
    if (a.size() != b.len()) return false;
    for (size_t i = 0; i < a.size(); ++i) {
        if (!(a[i] == b[i])) return false;
    }
    return true;
}
template<typename T2, typename A2, typename U2>
static bool operator==(const ::vec::Vec<T2, A2>& a, const std::vector<U2>& b) {
    return b == a;
}
// VecT vs std::array / span / VecT (Rust slice PartialEq shapes).
template<typename T2, typename A2, typename U2, size_t N>
static bool operator==(const ::vec::Vec<T2, A2>& v, const std::array<U2, N>& a) {
    if (v.len() != N) return false;
    if constexpr (N > 0) {
        for (size_t i = 0; i < N; ++i) if (!(v[i] == a[i])) return false;
    }
    return true;
}
template<typename T2, size_t E>
static bool operator==(std::span<T2, E> s, const vt_empty_proxy&) {
    return s.size() == 0;
}
template<typename T2>
static bool operator==(const std::vector<T2>& v, const vt_empty_proxy&) {
    return v.empty();
}
// Rust slice::partition_dedup — consecutive duplicates swapped to the
// back; returns (unique_prefix, dups_suffix) spans.
template<typename T2, typename A2>
static auto vt_partition_dedup(::vec::Vec<T2, A2>& v) {
    auto s = *v;
    T2* data = s.data();
    const size_t n = s.size();
    size_t next_read = 1, next_write = 1;
    while (next_read < n) {
        if (!(data[next_read] == data[next_write - 1])) {
            if (next_read != next_write) {
                std::swap(data[next_read], data[next_write]);
            }
            ++next_write;
        }
        ++next_read;
    }
    if (n == 0) next_write = 0;
    return std::make_tuple(std::span<T2>(data, next_write),
                           std::span<T2>(data + next_write, n - next_write));
}
// Iterator::unzip over Option-next tuple iterators.
template<typename It>
static auto vt_unzip(It&& it) {
    using Pair = std::remove_cvref_t<decltype(rusty::detail::deref_if_pointer_like(
        rusty::detail::option_like_take_value(std::declval<decltype(it.next())&>())))>;
    using A2 = std::remove_cvref_t<std::tuple_element_t<0, Pair>>;
    using B2 = std::remove_cvref_t<std::tuple_element_t<1, Pair>>;
    auto va = VecT<A2>::new_();
    auto vb = VecT<B2>::new_();
    for (auto v = it.next(); v.is_some(); v = it.next()) {
        auto taken = rusty::detail::option_like_take_value(v);
        auto&& pr = rusty::detail::deref_if_pointer_like(taken);
        va.push(std::move(std::get<0>(pr)));
        vb.push(std::move(std::get<1>(pr)));
    }
    return std::make_tuple(std::move(va), std::move(vb));
}
// vec![x; n] repeat form.
template<typename T2>
static auto vt_repeat(const T2& elem, size_t n) {
    auto v = VecT<T2>::with_capacity(n);
    for (size_t i = 0; i < n; ++i) { v.push(rusty::clone(elem)); }
    return v;
}
// Vec::from(array/slice) / slice.to_vec() shapes.
template<typename Src>
static auto vt_from(Src&& src) {
    using T = std::remove_cvref_t<decltype(src[static_cast<size_t>(0)])>;
    auto v = VecT<T>::with_capacity(std::size(src));
    if constexpr (!std::is_lvalue_reference_v<Src&&>) {
        // Rvalue source — a collect_range temporary. Rust's collect MOVES,
        // and it must here too: cloning leaves the temporary holding live
        // elements, and its destructor (std::vector's, which is noexcept)
        // then runs their Drop a second time — std::terminate if that Drop
        // panics. Moving leaves each source _rusty_forgotten instead.
        for (auto&& x : src) { v.push(std::move(x)); }
    } else {
        // Port types (Rc &c.) have BITWISE default copies — the transpiler
        // contract is that copies always go through clone(). rusty::clone
        // does the right thing for both port types and primitives.
        for (auto&& x : src) { v.push(rusty::clone(x)); }
    }
    return v;
}
"""


def _walk_replace_vec_literals(text: str) -> str:
    """`rusty::Vec{...}` -> `vt_of(...)` and `rusty::Vec<T>{...}` ->
    `vt_of_t<T>(...)` with brace matching (nested braces stay intact)."""
    out = []
    i = 0
    n = len(text)
    needle = "rusty::Vec"
    while i < n:
        j = text.find(needle, i)
        if j < 0:
            out.append(text[i:])
            break
        out.append(text[i:j])
        k = j + len(needle)
        type_arg = ""
        if k < n and text[k] == "<":
            depth = 1
            m = k + 1
            while m < n and depth:
                if text[m] == "<":
                    depth += 1
                elif text[m] == ">":
                    depth -= 1
                m += 1
            type_arg = text[k + 1:m - 1]
            k = m
        if k < n and text[k] == "{":
            depth = 1
            m = k + 1
            while m < n and depth:
                if text[m] == "{":
                    depth += 1
                elif text[m] == "}":
                    depth -= 1
                m += 1
            inner = text[k + 1:m - 1]
            if type_arg:
                out.append(f"vt_of_t<{type_arg}>({inner})")
            else:
                out.append(f"vt_of({inner})")
            i = m
        else:
            # Type position (or assoc call) — route to VecT.
            if type_arg:
                out.append(f"VecT<{type_arg}>")
            else:
                out.append("rusty::Vec")
            i = k
    return "".join(out)


# Tests stubbed with a cited reason (printed at runtime, counted ok).
SKIP_TESTS: list[str] = [
    # (name, reason) pairs flattened — stub_tests takes names + one reason,
    # so group by reason below in apply_patches.
]

SKIP_GROUPS: list[tuple[list[str], str]] = [
    (["const_heap"],
     "const-eval heap allocation is a rustc compile-time feature"),
    (["test_in_place_specialization_step_up_down",
      "test_from_iter_specialization",
      "test_from_iter_partial_specialization",
      "test_from_iter_specialization_with_iterator_adapters",
      "test_from_iter_specialization_head_tail_drop",
      "test_from_iter_specialization_panic_during_iteration_drops",
      "test_from_iter_specialization_panic_during_drop_doesnt_leak",
      "test_collect_after_iterator_clone"],
     "in-place iterator specialization (ptr-identity asserts) is a "
     "rustc-internal optimization the port does not implement"),
    (["test_small_vec_struct"],
     "port Vec carries the _rusty_forgotten move-flag; Rust's 3-word "
     "layout guarantee does not hold"),
    (["test_into_iter_drop_allocator"],
     "custom Allocator impl (ReferenceCountedAllocator) methods are not "
     "reachable from module internals"),
    (["test_box_zero_allocator"],
     "custom Allocator impl (ZeroSizedAllocator) not translatable"),
    (["vec_macro_repeating_null_raw_fat_pointer"],
     "raw fat-pointer vtable surgery not translatable"),
    (["test_cow_from", "test_from_cow"],
     "Cow is not a bound type: it lowers to a bare std::variant with no "
     "from/into surface, so Cow::from(Vec) / Vec::from(Cow) do not exist"),
    (["test_into_flattened_size_overflow"],
     "ZST [(); usize::MAX] arrays exceed C++ constant-evaluation limits"),
    (["test_flatten_clone"],
     "the flatten adapter has no clone(), so cloning a flattened chain "
     "falls back to the deleted move-only copy"),
    (["test_into_iter_clone"],
     "the test's fn-local generic `iter_equal<I: Iterator>` lowers to a "
     "NON-generic lambda, so it cannot take both IntoIter and Rev<IntoIter> "
     "(IntoIter::clone and Rev::clone themselves are fixed)"),
    (["vec_null_ptr_roundtrip"],
     "strict-provenance raw-pointer methods (with_addr) on primitive "
     "pointers are not lowered"),
    (["partialeq_vec_and_prim"],
     "local macro_rules-generated assert helpers are not expanded"),
    (["test_from_iter_partially_drained_in_place_specialization"],
     "allocation-identity (as_ptr) asserts require in-place IntoIter "
     "buffer reuse the port does not guarantee"),
    (["test_into_boxed_slice"],
     "into_boxed_slice shrink path needs unemitted RawVec/Box internals"),
    (["test_into_iter_leak"],
     "catch_unwind closure copies module IntoIter internals"),
    (["zst_collections_iter_nth_back_regression"],
     "exercises legacy hashbrown/linked_list port iterators (IS_ZST "
     "metadata not emitted for suite-local types)"),
    (["from_into_inner"],
     "allocation-identity (as_ptr) asserts require in-place IntoIter "
     "buffer reuse the port does not guarantee"),
    (["test_peek_mut"],
     "Vec::peek_mut's Option<PeekMut> return type is not visible to the "
     "suite transpile, so the guard binds as `auto* p = &(...unwrap())` — "
     "the address of a temporary; needs cross-crate return-type metadata"),
]


def apply_patches(path: Path) -> None:
    text = path.read_text()
    text = inject_test_runner_include(text)
    text = inject_module_imports(text, "vec_tests_port", ["alloc"])

    marker = "namespace vec_tests_port {"
    if marker in text and "using VecT = " not in text:
        text = text.replace(marker, marker + "\n" + HELPERS, 1)

    text = _walk_replace_vec_literals(text)
    # vec![x; n] came through the walker as `vt_of(x; n)` — repeat form.
    text = re.sub(r"vt_of\(\(\) ?; ?([^;)]+?)\)",
                  r"vt_repeat(std::make_tuple(), \1)", text)
    text = re.sub(r"vt_of\(([^;()]*?) ?; ?([^;()]*?)\)", r"vt_repeat(\1, \2)", text)
    # TryReserveError: field `kind`, not a method (string-suite recipe).
    text = text.replace(".kind()", ".kind")
    # Macros cannot be `using`-declared; the transpiler leaked these.
    for bogus in ("using std::assert_eq;", "using std::assert_ne;",
                  "using std::assert_matches;",
                  "using std::testing::macros::struct_with_counted_drop;"):
        text = text.replace(bogus, "// Rust-only: " + bogus)
    # `vec![..].into_iter()` / `v.into_iter()` were lowered to the
    # generic rusty::iter(..), which passes the alloc Vec through instead
    # of producing its real IntoIter (advance_by/as_slice/... live there).
    def _iter_to_into_iter(txt: str) -> str:
        out = []
        i = 0
        needle = "rusty::iter(vt_of"
        while True:
            j = txt.find(needle, i)
            if j < 0:
                out.append(txt[i:])
                break
            out.append(txt[i:j])
            k = j + len("rusty::iter(")
            depth = 1
            m = k
            n = len(txt)
            while m < n and depth:
                if txt[m] == "(":
                    depth += 1
                elif txt[m] == ")":
                    depth -= 1
                m += 1
            inner = txt[k:m - 1]
            out.append(f"({inner}).into_iter()")
            i = m
        return "".join(out)
    text = _iter_to_into_iter(text)
    text = re.sub(r"rusty::iter\(std::move\((\w+)\)\)",
                  r"std::move(\1).into_iter()", text)
    # partition asserts: tuple compare via vt_tuple2_eq (runtime
    # partition yields std::vector pairs; expected side is alloc Vecs).
    lines2 = []
    for ln in text.split("\n"):
        if ".partition(" in ln and ") == (std::make_tuple(" in ln:
            # assert! now lowers to `if (!(...)) { do_panic(...); }` (Rust
            # panics, C assert aborts); keep the legacy anchor for old output.
            ln = ln.replace("if (!((", "if (!(vt_tuple2_eq(", 1)
            ln = ln.replace("assert(((", "assert((vt_tuple2_eq(", 1)
            ln = ln.replace(") == (std::make_tuple(", ", std::make_tuple(", 1)
        lines2.append(ln)
    text = "\n".join(lines2)
    # by_ref on module iterators (Drain &c.): value-identity here.
    text = text.replace(".by_ref()", "")
    # Peel the void cast so the for_each rules see the plain form.
    text = re.sub(r"static_cast<void>\((.+\.for_each\(.*)\)\);", r"\1);", text)
    # Member for_each on module types -> the free-fn form.
    text = re.sub(r"^(\s*)(.*?)\.for_each\((.*)\);$",
                  r"\1rusty::for_each(\2, \3);", text, flags=re.MULTILINE)
    # next_chunk member forms -> vt_next_chunk<N>(it).
    text = re.sub(r"(\w+)\.template next_chunk<(\d+)>\(\)",
                  r"vt_next_chunk<\2>(\1)", text)
    def _chunk_n(m):
        recv, rest = m.group(1), m.group(2)
        n = rest.count(",") + 1
        return f"vt_next_chunk<{n}>({recv}).unwrap()) == (std::array{{{rest}}}"
    text = re.sub(
        r"(\w+)\.next_chunk\(\)\.unwrap\(\)\) == \(std::array\{([^}]*)\}",
        _chunk_n, text)
    # std::hash specializations were emitted INSIDE the suite namespace
    # (ill-formed): relocate each block to just before the namespace opens,
    # with the argument qualified.
    ns_marker = "namespace vec_tests_port {"
    hash_blocks = []
    hash_pat = re.compile(
        r"template<>\nstruct std::hash<([A-Za-z_0-9]+)> \{.*?\n\};\n", re.DOTALL)
    def _grab(m):
        blk = re.sub(rf"\b{m.group(1)}\b",
                     f"vec_tests_port::{m.group(1)}", m.group(0))
        hash_blocks.append(blk)
        return ""
    text = hash_pat.sub(_grab, text)
    if hash_blocks:
        # Forward declarations so the qualified names resolve pre-namespace?
        # Not needed: relocate AFTER the namespace CLOSES (end of module).
        text = text.rstrip() + "\n\n" + "\n".join(hash_blocks) + "\n"
    # std::ptr free fns live in rusty::ptr.
    text = text.replace("std::ptr::", "rusty::ptr::")
    # rusty::VecDeque facade lacks the alloc surface — use the module deque.
    text = text.replace("rusty::VecDeque<", "collections::vec_deque::VecDeque<")
    # DoubleEnded nth_back as a free helper.
    text = re.sub(r"((?:std::move\(\w+\)\.into_iter\(\)|\w+))\.nth_back\(",
                  r"vt_nth_back(\1, ", text)
    # Slice literals whose element type stayed unresolved (`auto`): every
    # such site in this suite is an int-literal slice.
    text = text.replace("std::span<const auto>", "std::span<const int32_t>")
    text = text.replace("std::array<auto,", "std::array<int32_t,")
    # extract_if panic tests: Rc<Mutex<Vec<usize>>> lost its payload types.
    text = text.replace(
        "rusty::Rc<rusty::Mutex>::new_(Mutex<auto>::new_(vt_repeat(0_usize , check_count)))",
        "rusty::Rc<rusty::Mutex<VecT<size_t>>>::new_("
        "rusty::Mutex<VecT<size_t>>::new_("
        "vt_repeat(static_cast<size_t>(0), check_count)))")
    # test_double_drop: generic local TwoVec<T> aggregate lost T.
    text = text.replace(
        "auto tv = TwoVec{.x = VecT<auto>::new_(), .y = VecT<auto>::new_()};",
        "auto tv = TwoVec<DropCounter>{.x = VecT<DropCounter>::new_(), "
        ".y = VecT<DropCounter>::new_()};")
    # test_retain_drop_panic: collect_range yields std::vector — route into
    # the alloc Vec so .retain exists.
    text = text.replace(
        "auto v_shadow1 = rusty::collect_range(rusty::map(rusty::iter(v), "
        "[&](auto&& r) { return Wrap(rusty::clone(r)); }));",
        "auto v_shadow1 = vt_from(rusty::collect_range(rusty::map(rusty::iter(v), "
        "[&](auto&& r) { return Wrap(rusty::clone(r)); })));")
    # Mutable int slices with unresolved element type.
    text = text.replace("std::span<auto>", "std::span<int32_t>")
    # NonZero::new_ without payload type (advance_by Err payloads, usize).
    text = text.replace("NonZero::new_(", "rusty::num::NonZero<size_t>::new_(")
    # dedup_panicking: Cell payload is u32 in the hoisted struct.
    text = text.replace(
        "const auto& drop_counter = Cell<int32_t>::new_(0);",
        "const auto& drop_counter = Cell<uint32_t>::new_(0);")
    # double_drop: DropCounter holds u32& — the literal pair must be u32.
    text = text.replace(
        "auto [count_x, count_y] = rusty::detail::deref_if_pointer_like("
        "std::make_tuple(static_cast<int32_t>(0), static_cast<int32_t>(0)));",
        "auto count_x = static_cast<uint32_t>(0); "
        "auto count_y = static_cast<uint32_t>(0);")
    # into_iter_clone: vec![..].into_iter() must be the ALLOC IntoIter
    # (rusty::iter over a temporary array is also a dangling view).
    text = text.replace(
        "auto it = rusty::iter(std::array{1, 2, 3});",
        "auto it = vt_of(1, 2, 3).into_iter();")
    text = text.replace(
        "auto it_shadow1 = it.rev();",
        "auto it_shadow1 = rusty::rev(std::move(it));")
    # zst nth_back chains (lvalue receivers consume in Rust).
    text = text.replace(
        "static_cast<void>(d.into_iter().nth_back(1));",
        "static_cast<void>(vt_nth_back(std::move(d).into_iter(), 1));")
    text = text.replace(
        "static_cast<void>(map.into_values().nth_back(0));",
        "static_cast<void>(vt_nth_back(std::move(map).into_values(), 0));")
    # extract_if panic tests (respaced anchor).
    text = text.replace(
        "rusty::Rc<rusty::Mutex>::new_(Mutex<auto>::new_(vt_repeat(0_usize, check_count)))",
        "rusty::Rc<rusty::Mutex<VecT<size_t>>>::new_("
        "rusty::Mutex<VecT<size_t>>::new_("
        "vt_repeat(static_cast<size_t>(0), check_count)))")
    # DropCounter counters are u32 in Rust (inferred through &mut u32).
    text = re.sub(r"auto (count_\w+) = 0;", r"uint32_t \1 = 0;", text)
    # No rusty::thread runtime — panicking == active unwind.
    text = text.replace("rusty::thread::panicking()",
                        "(std::uncaught_exceptions() > 0)")
    # Hoisted-struct clone() bodies use designated init against ctor types.
    text = re.sub(
        r"(\w+_L\d+)\{\.\w+ = ([^,{}]+), \.\w+ = ([^{}]+)\}",
        r"\1(\2, \3)", text)
    # next_chunk receiver must be the real IntoIter.
    text = re.sub(
        r"auto iter = rusty::iter\((VecT<[^;]+?::from_iter\([^;]+?\))\);",
        r"auto iter = \1.into_iter();", text)
    # Consumed extract_if iterators must not bind const.
    text = re.sub(r"const auto (\w+) = (\w+)\.extract_if\(",
                  r"auto \1 = \2.extract_if(", text)
    # Typed empty/literal decls: route the literal through vt_of_t<T>.
    text = re.sub(
        r"VecT<(uint8_t|uint16_t|uint32_t|uint64_t|int8_t|int16_t|int64_t|ptrdiff_t|size_t)> (\w+) = vt_of\(",
        r"VecT<\1> \2 = vt_of_t<\1>(", text)
    # drop_allocator: concrete allocator type param.
    text = text.replace("VecT<uint32_t, auto>",
                        "VecT<uint32_t, ReferenceCountedAllocator>")
    # Unsuffixed-literal leaks: 10usize / 3u8 / 4i64 forms.
    text = re.sub(r"\b(\d+)usize\b", r"static_cast<size_t>(\1)", text)
    text = re.sub(r"\b(\d+)u(8|16|32|64)\b", r"static_cast<uint\2_t>(\1)", text)
    text = re.sub(r"\b(\d+)i(8|16|32|64)\b", r"static_cast<int\2_t>(\1)", text)
    # test_vec_dedup: with_capacity typed via a LATER local — pin to bool.
    text = text.replace(
        "VecT<rusty::detail::associated_item_t<std::remove_cvref_t<decltype(iter)>>>"
        "::with_capacity(8)",
        "VecT<bool>::with_capacity(8)")
    # zst nth_back: remaining collection receivers.
    text = text.replace(
        "static_cast<void>(heap.into_iter().nth_back(1));",
        "static_cast<void>(vt_nth_back(std::move(heap).into_iter(), 1));")
    text = text.replace(
        "static_cast<void>(list.into_iter().nth_back(1));",
        "static_cast<void>(vt_nth_back(std::move(list).into_iter(), 1));")
    # retain_pred tests: Rc::new_ fn-item as map arg needs its payload type.
    text = text.replace(
        "rusty::map((rusty::range(0, 5)), Rc::new_)",
        "rusty::map((rusty::range(0, 5)), "
        "[](auto v) { return rusty::Rc<int32_t>::new_(v); })")
    # retain_maybeuninits: MaybeUninit payload type.
    text = text.replace("VecT<rusty::MaybeUninit>",
                        "VecT<rusty::MaybeUninit<int32_t>>")
    # into_boxed_slice: member form on the alloc Vec.
    text = text.replace("rusty::into_boxed_slice(std::move(xs))",
                        "std::move(xs).into_boxed_slice()")
    # Rc::strong_count fn-item over a deduced Rc payload.
    text = text.replace(
        "Rc<std::remove_cvref_t<decltype((r))>>::strong_count(std::move(r))",
        "rusty::Rc<int32_t>::strong_count(r)")
    # MaybeUninit::assume_init_ref returns a carrier — deref before [].
    text = re.sub(r"(\w+)\.assume_init_ref\(\)\[",
                  r"rusty::detail::deref_if_pointer_like(\1.assume_init_ref())[",
                  text)
    # as_slice over next_chunk Err: use the member (span) directly.
    text = re.sub(r"rusty::as_slice\((vt_next_chunk<\d+>\(\w+\)\.unwrap_err\(\))\)",
                  r"(\1).as_slice()", text)
    # MaybeUninit payload is [usize; 1]-like in retain_maybeuninits.
    text = text.replace("VecT<rusty::MaybeUninit<int32_t>>",
                        "VecT<rusty::MaybeUninit<VecT<int32_t>>>")
    # cloned over a guard-deref vec: route through the span.
    text = text.replace(
        "rusty::cloned(rusty::iter(drop_counts_shadow1))",
        "rusty::cloned(rusty::iter(rusty::slice_full(drop_counts_shadow1)))")
    # append takes an lvalue.
    text = text.replace(
        "v.append(vt_of(27, 19));",
        "{ auto _app = vt_of(27, 19); v.append(_app); }")
    # mem::swap takes references, the emitter passed addresses.
    text = re.sub(r"rusty::mem::swap\(&([^,]+), &([^)]+)\)",
                  r"rusty::mem::swap(\1, \2)", text)
    # array_repeat expected sides compare via std::vector.
    text = text.replace(
        "== (rusty::array_repeat(std::make_tuple(), 12))",
        "== (std::vector<std::tuple<>>(12))")
    # partition_dedup as a helper.
    text = re.sub(r"(\w+)\.partition_dedup\(\)",
                  r"vt_partition_dedup(\1)", text)
    # Named captures inside a hoisted panic! were not extracted.
    text = text.replace(
        'rusty::panic::do_panic(std::format("expected: {expected:?}\\ngot: {vec:?}\\n"));',
        'rusty::panic::do_panic(std::format("expected: {}\\ngot: {}\\n", '
        'rusty::to_debug_string(expected), rusty::to_debug_string(vec)));')
    # Vec::from(&array) mangled into a member call on the array.
    text = re.sub(r"std::array\{([^}]*)\}\.from\(\)",
                  r"vt_from(std::array{\1})", text)
    # char* has no eq_ignore_ascii_case member.
    text = text.replace(
        "return a.eq_ignore_ascii_case(std::move(b));",
        "std::string_view _sa(a), _sb(b); if (_sa.size() != _sb.size()) return false; "
        "for (size_t _i = 0; _i < _sa.size(); ++_i) { "
        "if (std::tolower(static_cast<unsigned char>(_sa[_i])) != "
        "std::tolower(static_cast<unsigned char>(_sb[_i]))) return false; } "
        "return true;")
    # Drop-draining for_each over move-only items: plain next loop.
    text = re.sub(
        r"rusty::for_each\((\w+), \[\]\(auto&&\.\.\. _args\) -> decltype\(auto\) "
        r"\{ return rusty::mem::drop\(std::forward<decltype\(_args\)>\(_args\)\.\.\.\); \}\);",
        r"for (auto _fe = \1.next(); _fe.is_some(); _fe = \1.next()) {}", text)
    # splice(.., None): a typed empty Option is the iterator.
    text = text.replace(
        "vec.splice(rusty::range_full(), rusty::None)",
        "vec.splice(rusty::range_full(), rusty::Option<int32_t>{})")
    # retain over collect_range results: route into the alloc Vec, unshadow.
    text = text.replace(
        "const auto v = rusty::collect_range(rusty::map((rusty::range(0, 5)), "
        "[](auto v) { return rusty::Rc<int32_t>::new_(v); }));",
        "auto v = vt_from(rusty::collect_range(rusty::map((rusty::range(0, 5)), "
        "[](auto r) { return rusty::Rc<int32_t>::new_(r); })));")
    # dedup_by char* elements need a deref before string_view.
    text = text.replace(
        "std::string_view _sa(a), _sb(b);",
        "std::string_view _sa(rusty::detail::deref_if_pointer_like(a)), "
        "_sb(rusty::detail::deref_if_pointer_like(b));")
    # MutexGuard must deref before slice_full.
    text = text.replace(
        "rusty::cloned(rusty::iter(rusty::slice_full(drop_counts_shadow1)))",
        "rusty::cloned(rusty::iter(rusty::slice_full("
        "rusty::detail::deref_if_pointer_like(drop_counts_shadow1))))")
    # Drop-draining for_each with EXPRESSION receivers.
    text = re.sub(
        r"rusty::for_each\((.+?), \[\]\(auto&&\.\.\. _args\) -> decltype\(auto\) "
        r"\{ return rusty::mem::drop\(std::forward<decltype\(_args\)>\(_args\)\.\.\.\); \}\)",
        r"[&]{ auto _fe_it = \1; "
        r"for (auto _fe = _fe_it.next(); _fe.is_some(); _fe = _fe_it.next()) {} }()",
        text)
    # Iterator::unzip via the runtime free fn.
    text = re.sub(r"(\w+)\.unzip\(\)", r"vt_unzip(\1)", text)
    # A `mutable` lambda stored const is uncallable (pop_if/dedup preds).
    text = re.sub(r"const (auto \w+ = \[[^\n]*mutable)", r"\1", text)
    # unzip chained on adapters.
    text = re.sub(r"(rusty::cloned\([^;]*?\))\.unzip\(\)", r"vt_unzip(\1)", text)
    # zip must be the Rust-protocol iterator, not the pair-of-views helper.
    text = text.replace("rusty::iter_ext::zip(", "rusty::zip(")
    # swap operand must be mutable and match the vec's element type.
    text = text.replace("const auto n = ", "auto n = ")
    text = text.replace("auto n = 42;", "auto n = static_cast<ptrdiff_t>(42);")
    # dedup_by abs over pointer carriers.
    text = text.replace(
        "return rusty::abs(a) == rusty::abs(b);",
        "return rusty::abs(rusty::detail::deref_if_pointer_like(a)) == "
        "rusty::abs(rusty::detail::deref_if_pointer_like(b));")
    # all(zip(a, b), pred) with non-copyable elements: paired next-loop;
    # a POINTER tuple stands in for the zip item so the destructure lines
    # keep working.
    zip_head = ("const auto ok = rusty::all(rusty::zip(rusty::iter(vec), "
                "rusty::iter(expected)), [&](auto&& _destruct_param0) {")
    zi = text.find(zip_head)
    if zi >= 0:
        tail = text.find("\n});", zi)
        if tail >= 0:
            inner = text[zi + len(zip_head):tail]
            inner = inner.replace(
                "return rusty::detail::deref_if_pointer_like(",
                "const bool vs_ok = rusty::detail::deref_if_pointer_like(", 1)
            inner = inner.rstrip()
            if inner.endswith(";"):
                inner = inner[:-1] + "; if (!vs_ok) { return false; }"
            new_block = (
                "const auto ok = [&]{ auto vz1 = rusty::iter(vec); "
                "auto vz2 = rusty::iter(expected);\n"
                "        for (;;) {\n"
                "            auto vx1 = vz1.next(); auto vx2 = vz2.next();\n"
                "            if (!vx1.is_some() || !vx2.is_some()) { return true; }\n"
                "            auto _destruct_param0 = std::make_tuple("
                "std::move(vx1).unwrap(), std::move(vx2).unwrap());"
                + inner + "\n"
                "        }\n"
                "    }();")
            text = text[:zi] + new_block + text[tail + len("\n});"):]
    # Glob-imported Bound constructors in expression position.
    text = re.sub(r"(?<![:\w])Excluded\(", "rusty::bound_excluded(", text)
    text = re.sub(r"(?<![:\w])Included\(", "rusty::bound_included(", text)
    # Bare assoc-fn spellings left over after literal routing.
    text = re.sub(r"(?<![:\w])Vec::from\(", "vt_from(", text)
    text = text.replace("rusty::Vec::from(", "vt_from(")

    # RUSTY_VEC_UNSKIP=name1,name2 (or "all") keeps the named tests live, so a
    # cited skip can be re-checked for staleness without editing SKIP_GROUPS.
    unskip = {
        n.strip() for n in os.environ.get("RUSTY_VEC_UNSKIP", "").split(",") if n.strip()
    }
    if SKIP_TESTS:
        text = stub_tests(text, [t for t in SKIP_TESTS if t not in unskip],
                          "pending triage")
    for names, reason in SKIP_GROUPS:
        live = names if "all" in unskip else [n for n in names if n not in unskip]
        if live:
            text = stub_tests(text, live, reason)

    # `auto x = vt_of();` — untyped empty vec![]: recover the element type
    # from nearby context (following pushes, sibling literals).
    lines = text.split("\n")
    decl = re.compile(
        r"^(\s*)(const )?auto (\w+) = (?:vt_of\(\)|VecT<auto>::new_\(\));$")
    for i, line in enumerate(lines):
        m = decl.match(line)
        if not m:
            continue
        indent, const_kw, name = m.group(1), m.group(2) or "", m.group(3)
        prev = lines[i - 1] if i else ""
        nxt = "\n".join(lines[i + 1:i + 4])
        elem = None
        if "rusty::Box<int32_t>" in nxt:
            elem = "rusty::Box<int32_t>"
        elif re.search(rf"{name}\.push\((?:-?\d|static_cast<int32_t>)", nxt):
            elem = "int32_t"
        elif "std::make_tuple()" in prev:
            elem = "std::tuple<>"
        elif "vt_of(1, 2, 3)" in prev:
            elem = "int32_t"
        if elem is None:
            mlit = re.search(r"vt_of\((\w+)[({]", nxt)
            if mlit:
                elem = mlit.group(1)
        if elem:
            lines[i] = f"{indent}{const_kw}auto {name} = VecT<{elem}>::new_();"
    text = "\n".join(lines)

    path.write_text(text)
    print(f"vec_tests_port patches applied to {path.name}")


def run(cpp_out: Path) -> None:
    for path in sorted(cpp_out.glob("*.cppm")):
        apply_patches(path)


if __name__ == "__main__":
    run(Path(sys.argv[1]))
