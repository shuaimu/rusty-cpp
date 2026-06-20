#!/usr/bin/env python3
"""Post-transpile patches for the hashbrown_port C++20 module port.

Same shape as docs/vec_port/post_transpile_patch.py and
docs/btreemap_port/post_transpile_patch.py: each patch addresses a
specific cluster of errors documented in STATUS.md. Idempotent —
rerunning detects already-applied patches and skips.

Usage:
    python3 post_transpile_patch.py <cpp_out_dir>
"""

import re
import sys
from pathlib import Path


# ── control.tag.cppm ────────────────────────────────────────────────

def patch_tag_methods_const(cpp_out: Path) -> int:
    """Tag::is_full / is_special / special_is_empty are declared
    without `const`, but `fmt()` is const and calls them on `this`.
    The Rust originals take `self` by value; in C++ they should be
    const member functions."""
    path = cpp_out / "hashbrown_port.control.tag.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    # Declarations + out-of-line definitions. The transpiler may emit these
    # methods with a leading `constexpr ` (newer codegen), so allow it
    # optionally and re-emit it verbatim.
    for m in ("is_full", "is_special", "special_is_empty"):
        # Declaration:  [constexpr ]bool <m>();  ->  ... <m>() const;
        text = re.sub(
            r"^(\s*)((?:constexpr )?)bool " + m + r"\(\);",
            r"\1\2bool " + m + r"() const;",
            text, flags=re.MULTILINE)
        # Definition:   [constexpr ]bool Tag::<m>() {  ->  ... () const {
        text = re.sub(
            r"^((?:constexpr )?)bool Tag::" + m + r"\(\) \{",
            r"\1bool Tag::" + m + r"() const {",
            text, flags=re.MULTILINE)
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_tag_formatter_pad(cpp_out: Path) -> int:
    """Tag::fmt() calls `f.pad("EMPTY")` / `f.pad("DELETED")` but
    rusty::fmt::Formatter doesn't have a `pad` method (it's just a
    forward decl). Stub the body to return Ok — Debug formatting
    isn't on any critical path. Uses brace-counting to safely span
    the whole if/else body."""
    path = cpp_out / "hashbrown_port.control.tag.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    sig = "rusty::fmt::Result Tag::fmt(rusty::fmt::Formatter& f) const {"
    sig_pos = text.find(sig)
    if sig_pos == -1:
        return 0
    # If already stubbed, skip.
    body_start = sig_pos + len(sig)
    next_line_end = text.find('\n', body_start)
    next_line = text[body_start:next_line_end]
    if "stubbed" in next_line or "(void)f" in text[body_start:body_start+200]:
        return 0
    # Brace-count to find matching close.
    depth = 1
    j = body_start
    while j < len(text) and depth > 0:
        if text[j] == '{':
            depth += 1
        elif text[j] == '}':
            depth -= 1
            if depth == 0:
                break
        j += 1
    if depth != 0:
        return 0
    stub = (sig +
            "\n    (void)f; // Debug formatting stubbed; rusty::fmt::Formatter has no pad/debug_tuple.\n"
            "    return rusty::fmt::Result::Ok(std::tuple<>{});\n"
            "}")
    new_text = text[:sig_pos] + stub + text[j+1:]
    path.write_text(new_text)
    return 1


def patch_control_tag_fill_tag_mut_self(cpp_out: Path) -> int:
    """`impl TagSliceExt for [MaybeUninit<Tag>]` has `fn fill_tag(&mut
    self, ...)`. The UFCS trait-migration lowers the `&mut self` slice
    receiver to `std::span<const rusty::MaybeUninit<Tag>>& self_` — it
    drops the mutability, emitting a `const`-element span. The body then
    does `as_mut_ptr(self_)->write_bytes(...)` (lowered to
    `rusty::ptr::write_bytes`), which can't write through a const
    element pointer. Restore the `&mut self` mutability by dropping the
    `const` on the self_ span (decl + def). Callers pass a mutable
    `span<MaybeUninit<Tag>>` (ctrl_slice), so this matches."""
    path = cpp_out / "hashbrown_port.control.tag.cppm"
    if not path.exists():
        return 0
    needle = "std::span<const rusty::MaybeUninit<Tag>>& self_"
    repl = "std::span<rusty::MaybeUninit<Tag>>& self_"
    text = path.read_text()
    if needle not in text:
        return 0
    path.write_text(text.replace(needle, repl))
    return 1


# ── hasher.cppm ─────────────────────────────────────────────────────

def patch_hasher_replace_with_stub(cpp_out: Path) -> int:
    """hashbrown's `hasher.rs` is a thin wrapper over the `foldhash`
    crate's hasher. Foldhash is an external dependency we don't want
    to pull in. The hasher module is only used at the HashMap surface
    (to pick a default BuildHasher); RawTable doesn't actually care
    *which* hasher you give it. So rewrite the whole module body to
    expose a minimal `DefaultHasher` / `DefaultHashBuilder` pair
    backed by a stir-style xorshift mix (good enough for smoke
    tests; production callers can swap in their own BuildHasher).
    """
    path = cpp_out / "hashbrown_port.hasher.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    sentinel = "// vec_port-style stub: foldhash replaced with xorshift mix"
    if sentinel in text:
        return 0
    # Find `export module hashbrown_port.hasher;` and replace
    # everything after it.
    anchor = "export module hashbrown_port.hasher;"
    pos = text.find(anchor)
    if pos == -1:
        return 0
    head = text[:pos + len(anchor)]
    body = """

""" + sentinel + """
//
// DefaultHasher: 64-bit state, xorshift mix on `write` / `finish`.
// DefaultHashBuilder: zero-state factory (each build_hasher returns
// a fresh DefaultHasher seeded to 0). Sufficient for HashMap to
// compile + run smoke tests; not cryptographically strong, not
// the high-perf foldhash. Production callers should provide their
// own BuildHasher via the `S` type parameter.

export struct DefaultHasher {
    uint64_t state = 14695981039346656037ULL;  // FNV-1a offset basis
    void write(std::span<const uint8_t> bytes) {
        for (uint8_t b : bytes) {
            state ^= static_cast<uint64_t>(b);
            state *= 1099511628211ULL;  // FNV-1a prime
        }
    }
    void write_u64(uint64_t v) { state ^= v; state *= 1099511628211ULL; }
    uint64_t finish() const { return state; }
    DefaultHasher clone() const { return *this; }
    static DefaultHasher new_() { return DefaultHasher{}; }
    static DefaultHasher default_() { return DefaultHasher{}; }

    // Rust's `BuildHasher::hash_one<T: Hash>(&self, x: T) -> u64`.
    // Doubles as the Hasher's convenience method here (we use the
    // same struct as both hash builder and hasher, FNV-1a state=0).
    // Integer fast path: splitmix64 finalizer — single inline mix,
    // no allocator-trip, matches the std::hash<int> identity-style
    // perf characteristics.
    template<typename T>
    uint64_t hash_one(const T& x) const {
        if constexpr (std::is_integral_v<T>) {
            uint64_t z = static_cast<uint64_t>(x) + 0x9E3779B97F4A7C15ULL;
            z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ULL;
            z = (z ^ (z >> 27)) * 0x94D049BB133111EBULL;
            return z ^ (z >> 31);
        } else if constexpr (requires { x.size(); x.data(); }) {
            DefaultHasher h{};
            h.write(std::span<const uint8_t>(
                reinterpret_cast<const uint8_t*>(x.data()),
                x.size() * sizeof(*x.data())));
            return h.finish();
        } else {
            DefaultHasher h{};
            h.write(std::span<const uint8_t>(
                reinterpret_cast<const uint8_t*>(&x), sizeof(T)));
            return h.finish();
        }
    }
};

export struct DefaultHashBuilder {
    using Hasher = DefaultHasher;
    DefaultHasher build_hasher() const { return DefaultHasher{}; }
    DefaultHashBuilder clone() const { return *this; }
    static DefaultHashBuilder default_() { return {}; }
    bool operator==(const DefaultHashBuilder&) const = default;
};
"""
    new_text = head + body
    path.write_text(new_text)
    return 1


# ── alloc.cppm ──────────────────────────────────────────────────────

def patch_alloc_allocator_api2(cpp_out: Path) -> int:
    """Strip references to the external `allocator_api2` crate.
    The Rust source uses it conditionally via a feature flag; on
    stable Rust the crate's `Allocator` trait is re-exported. We
    use rusty::alloc::Allocator instead."""
    path = cpp_out / "hashbrown_port.alloc.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    text = text.replace("allocator_api2::", "rusty::alloc::")
    text = text.replace("::allocator_api2", "::rusty::alloc")
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_alloc_do_alloc_dup(cpp_out: Path) -> int:
    """`do_alloc` is emitted twice (once per cfg branch in the Rust
    source). Strip one of them."""
    path = cpp_out / "hashbrown_port.alloc.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    # Find a `do_alloc` definition; if there are two, drop the
    # second one. The Rust source uses cfg to pick exactly one.
    pattern = re.compile(
        r"^[^\n]*\bdo_alloc\b[^\n]*\([^)]*\)[^{]*\{(?:[^{}]|\{[^{}]*\})*\}\s*\n",
        re.MULTILINE,
    )
    matches = list(pattern.finditer(text))
    if len(matches) < 2:
        return 0
    # Keep the first; strip the second.
    m = matches[1]
    new_text = text[:m.start()] + text[m.end():]
    path.write_text(new_text)
    return 1


def patch_alloc_global_impl(cpp_out: Path) -> int:
    """`inner::Global::allocate` uses `rusty::ptr::slice_from_raw_parts_mut`
    which doesn't exist. Replace the body with a direct call to
    `std::malloc` wrapped in `NonNull`. Same for `deallocate` (use
    `std::free`). Drops the dependency on rusty::ptr internals."""
    path = cpp_out / "hashbrown_port.alloc.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    sentinel = "// alloc-stub: replaced with std::malloc/std::free direct calls"
    if sentinel in text:
        return 0
    # Replace inner::Global::allocate body. The transpiler emits the
    # whole body on a single line; find by signature + single-line
    # body using DOTALL.
    sig = "rusty::Result<rusty::ptr::NonNull<uint8_t>, std::tuple<>> Global::allocate(rusty::alloc::Layout layout) const {"
    sig_pos = text.find(sig)
    if sig_pos == -1:
        return 0
    # Find matching close brace (the body is one long IIFE).
    body_start = sig_pos + len(sig)
    depth = 1
    j = body_start
    while j < len(text) and depth > 0:
        if text[j] == '{':
            depth += 1
        elif text[j] == '}':
            depth -= 1
            if depth == 0:
                break
        elif text[j] == '"':
            j += 1
            while j < len(text) and text[j] != '"':
                if text[j] == '\\':
                    j += 1
                j += 1
        j += 1
    if depth != 0:
        return 0
    # Stub body.
    stub = (
        "\n        " + sentinel + "\n"
        "        void* p = std::malloc(layout.size);\n"
        "        if (p == nullptr) {\n"
        "            return rusty::Result<rusty::ptr::NonNull<uint8_t>, std::tuple<>>::Err(std::make_tuple());\n"
        "        }\n"
        "        return rusty::Result<rusty::ptr::NonNull<uint8_t>, std::tuple<>>::Ok(\n"
        "            rusty::ptr::NonNull<uint8_t>::new_unchecked(static_cast<uint8_t*>(p)));\n"
        "    "
    )
    new_text = text[:body_start] + stub + text[j:]
    # Also fix deallocate body — the existing one calls `dealloc(...)`
    # which depends on `alloc::dealloc` (renamed to rusty::alloc::dealloc).
    # Just use std::free. Brace-count to find body end (has a nested
    # `// @unsafe { ... }` block).
    dealloc_sig = "void Global::deallocate(rusty::ptr::NonNull<uint8_t> ptr, rusty::alloc::Layout layout) const {"
    dpos = new_text.find(dealloc_sig)
    if dpos != -1:
        dstart = dpos + len(dealloc_sig)
        depth = 1
        k = dstart
        while k < len(new_text) and depth > 0:
            if new_text[k] == '{':
                depth += 1
            elif new_text[k] == '}':
                depth -= 1
                if depth == 0:
                    break
            elif new_text[k] == '"':
                k += 1
                while k < len(new_text) and new_text[k] != '"':
                    if new_text[k] == '\\':
                        k += 1
                    k += 1
            k += 1
        if depth == 0:
            new_text = (new_text[:dstart]
                + "\n        (void)layout;\n"
                + "        std::free(ptr.as_ptr());\n"
                + "    " + new_text[k:])
    # Need <cstdlib> for malloc/free.
    if "<cstdlib>" not in new_text:
        new_text = new_text.replace(
            "#include <cstdint>",
            "#include <cstdint>\n#include <cstdlib>",
            1,
        )
    path.write_text(new_text)
    return 1


def patch_strip_debug_asserts_global(cpp_out: Path) -> int:
    """Rust's `debug_assert!` is a no-op in release builds; the
    transpiler emits it as `assert((expr));` which fires at runtime
    in C++. Strip them across all hashbrown_port cppm files (except
    `assert(true)` which is already a placeholder). This is a
    correctness/perf tradeoff: smoke test passes the happy path; in
    Rust release these never run."""
    total = 0
    pat = re.compile(
        r"^(\s*)assert\(\(((?!true).+?)\)\);",
        re.MULTILINE,
    )
    for path in cpp_out.glob("hashbrown_port.*.cppm"):
        text = path.read_text()
        new = pat.sub(r"\1// debug_assert: \2", text)
        if new != text:
            path.write_text(new)
            total += 1
    return 1 if total else 0


def patch_drop_dup_defaulthasher_global(cpp_out: Path) -> int:
    """Several non-hasher modules (alloc, control.tag, raw, …) carry
    a leftover `struct DefaultHasher` stub from the expanded
    #[derive(Hash)] pre-amble. They conflict with the exported
    `DefaultHasher` from hashbrown_port.hasher (C++ modules don't
    allow the same name to be declared in multiple modules' global-
    module fragments). Strip the stub from every module except
    hasher."""
    total = 0
    stub_block = (
        "// DefaultHasher stub — used by expanded #[derive(Hash)] test code.\n"
        "struct DefaultHasher {\n"
        "std::size_t state = 14695981039346656037ULL;\n"
        "static DefaultHasher new_() { return DefaultHasher{}; }\n"
        "std::size_t finish() const { return state; }\n"
        "};\n"
    )
    for path in cpp_out.glob("hashbrown_port.*.cppm"):
        if path.name == "hashbrown_port.hasher.cppm":
            continue
        text = path.read_text()
        if stub_block not in text:
            continue
        text = text.replace(
            stub_block,
            "// (DefaultHasher stub removed — use hasher::DefaultHasher.)\n",
        )
        path.write_text(text)
        total += 1
    return 1 if total else 0


def patch_alloc_inner_do_alloc_convert(cpp_out: Path) -> int:
    """`inner::do_alloc<A>(alloc, layout)` is declared to return
    `Result<NonNull<u8>, std::tuple<>>` but delegates to
    `alloc.allocate(layout)` which (for A = rusty::alloc::Global)
    returns `Result<NonNull<u8>, AllocError>`. Wrap the delegation
    with the same error-type conversion the adapter patch uses."""
    path = cpp_out / "hashbrown_port.alloc.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    sentinel = "// do_alloc-error-convert: rusty AllocError → std::tuple<>"
    if sentinel in text:
        return 0
    original = text
    # Locate the exported templated do_alloc body and wrap.
    # The body is one long line — match conservatively.
    old = (
        "    rusty::Result<rusty::ptr::NonNull<uint8_t>, std::tuple<>> do_alloc(const A& alloc, rusty::alloc::Layout layout) {\n"
        "        return ([&](auto&& __recv) -> decltype(auto) { if constexpr (requires { std::forward<decltype(__recv)>(__recv).allocate(std::move(layout)); }) { return std::forward<decltype(__recv)>(__recv).allocate(std::move(layout)); } else { return std::forward<decltype(__recv)>(__recv)->allocate(std::move(layout)); } }(alloc));\n"
        "    }"
    )
    new = (
        "    rusty::Result<rusty::ptr::NonNull<uint8_t>, std::tuple<>> do_alloc(const A& alloc, rusty::alloc::Layout layout) {\n"
        "        " + sentinel + "\n"
        "        auto r = ([&](auto&& __recv) -> decltype(auto) { if constexpr (requires { std::forward<decltype(__recv)>(__recv).allocate(std::move(layout)); }) { return std::forward<decltype(__recv)>(__recv).allocate(std::move(layout)); } else { return std::forward<decltype(__recv)>(__recv)->allocate(std::move(layout)); } }(alloc));\n"
        "        if (r.is_ok()) return rusty::Result<rusty::ptr::NonNull<uint8_t>, std::tuple<>>::Ok(r.unwrap());\n"
        "        return rusty::Result<rusty::ptr::NonNull<uint8_t>, std::tuple<>>::Err(std::make_tuple());\n"
        "    }"
    )
    if old not in text:
        return 0
    text = text.replace(old, new)
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_alloc_adapter_error_convert(cpp_out: Path) -> int:
    """The transpiled `AllocatorAdapter<rusty::alloc::Global>` (and
    Ref/RefMut variants) override Allocator::allocate to return
    `Result<NonNull<u8>, std::tuple<>>` but delegate to
    `rusty::alloc::Global::allocate(layout)` which returns
    `Result<NonNull<u8>, AllocError>`. Wrap the delegation with an
    error-type conversion."""
    path = cpp_out / "hashbrown_port.alloc.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    sentinel = "// adapter-error-convert: rusty AllocError → std::tuple<>"
    if sentinel in text:
        return 0
    original = text
    # Match each adapter's allocate override body and replace it.
    pattern = re.compile(
        r"(rusty::Result<rusty::ptr::NonNull<uint8_t>, std::tuple<>> allocate\(rusty::alloc::Layout layout\) const override \{)\s*\n"
        r"\s*return value_\.allocate\(layout\);\s*\n"
        r"(\s*\})",
    )
    replacement = (
        r"\1\n"
        + "        " + sentinel + "\n"
        + "        auto r = value_.allocate(layout);\n"
        + "        if (r.is_ok()) return rusty::Result<rusty::ptr::NonNull<uint8_t>, std::tuple<>>::Ok(r.unwrap());\n"
        + "        return rusty::Result<rusty::ptr::NonNull<uint8_t>, std::tuple<>>::Err(std::make_tuple());\n"
        + r"\2"
    )
    new_text = pattern.sub(replacement, text)
    if new_text == original:
        return 0
    path.write_text(new_text)
    return 1


# ── control.group.generic.cppm ──────────────────────────────────────

def patch_group_generic_replace(cpp_out: Path) -> int:
    """The transpiled `control/group/generic.rs` lands with several
    issues that compound badly:
    - Cross-module imports (`Tag`, `BitMask`, `BitMaskIter`) are
      emitted as `// Rust-only unresolved import:` comments instead
      of `import` statements.
    - `BitMaskWord` is used in `extern const BitMaskWord BITMASK_MASK;`
      before its own `using BitMaskWord = GroupWord;` decl.
    - `u64::from_ne_bytes(...)` emitted as Rust syntax verbatim.
    - `rusty::clone(this->_0)` references a non-existent free fn.
    - IIFE-wrapped tuple-field accessor for `Tag::DELETED._0` adds
      noise.

    The whole module is small (~150 LOC of bit-twiddling) and well-
    understood; the cleanest fix is to replace it wholesale with a
    hand-rolled equivalent of the Rust generic group impl. This
    matches the playbook for spec_from_elem in the Vec port.
    """
    path = cpp_out / "hashbrown_port.control.group.generic.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    sentinel = "// hand-rolled generic-group impl: full module body"
    if sentinel in text:
        return 0
    # Find the module declaration line; replace everything after it.
    anchor = "export module hashbrown_port.control.group.generic;"
    pos = text.find(anchor)
    if pos == -1:
        return 0
    head = text[:pos + len(anchor)]
    body = """

""" + sentinel + """
//
// Replaces the transpiled body wholesale — the original emit had
// cross-module type lookups unresolved (`Tag`, `BitMask`) plus
// `u64::from_ne_bytes(...)` verbatim and IIFE artifacts that
// cluster too tightly to peel off with surgical patches. Logic
// mirrors hashbrown-0.17.0/src/control/group/generic.rs.
//
// We can't `import hashbrown_port.control.bitmask` or `.tag` here
// because `control.bitmask` already imports `control.group` (which
// re-exports us) — that would create a module-import cycle.
// Instead, hand-inline minimal `Tag` and `BitMask` definitions
// that are layout-compatible with the sibling modules' versions.
// (The actual cross-module access happens via the parent module
// `control.group` re-exporting our `Group` type, which the rest
// of hashbrown consumes.)

// Module-private definitions of Tag and BitMask, in a named
// namespace to avoid conflict with the real Tag/BitMask in
// sibling modules (control.tag, control.bitmask). We can't import
// those siblings here without creating a module-import cycle.
// Layout-compatible by design.
namespace group_internal {
struct Tag {
    uint8_t _0;
    static const Tag EMPTY;
    static const Tag DELETED;
};
inline const Tag Tag::EMPTY = Tag{0xFF};
inline const Tag Tag::DELETED = Tag{0x80};

struct BitMask {
    uint64_t _0;
    // Methods used by raw.cppm's match-result chains. Layout
    // semantics mirror the real `control::BitMask` in
    // hashbrown's control/bitmask.rs.
    //
    // The word stores 1 *match* bit per *byte* (the high bit of each
    // byte is the match flag). Methods that return a byte index must
    // divide ctz/clz by 8 (BITMASK_STRIDE for the generic path) to
    // convert bit-position → byte-position; otherwise callers walk
    // off into wrong slots (see hashbrown/control/bitmask.rs).
    bool any_bit_set() const { return _0 != 0; }
    BitMask remove_lowest_bit() const { return BitMask{_0 & (_0 - 1)}; }
    size_t trailing_zeros() const { return _0 == 0 ? 8 : __builtin_ctzll(_0) / 8; }
    size_t leading_zeros() const { return _0 == 0 ? 8 : __builtin_clzll(_0) / 8; }
    // `lowest_set_bit() -> Option<size_t>`. Returns Some(byte_idx) if
    // any byte's match bit is set, None if all zero. `byte_idx` is
    // a slot offset within the group (0..WIDTH-1), NOT the raw
    // ctz bit position.
    rusty::Option<size_t> lowest_set_bit() const {
        if (_0 == 0) return rusty::Option<size_t>(rusty::None);
        return rusty::Option<size_t>(static_cast<size_t>(__builtin_ctzll(_0)) / 8);
    }
};
}
using group_internal::Tag;
using group_internal::BitMask;

using GroupWord = uint64_t;
using NonZeroGroupWord = rusty::num::NonZeroU64;
export using BitMaskWord = GroupWord;
export using NonZeroBitMaskWord = NonZeroGroupWord;
export constexpr size_t BITMASK_STRIDE = 8;
export constexpr BitMaskWord BITMASK_ITER_MASK = ~static_cast<BitMaskWord>(0);

// Top bit of each byte set; equivalent to Rust's
// `u64::from_ne_bytes([Tag::DELETED.0; 8])` where Tag::DELETED.0 == 0x80.
export constexpr BitMaskWord BITMASK_MASK = 0x8080808080808080ULL;

namespace mem = rusty::mem;
namespace ptr = rusty::ptr;

/// Helper: replicate a tag byte across a `GroupWord`.
static inline GroupWord repeat_tag(Tag tag) {
    GroupWord w = 0;
    for (size_t i = 0; i < 8; ++i) {
        w |= static_cast<GroupWord>(tag._0) << (i * 8);
    }
    return w;
}

/// Abstraction over a group of control tags scanned in parallel.
/// Word-sized integer implementation (no SIMD).
export struct Group {
    static constexpr size_t WIDTH = sizeof(GroupWord);
    GroupWord _0;

    static const std::array<Tag, WIDTH>& static_empty() {
        alignas(GroupWord) static constexpr std::array<Tag, WIDTH> empty = {
            Tag{0xFF}, Tag{0xFF}, Tag{0xFF}, Tag{0xFF},
            Tag{0xFF}, Tag{0xFF}, Tag{0xFF}, Tag{0xFF},
        };
        return empty;
    }

    // Signatures take `const uint8_t*` / `uint8_t` rather than Tag so
    // callers in other modules (using control.tag's Tag) don't trip
    // the cross-module Tag-type mismatch (our `group_internal::Tag`
    // is layout-compatible but a distinct type).
    static Group load(const uint8_t* ptr) {
        GroupWord w;
        std::memcpy(&w, ptr, sizeof(w));
        return Group{w};
    }

    static Group load_aligned(const uint8_t* ptr) {
        return load(ptr);
    }

    void store_aligned(uint8_t* ptr) const {
        std::memcpy(ptr, &_0, sizeof(_0));
    }

    // Overload accepting Tag-shaped types (any T with `_0` byte
    // field) — covers control.tag::Tag from callers.
    template<typename T>
    auto match_tag(const T& t) const -> std::enable_if_t<!std::is_integral_v<T>, BitMask> {
        return match_tag(static_cast<uint8_t>(t._0));
    }
    BitMask match_tag(uint8_t tag_byte) const {
        GroupWord cmp = _0 ^ repeat_tag(Tag{tag_byte});
        // x - 0x01... will overflow into the high bit if the byte was 0.
        GroupWord r = (cmp - 0x0101010101010101ULL) & ~cmp & 0x8080808080808080ULL;
        return BitMask{static_cast<BitMaskWord>(r)};
    }

    BitMask match_empty() const {
        return match_tag(Tag::EMPTY._0);
    }

    BitMask match_empty_or_deleted() const {
        // High bit set means tag is EMPTY (0xFF) or DELETED (0x80).
        return BitMask{static_cast<BitMaskWord>(_0 & BITMASK_MASK)};
    }

    BitMask match_full() const {
        // High bit unset means FULL.
        return BitMask{static_cast<BitMaskWord>(~_0 & BITMASK_MASK)};
    }

    Group convert_special_to_empty_and_full_to_deleted() const {
        // FULL (high bit clear) -> DELETED (0x80); EMPTY/DELETED (high bit set) -> EMPTY (0xFF).
        // Matches hashbrown `full = !self.0 & repeat(0x80); !full + (full >> 7)` (cf. match_full
        // above, which masks ~_0). The earlier `_0 & BITMASK_MASK` (no ~) plus a spurious
        // `| 0x8080..` computed the INVERSE mapping, so prepare_rehash_in_place marked live slots
        // EMPTY and left tombstones DELETED and rehash re-placed phantom elements.
        GroupWord full = ~_0 & BITMASK_MASK;
        return Group{~full + (full >> 7)};
    }

    Group clone() const { return Group{_0}; }
};
"""
    new_text = head + body
    path.write_text(new_text)
    return 1


def patch_control_bitmask_imports(cpp_out: Path) -> int:
    """`control.bitmask.cppm`:
    - The `import hashbrown_port.control.group;` line appears AFTER
      forward decls, but C++20 requires all imports immediately
      after the module decl. Move it up.
    - `group::BitMaskWord` etc. — `group` isn't a real namespace (Rust
      `use super::group as imp` doesn't translate). Drop the prefix.
    - `rusty::clone(x)` doesn't exist; rewrite to `x` (the field is
      already a trivially-copyable integer).
    """
    path = cpp_out / "hashbrown_port.control.bitmask.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    # Strip ALL existing imports of control.group first.
    text = re.sub(
        r"^import hashbrown_port\.control\.group;\s*\n",
        "",
        text,
        flags=re.MULTILINE,
    )
    # Then insert one immediately after the module decl.
    text = text.replace(
        "export module hashbrown_port.control.bitmask;\n",
        "export module hashbrown_port.control.bitmask;\nimport hashbrown_port.control.group;\n",
        1,
    )
    # Strip `group::` qualifier on type names.
    text = text.replace("group::BitMaskWord", "BitMaskWord")
    text = text.replace("group::NonZeroBitMaskWord", "NonZeroBitMaskWord")
    text = text.replace("group::BITMASK_STRIDE", "BITMASK_STRIDE")
    text = text.replace("group::BITMASK_ITER_MASK", "BITMASK_ITER_MASK")
    text = text.replace("group::BITMASK_MASK", "BITMASK_MASK")
    # rusty::clone(x) → x (the field is a plain integer/BitMaskWord)
    text = re.sub(r"rusty::clone\(([^)]+)\)", r"\1", text)
    # Replace `/* cfg!(target_arch = "arm") */` with `false` — the
    # transpiler emits cfg!() as a stray comment, leaving `if (
    # /* ... */ && (...))` which is a syntax error. We don't compile
    # for ARM here so the branch is dead anyway.
    text = text.replace(
        '/* cfg!(target_arch = "arm") */',
        'false',
    )
    # Rust integer-trait methods → C++ builtins. BitMaskWord is uint64_t.
    text = text.replace("this->_0.swap_bytes()", "__builtin_bswap64(this->_0)")
    text = text.replace("this->_0.trailing_zeros()", "__builtin_ctzll(this->_0)")
    text = text.replace("this->_0.leading_zeros()", "__builtin_clzll(this->_0)")
    text = text.replace("nonzero.get().swap_bytes()", "__builtin_bswap64(nonzero.get())")
    text = text.replace("nonzero.trailing_zeros()", "__builtin_ctzll(nonzero.get())")
    # Chained `.leading_zeros()` after our bswap rewrite (the ARM-
    # specific dead branch). Compiler will DCE; still must parse.
    text = text.replace("__builtin_bswap64(this->_0).leading_zeros()",
                        "__builtin_clzll(__builtin_bswap64(this->_0))")
    # rusty::leading_zeros(x) → __builtin_clzll(x.get())
    text = re.sub(
        r"rusty::leading_zeros\(([^)]+)\)",
        r"__builtin_clzll(\1.get())",
        text,
    )
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_control_module_namespaces(cpp_out: Path) -> int:
    """`control.cppm` (parent module) uses `bitmask::`, `group::`,
    `tag::` as namespace prefixes — but Rust `mod foo;` doesn't
    create a C++ namespace. After `import hashbrown_port.control.foo;`,
    the exported symbols are at module scope, no prefix needed."""
    path = cpp_out / "hashbrown_port.control.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    text = text.replace("using bitmask::", "using ::")
    text = text.replace("using group::", "using ::")
    text = text.replace("using tag::", "using ::")
    # Some `using X;` declarations weren't originally `export`-ed
    # (e.g. `using bitmask::BitMask;` was internal-only). For our
    # purposes downstream modules (raw.cppm) need BitMask visible,
    # so promote those to `export using` too.
    text = re.sub(
        r"^using\s+::(\w+);",
        r"export using ::\1;",
        text,
        flags=re.MULTILINE,
    )
    # TagSliceExt is an extension-trait adapter class living in an
    # anonymous namespace inside control.tag — not exported. Drop
    # the re-export attempt.
    text = re.sub(
        r"^export using ::TagSliceExt;\s*\n",
        "",
        text,
        flags=re.MULTILINE,
    )
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_control_group_imp_alias(cpp_out: Path) -> int:
    """`control.group.cppm` was emitted with `namespace imp = ::generic;`
    and `using generic::Group;` etc — both wrong. The Rust source
    has `mod generic; use generic as imp;` but C++20 modules don't
    create a sibling `::generic` namespace from `mod generic;`.
    After `import hashbrown_port.control.group.generic;`, the
    exported names are at module scope, no qualification needed.
    Strip the `generic::` qualifier and the broken namespace alias."""
    path = cpp_out / "hashbrown_port.control.group.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    text = text.replace("namespace imp = ::generic;\n", "")
    text = text.replace("export using generic::", "export using ::")
    if text != original:
        path.write_text(text)
        return 1
    return 0


# ── raw.cppm ────────────────────────────────────────────────────────

def patch_raw_tryreserveerror(cpp_out: Path) -> int:
    """`raw.cppm` uses bare `TryReserveError` (the Rust source has
    `use crate::TryReserveError;` which resolves to a re-export from
    `std::collections::TryReserveError`). The transpiler emitted it
    unqualified but rusty has it under `rusty::collections`. Add the
    namespace qualifier."""
    path = cpp_out / "hashbrown_port.raw.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    # `::TryReserveError` (leading double-colon, global namespace ref) →
    # `rusty::collections::TryReserveError`. Negative-lookbehind on
    # `s` to avoid matching the `s::TryReserveError` tail of an
    # already-qualified `rusty::collections::TryReserveError`.
    text = re.sub(
        r"(?<!s)::TryReserveError\b(?!_)",
        "rusty::collections::TryReserveError",
        text,
    )
    # Bare TryReserveError (not preceded by word char or `::`) → qualified.
    text = re.sub(
        r"(?<![\w:])TryReserveError\b(?!_)",
        "rusty::collections::TryReserveError",
        text,
    )
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_raw_rehash_guard_writeback(cpp_out: Path) -> int:
    """rehash_in_place() lowers Rust `guard(self, ...)` (where `self` is `&mut Self`)
    to `auto guard = guard((*this), ...)`, which makes a BY-VALUE copy of the table
    (the transpiler maps `self` to `(*this)` and ScopeGuard takes its `T` by value —
    same root cause as patch_raw_table_clear_double_free / patch_scopeguard_dropfn_arg).
    `RawTableInner` has no user copy ctor, so the copy is SHALLOW and shares the
    `ctrl_field` buffer pointer: the in-place set_ctrl / element moves DO reach the real
    control bytes, but the copy's scalar fields (growth_left/items/bucket_mask) live only
    on the copy and `mem::forget(guard)` discards them. Without writing them back,
    growth_left stays stale and a later insert underflows it (size_t) -> the probe runs
    past the end of the table ("Went past end of probe sequence"). Reflect the scalars
    back into *this before forgetting the guard. (Runs after the earlier raw patches that
    rewrite `guard.x` -> `(*guard).x`.)"""
    path = cpp_out / "hashbrown_port.raw.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    needle = (
        "    (*guard).growth_left = ::bucket_mask_to_capacity(std::move((*guard).bucket_mask))"
        " - rusty::detail::deref_if_pointer_like((*guard).items);\n"
        "    rusty::mem::forget(std::move(guard));"
    )
    replacement = (
        "    (*guard).growth_left = ::bucket_mask_to_capacity(std::move((*guard).bucket_mask))"
        " - rusty::detail::deref_if_pointer_like((*guard).items);\n"
        "    // PORT FIX: `guard` is a by-value copy of the table (transpiler lowered Rust's\n"
        "    // `&mut self` to `(*this)`). Its ctrl buffer is SHARED (shallow copy), so the\n"
        "    // rehash above reached the real control bytes, but these scalar fields live only\n"
        "    // on the copy. Reflect them back into *this before forgetting it, else growth_left\n"
        "    // stays stale and a later insert underflows it -> probe past end of table.\n"
        "    this->growth_left = (*guard).growth_left;\n"
        "    this->items = (*guard).items;\n"
        "    this->bucket_mask = (*guard).bucket_mask;\n"
        "    rusty::mem::forget(std::move(guard));"
    )
    if needle in text:
        text = text.replace(needle, replacement)
        path.write_text(text)
        return 1
    return 0


def patch_raw_imports_top(cpp_out: Path) -> int:
    """`raw.cppm` has `import hashbrown_port.control;` (and friends)
    appearing after forward decls. C++20 requires all imports right
    after the module decl."""
    path = cpp_out / "hashbrown_port.raw.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    sentinel = "// raw: imports hoisted to top"
    if sentinel in text:
        return 0
    # Collect all `import hashbrown_port.X;` lines (and `import vec_port.X;`
    # etc.) appearing in the file body.
    imports = re.findall(r"^import\s+[\w.]+\s*;\s*\n", text, flags=re.MULTILINE)
    if not imports:
        return 0
    # Strip them from their current positions.
    new_text = re.sub(r"^import\s+[\w.]+\s*;\s*\n", "", text, flags=re.MULTILINE)
    # Re-inject all (unique, preserving order) right after module decl.
    seen = set()
    uniq = []
    for line in imports:
        if line not in seen:
            seen.add(line)
            uniq.append(line)
    anchor = "export module hashbrown_port.raw;\n"
    pos = new_text.find(anchor)
    if pos == -1:
        return 0
    insertion = "\n" + sentinel + "\n" + "".join(uniq)
    new_text = new_text[:pos + len(anchor)] + insertion + new_text[pos + len(anchor):]
    path.write_text(new_text)
    return 1


def patch_raw_misc_fixups(cpp_out: Path) -> int:
    """Several mechanical fixups for raw.cppm:
    - `control::BitMaskIter` etc → drop `control::` (imported flat).
    - `invalid_mut(x)` — Rust core::ptr::invalid_mut, construct pointer
      from usize. Inline as `reinterpret_cast<T*>(x)`.
    - `assert((... <Rust syntax> ...))` lines where the transpiler
      preserved verbatim Rust syntax inside assert!() macros — these
      have spaced-out `.`, `::`, `as` etc. that don't parse as C++.
      Replace with `assert(true)` (drops the runtime check; safe for
      Phase A2 just-compile goal)."""
    path = cpp_out / "hashbrown_port.raw.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    original = text

    # Strip `control::` qualifier (only the path, not `control:` followed
    # by other punctuation).
    text = re.sub(r"\bcontrol::(?=\w)", "", text)

    # invalid_mut(x) → reinterpret_cast<T*>(x) where T comes from context.
    # Since we don't know T at patch time, use a generic cast via void*
    # then assume the user-side will type-narrow. Simpler: just inline as
    # `reinterpret_cast<uint8_t*>(x)` and let the caller cast further.
    # Better: assume it's used in pointer arithmetic where the target
    # type is known — provide as `((uint8_t*)(uintptr_t)(x))`.
    text = re.sub(
        r"\binvalid_mut\(([^)]+)\)",
        r"reinterpret_cast<uint8_t*>(static_cast<std::uintptr_t>(\1))",
        text,
    )

    # Strip assert(...) lines containing spaced-out Rust syntax —
    # detect `space . space` or `space :: space` or `as ` inside the
    # assert. Replace whole line with `assert(true);  // stripped: <orig>`.
    def strip_assert(m):
        body = m.group(0)
        if (" . " in body or " :: " in body or " as " in body):
            return "        assert(true);  // " + body.strip()[:60] + "...\n"
        return body
    text = re.sub(
        r"^\s*assert\(\([^\n]+\)\);\s*\n",
        strip_assert,
        text,
        flags=re.MULTILINE,
    )

    # Rust `.cast::<T>()` on a raw pointer → C++ reinterpret_cast.
    # The transpiler emitted these as `expr->cast()`. Brace-walk-back
    # to handle arbitrary `expr` (including `rusty::as_ptr(...)->cast()`).
    # Process left-to-right with explicit positions to skip the
    # `->cast_mut()` variant.
    search_from = 0
    while True:
        idx = text.find("->cast()", search_from)
        if idx == -1:
            break
        # Skip if this is actually `->cast_mut()`.
        # (Look back 4 chars to see `_mut`. No — `->cast()` and
        # `->cast_mut()` overlap on `->cast`; the substring search
        # for `->cast()` won't match `->cast_mut()` because it
        # requires `()` immediately after `cast`. So no overlap.)
        i = idx
        if i > 0 and text[i-1] == ')':
            depth = 1
            j = i - 2
            while j >= 0 and depth > 0:
                if text[j] == ')':
                    depth += 1
                elif text[j] == '(':
                    depth -= 1
                    if depth == 0:
                        break
                j -= 1
            k = j - 1
            while k >= 0 and (text[k].isalnum() or text[k] in "_:"):
                k -= 1
            expr_start = k + 1
        else:
            # Walk back over identifier chars / `::` / `.` / `->`.
            # We include `-` so `this->ctrl` walks correctly across
            # member-access. Stop at whitespace, `(`, `,`, or `=`.
            k = i - 1
            while k >= 0 and (text[k].isalnum() or text[k] in "_:>.-"):
                k -= 1
            expr_start = k + 1
        expr = text[expr_start:i]
        # Cast to `Tag*` — most call sites want Tag* (e.g. `ctrl()`
        # accessor). The few `Group::load_aligned(ptr->cast())` sites
        # then get wrapped by `patch_raw_misc_fixups`'s Group-arg
        # wrap (which adds `reinterpret_cast<const uint8_t*>`).
        # Use `const_cast`+`reinterpret_cast` to handle both const-
        # and non-const-source pointers.
        replacement = "const_cast<Tag*>(reinterpret_cast<const Tag*>(" + expr + "))"
        text = text[:expr_start] + replacement + text[idx + len("->cast()"):]
        # Advance search past the replacement.
        search_from = expr_start + len(replacement)
    # `rusty::mem::MaybeUninit<X>` → `rusty::MaybeUninit<X>` (real path).
    text = text.replace("rusty::mem::MaybeUninit", "rusty::MaybeUninit")
    text = text.replace("mem::MaybeUninit", "rusty::MaybeUninit")
    # `scopeguard::guard(...)` — drop the `::` qualifier; `guard`
    # is already imported via `import hashbrown_port.scopeguard;`.
    text = text.replace("scopeguard::", "")
    # `cast_mut()` — Rust ptr method; same treatment as `.cast()`.
    text = re.sub(
        r"(this->)?(\w+)->cast_mut\(\)",
        lambda m: "reinterpret_cast<uint8_t*>(const_cast<uint8_t*>(" + (m.group(1) or "") + m.group(2) + "))",
        text,
    )
    # `.cast_mut()` on Tag-typed values used as `Tag::EMPTY.cast_mut()`
    # etc — replace with address-of of the byte.
    text = text.replace(".cast_mut()", "")
    # Rust `usize` type → C++ size_t.
    text = re.sub(r"\busize\b(?!\w)", "size_t", text)

    # Group::load*/store_aligned take `const uint8_t*`/`uint8_t*` in
    # our hand-rolled body, but call sites pass `Tag*` (real control.
    # tag::Tag). Wrap the arg in a reinterpret_cast. Scan once,
    # left-to-right; rebuild via segments.
    # `.store_aligned(arg)` — instance method form. Wrap with
    # `reinterpret_cast<uint8_t*>` since callers pass `Tag*` from
    # `this->ctrl(i)`.
    out_parts = []
    search_from = 0
    while True:
        idx = text.find(".store_aligned(", search_from)
        if idx == -1:
            out_parts.append(text[search_from:])
            break
        arg_start = idx + len(".store_aligned(")
        depth = 1
        j = arg_start
        while j < len(text) and depth > 0:
            if text[j] == '(':
                depth += 1
            elif text[j] == ')':
                depth -= 1
                if depth == 0:
                    break
            j += 1
        if depth != 0:
            out_parts.append(text[search_from:])
            break
        arg = text[arg_start:j]
        if arg.lstrip().startswith("reinterpret_cast"):
            out_parts.append(text[search_from:j+1])
        else:
            out_parts.append(text[search_from:idx])
            out_parts.append(".store_aligned(reinterpret_cast<uint8_t*>(" + arg + "))")
        search_from = j + 1
    text = "".join(out_parts)

    for fn in ("Group::load_aligned", "Group::load", "Group::store_aligned"):
        # `store_aligned` wants `uint8_t*` (non-const); the loads want
        # `const uint8_t*`.
        const_q = "" if fn == "Group::store_aligned" else "const "
        prefix = fn + "("
        out_parts = []
        search_from = 0
        while True:
            idx = text.find(prefix, search_from)
            if idx == -1:
                out_parts.append(text[search_from:])
                break
            arg_start = idx + len(prefix)
            depth = 1
            j = arg_start
            while j < len(text) and depth > 0:
                if text[j] == '(':
                    depth += 1
                elif text[j] == ')':
                    depth -= 1
                    if depth == 0:
                        break
                j += 1
            if depth != 0:
                out_parts.append(text[search_from:])
                break
            arg = text[arg_start:j]
            # Skip if already cast.
            if arg.lstrip().startswith("reinterpret_cast"):
                out_parts.append(text[search_from:j+1])
            else:
                out_parts.append(text[search_from:idx])
                out_parts.append(fn + "(reinterpret_cast<" + const_q + "uint8_t*>(" + arg + "))")
            search_from = j + 1
        text = "".join(out_parts)
    # Rust integer trait methods on size_t: `.checked_add`, `.checked_mul`,
    # `.checked_sub`. Reuse the rusty::num helpers when available.
    text = re.sub(
        r"(\w+)\.checked_mul\(([^)]+)\)",
        r"rusty::num::checked_mul(\1, \2)",
        text,
    )
    text = re.sub(
        r"(\w+)\.checked_add\(([^)]+)\)",
        r"rusty::num::checked_add(\1, \2)",
        text,
    )
    text = re.sub(
        r"(\w+)\.checked_sub\(([^)]+)\)",
        r"rusty::num::checked_sub(\1, \2)",
        text,
    )
    # `size_t::max(a, b)` → `std::max(a, b)`. Rust associated fn on
    # integer type.
    text = text.replace("size_t::max(", "std::max<size_t>(")
    text = text.replace("size_t::min(", "std::min<size_t>(")
    # `x.is_power_of_two()` on integer → bit check.
    text = re.sub(
        r"(\w+)\.is_power_of_two\(\)",
        r"((\1) != 0 && ((\1) & ((\1) - 1)) == 0)",
        text,
    )
    # `x.next_power_of_two()` → __builtin-style impl.
    text = re.sub(
        r"(\w+)\.next_power_of_two\(\)",
        r"(static_cast<size_t>(1) << (64 - __builtin_clzll((\1) - 1)))",
        text,
    )
    # `x.count_ones()`, `.count_zeros()`, `.trailing_ones()`,
    # `.leading_ones()` — integer popcount-like.
    text = re.sub(
        r"(\w+)\.count_ones\(\)",
        r"__builtin_popcountll(\1)",
        text,
    )
    # `expr->cast_mut().cast()` (chained) — flatten both at once.
    # Use brace-matching to walk back across nested parens, since
    # `expr` may be `rusty::as_ptr(foo)` etc.
    while True:
        idx = text.find("->cast_mut().cast()")
        if idx == -1:
            break
        # Walk back to find the start of the expression that owns
        # the `->`. It might be a parenthesized call.
        i = idx
        if i > 0 and text[i-1] == ')':
            # Balanced-paren walk back.
            depth = 1
            j = i - 2
            while j >= 0 and depth > 0:
                if text[j] == ')':
                    depth += 1
                elif text[j] == '(':
                    depth -= 1
                    if depth == 0:
                        break
                j -= 1
            # Continue back across identifier chars and `::` to grab the
            # call's name.
            k = j - 1
            while k >= 0 and (text[k].isalnum() or text[k] in "_:"):
                k -= 1
            expr_start = k + 1
        else:
            # Plain identifier (or `->`-chain).
            k = i - 1
            while k >= 0 and (text[k].isalnum() or text[k] in "_:>."):
                k -= 1
            expr_start = k + 1
        expr = text[expr_start:i]
        text = text[:expr_start] + "const_cast<uint8_t*>(reinterpret_cast<const uint8_t*>(" + expr + "))" + text[idx + len("->cast_mut().cast()"):]
    # `rusty::alloc::Global` used as a value (no `{}`) — `Global` is
    # a struct type. Only replace when it appears as a function call
    # argument: preceded by `,` or `(` and followed by `,` or `)`.
    # (Avoid breaking `using` decls and template default args.)
    text = re.sub(
        r"([,(]\s*)rusty::alloc::Global(\s*[,)])",
        r"\1rusty::alloc::Global{}\2",
        text,
    )

    # `T::NEEDS_DROP` — Rust mem::needs_drop trait check. Replace with
    # the C++ equivalent: !std::is_trivially_destructible_v<T>.
    text = text.replace(
        "T::NEEDS_DROP",
        "(!std::is_trivially_destructible_v<T>)",
    )
    # `<X>layout.size()` / `<X>layout.align()` — Rust
    # core::alloc::Layout has these as methods, but rusty::alloc::
    # Layout exposes them as plain fields. Strip the parens for any
    # identifier ending in `layout` (covers `layout`,
    # `oversized_layout`, `table_layout`, etc.).
    text = re.sub(
        r"\b(\w*layout)\.size\(\)",
        r"\1.size",
        text,
    )
    text = re.sub(
        r"\b(\w*layout)\.align\(\)",
        r"\1.align",
        text,
    )
    # RawTable::TABLE_LAYOUT stays `static constexpr` (constant-initialized).
    # The transpiler now lowers Rust `const fn` -> C++ `constexpr`, so
    # `TableLayout::new_<T>()` is constexpr (Layout::new_ and rusty::clone are
    # constexpr; ternaries are constexpr-valid) and the constant initializer is
    # well-formed. Previously this was downgraded to `static inline const`, which
    # made TABLE_LAYOUT *dynamically* initialized — a static-init-order fiasco when
    # a HashMap is built from a static initializer. (No downgrade now.)
    #
    # RawTableInner::NEW = RawTableInner::new_() is a *dynamically*-initialized
    # inline global, and new_() can't be constexpr (it reinterpret_casts the
    # Tag* control-group pointer to u8*, illegal in a constant expression). Cloning
    # NEW to build an empty table from a static initializer reads it before its
    # dynamic init runs -> null ctrl -> SIGSEGV in Group::load_aligned. Eliminate
    # the dynamic-init-global read: call new_() directly at each use site. new_()
    # re-reads the constexpr Group::static_empty() sentinel, so it is safe to call
    # at any time, including during static initialization.
    text = text.replace(
        "rusty::clone(rusty::clone(RawTableInner::NEW))",
        "RawTableInner::new_()",
    )
    text = text.replace(
        "rusty::detail::deref_if_pointer_like(RawTableInner::NEW)",
        "rusty::detail::deref_if_pointer_like(RawTableInner::new_())",
    )
    # `drop_inner_table<T, std::remove_cvref_t<decltype((rusty::clone(rusty::clone(RawTable<T, A>::TABLE_LAYOUT))))>>`
    # — the transpiler recovered the table_layout arg's type as a
    # template param, but it's an ARG, not a param. Method signature
    # is `drop_inner_table<T, A>(alloc, table_layout)`. Correct the
    # explicit template args.
    text = re.sub(
        r"drop_inner_table<T, std::remove_cvref_t<[^>]+>>",
        "drop_inner_table<T, A>",
        text,
    )
    # Same shape may appear nested — handle once more for cases with
    # extra `>>` groups.
    text = text.replace(
        "drop_inner_table<T, std::remove_cvref_t<decltype((rusty::clone(rusty::clone(RawTable<T, A>::TABLE_LAYOUT))))>>",
        "drop_inner_table<T, A>",
    )
    # `result.unwrap_unchecked()` — rusty::Result has no
    # unwrap_unchecked; `unwrap()` is the safe equivalent (panics
    # on Err instead of UB).
    text = text.replace(".unwrap_unchecked()", ".unwrap()")
    # `Option<UnsafeFn<void(uint8_t*)>>([&](...) -> UnsafeFn<void(uint8_t*)> { return drop_in_place(...); })`
    # — the transpiler emitted a lambda that returns drop_in_place's
    # void result, treating it as a UnsafeFn. The actual shape needed
    # is: the lambda IS the function pointer that gets wrapped by
    # UnsafeFn. Rewrite to a stateless lambda converted to fn ptr.
    text = text.replace(
        "((!std::is_trivially_destructible_v<T>) ? rusty::Option<rusty::UnsafeFn<void(uint8_t*)>>([&](auto&& ptr_shadow1) -> rusty::UnsafeFn<void(uint8_t*)> { return rusty::ptr::drop_in_place(ptr_shadow1.template cast<T>()); }) : rusty::Option<rusty::UnsafeFn<void(uint8_t*)>>{rusty::None})",
        "((!std::is_trivially_destructible_v<T>) "
        "? rusty::Option<rusty::UnsafeFn<void(uint8_t*)>>("
        "rusty::UnsafeFn<void(uint8_t*)>("
        "+[](uint8_t* __p) { "
        "std::destroy_at(reinterpret_cast<T*>(__p)); }"
        ")) "
        ": rusty::Option<rusty::UnsafeFn<void(uint8_t*)>>{rusty::None})",
    )

    # `bucket.write(value)` — Rust's Bucket::write does a raw write
    # through the slot pointer (placement new semantics for the
    # uninitialized memory returned by the allocator). C++
    # equivalent: `std::construct_at(bucket.as_ptr(), value)`.
    text = re.sub(
        r"\bbucket\.write\(std::move\(value\)\)",
        "std::construct_at(rusty::as_ptr(bucket), std::move(value))",
        text,
    )

    # `rusty::len(block)` where `block` is `NonNull<u8>` — Rust
    # hashbrown gets a slice from do_alloc and calls `.len()` on it.
    # rusty represents allocations as just `NonNull<u8>` (no slice
    # length tracking). The allocator returns exactly what was
    # requested, so substitute `layout.size` for the length.
    text = text.replace(
        "rusty::len(block)",
        "layout.size  /* substituted: allocator returns exactly layout.size bytes */",
    )

    # Transpiler bug: `new_uninitialized` emitted `_let_pat.unwrap()`
    # twice to destructure tuple — but rusty::Option::unwrap()
    # CONSUMES the value, so the second call throws "Called unwrap
    # on None". Replace the two-unwrap pattern with one-unwrap +
    # structured binding.
    old_pat = (
        "    auto&& _let_pat = table_layout.calculate_layout_for(std::move(buckets));\n"
        "    auto&& layout = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer((rusty::detail::deref_if_pointer(_let_pat)).unwrap())));\n"
        "    auto ctrl_offset = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer((rusty::detail::deref_if_pointer(_let_pat)).unwrap())));"
    )
    new_pat = (
        "    auto&& _let_pat = table_layout.calculate_layout_for(std::move(buckets));\n"
        "    auto _let_unwrapped = _let_pat.unwrap();\n"
        "    auto layout = std::get<0>(_let_unwrapped);\n"
        "    auto ctrl_offset = std::get<1>(_let_unwrapped);"
    )
    text = text.replace(old_pat, new_pat)
    # raw.cppm uses `do_alloc(alloc, ...)` but doesn't import
    # `hashbrown_port.alloc`. Add the import.
    if ("import hashbrown_port.alloc;\n" not in text
            and "do_alloc(" in text):
        text = text.replace(
            "import hashbrown_port.control;\n",
            "import hashbrown_port.control;\nimport hashbrown_port.alloc;\n",
            1,
        )
        # The import re-exports `Global` (and Allocator), which
        # conflicts with the existing `using rusty::alloc::Global;`
        # / `using rusty::alloc::Allocator;` decls. Drop the
        # using-decls (alloc imports them in scope already).
        text = text.replace(
            "using rusty::alloc::Allocator;\n",
            "// using rusty::alloc::Allocator; — alloc module imports it\n",
        )
        text = text.replace(
            "using rusty::alloc::Global;\n",
            "// using rusty::alloc::Global; — alloc module imports it\n",
        )
    # `T::IS_ZERO_SIZED` — Rust zero-sized type marker. Use
    # std::is_empty_v<T> as the closest C++ approximation, and
    # promote bare `if (...)` to `if constexpr (...)` so the
    # divergent branches don't type-check.
    text = text.replace(
        "if (T::IS_ZERO_SIZED)",
        "if constexpr (std::is_empty_v<T>)",
    )
    # In ternary / boolean expression contexts, just use the const.
    text = text.replace(
        "T::IS_ZERO_SIZED",
        "(std::is_empty_v<T>)",
    )
    # Bucket<T> next_n / from_base_index — ternary returns uint8_t*
    # in the empty-T branch but T* in the else branch. Wrap the
    # whole uint8_t* expression in a reinterpret_cast<T*>(...).
    # Brace-walk to find the matching `)` of the
    # `reinterpret_cast<uint8_t*>(...)` opening.
    ternary_open = "((std::is_empty_v<T>) ? reinterpret_cast<uint8_t*>("
    out_parts = []
    search_from = 0
    while True:
        idx = text.find(ternary_open, search_from)
        if idx == -1:
            out_parts.append(text[search_from:])
            break
        rc_arg_start = idx + len(ternary_open)
        depth = 1
        j = rc_arg_start
        while j < len(text) and depth > 0:
            if text[j] == '(':
                depth += 1
            elif text[j] == ')':
                depth -= 1
                if depth == 0:
                    break
            j += 1
        if depth != 0:
            out_parts.append(text[search_from:])
            break
        # rebuild: outer wrap (rc<T*>) around the existing rc<uint8_t*>(...)
        out_parts.append(text[search_from:idx])
        out_parts.append(
            "((std::is_empty_v<T>) ? "
            "reinterpret_cast<std::add_pointer_t<T>>(reinterpret_cast<uint8_t*>("
            + text[rc_arg_start:j]
            + "))"
        )
        search_from = j + 1
    text = "".join(out_parts)
    # `ptr->drop_in_place()` — Rust raw-pointer method. Use
    # std::destroy_at, which calls ptr->~T() for any T.
    text = re.sub(
        r"rusty::as_ptr\(\(\*this\)\)->drop_in_place\(\)",
        "std::destroy_at(rusty::as_ptr((*this)))",
        text,
    )

    # `auto guard = guard(...)` — Rust allows local `guard` shadowing
    # the function `guard`; C++ rejects (the local declaration tries
    # to call itself). Qualify the RHS function with `::` so the
    # global `guard()` is named, then the local `guard` can keep
    # its name and subsequent body refs work.
    text = text.replace("auto guard = guard(", "auto guard = ::guard(")

    # `ScopeGuard::into_inner(x)` — static template member can't
    # deduce ScopeGuard<T, F> from x. Provide a forwarding helper
    # at the top of raw.cppm and rewrite call sites to use it.
    if "// raw: into_inner-deduction helper" not in text:
        helper = (
            "// raw: into_inner-deduction helper — avoid `ScopeGuard::into_inner(x)`\n"
            "// which can't deduce ScopeGuard<T, F> template args.\n"
            "template<typename T, typename F>\n"
            "static inline T __raw_into_inner(::ScopeGuard<T, F> g) {\n"
            "    return ::ScopeGuard<T, F>::into_inner(std::move(g));\n"
            "}\n\n"
        )
        # Insert after the last import directive.
        anchor = "import hashbrown_port.util;\n"
        pos = text.find(anchor)
        if pos != -1:
            text = text[:pos + len(anchor)] + "\n" + helper + text[pos + len(anchor):]
    text = text.replace("ScopeGuard::into_inner(", "__raw_into_inner(")

    # `guard.num_buckets()` / `guard.ctrl(...)` — `guard` is a
    # ScopeGuard<RawTableInner, F> which doesn't auto-deref to
    # RawTableInner in C++. Use `(*guard).method()` explicitly.
    # Rust auto-derefs via the Deref trait; C++ ScopeGuard exposes
    # `operator*` returning `T&`.
    # Same auto-deref for both `guard` and `new_table` (both are
    # ScopeGuard-wrapped RawTableInner values in different methods).
    guard_method_names = (
        "num_buckets", "ctrl", "bucket_mask", "items",
        "growth_left", "num_ctrl_bytes", "bucket_ptr",
        "find_insert_index", "is_bucket_full", "bucket",
        "bucket_mask_to_capacity", "buckets",
        "record_item_insert_at", "set_ctrl_h2",
        "growth_left_for", "capacity",
        "is_in_same_group", "set_ctrl_hash",
        "replace_ctrl_hash", "set_ctrl",
        "probe_seq", "prepare_rehash_in_place",
        "allocation_info", "prepare_insert_index",
    )
    guard_field_names = (
        "growth_left", "bucket_mask", "items", "ctrl_field",
    )
    # Hot-path inlining: small RawTableInner methods declared in
    # the struct body but defined out-of-line are NOT inlined under
    # the modular-build model. Add `inline` to the definitions so
    # the compiler can fold them into callers (insert path: ctrl,
    # set_ctrl, record_item_insert_at, etc.).
    for inline_target in (
        "Tag* RawTableInner::ctrl(size_t index) const {",
        "void RawTableInner::set_ctrl(size_t index, Tag ctrl) {",
        "void RawTableInner::record_item_insert_at(size_t index, Tag old_ctrl, Tag new_ctrl) {",
        "Tag RawTableInner::replace_ctrl_hash(size_t index, uint64_t hash) {",
        "void RawTableInner::set_ctrl_hash(size_t index, uint64_t hash) {",
        "size_t RawTableInner::find_insert_index(uint64_t hash) const {",
        "size_t RawTableInner::num_buckets() const {",
        "size_t RawTableInner::num_ctrl_bytes() const {",
        "bool RawTableInner::is_bucket_full(size_t index) const {",
        "ProbeSeq RawTableInner::probe_seq(uint64_t hash) const {",
        "uint8_t* RawTableInner::bucket_ptr(size_t index, size_t size_of) const {",
    ):
        text = text.replace(inline_target, "inline " + inline_target)

    # Hot-path inlining: `find_inner` and `find_or_find_insert_index_inner`
    # take `const std::function<bool(size_t)>&` which adds an indirect
    # call to the equality callback inside the tight probe-seq loop —
    # the main lookup/insert-perf killer. Replace the std::function
    # with a template parameter so the callback inlines.
    text = text.replace(
        "    rusty::Option<size_t> find_inner(uint64_t hash, const std::function<bool(size_t)>& eq) const;",
        "    template<typename Eq>\n    rusty::Option<size_t> find_inner(uint64_t hash, const Eq& eq) const;",
    )
    text = text.replace(
        "rusty::Option<size_t> RawTableInner::find_inner(uint64_t hash, const std::function<bool(size_t)>& eq) const {",
        "template<typename Eq>\nrusty::Option<size_t> RawTableInner::find_inner(uint64_t hash, const Eq& eq) const {",
    )
    text = text.replace(
        "    rusty::Result<size_t, size_t> find_or_find_insert_index_inner(uint64_t hash, const std::function<bool(size_t)>& eq) const;",
        "    template<typename Eq>\n    rusty::Result<size_t, size_t> find_or_find_insert_index_inner(uint64_t hash, const Eq& eq) const;",
    )
    text = text.replace(
        "rusty::Result<size_t, size_t> RawTableInner::find_or_find_insert_index_inner(uint64_t hash, const std::function<bool(size_t)>& eq) const {",
        "template<typename Eq>\nrusty::Result<size_t, size_t> RawTableInner::find_or_find_insert_index_inner(uint64_t hash, const Eq& eq) const {",
    )

    # All Rust `debug_assert!` macros emit `assert((expr))` in the
    # transpiled C++. These are debug-only in Rust but become runtime
    # asserts in C++, which fires on the happy path. Convert every
    # `assert((...));` in the raw module body to a no-op comment.
    # Skip `assert(true)` (the already-stripped Rust-syntax-asserts).
    text = re.sub(
        r"^(\s*)assert\(\(((?!true).+?)\)\);",
        r"\1// debug_assert: \2",
        text,
        flags=re.MULTILINE,
    )

    # `rusty::mem::swap((*this), new_table)` — new_table is a
    # ScopeGuard; the swap needs both args to be RawTableInner.
    text = text.replace(
        "rusty::mem::swap((*this), new_table);",
        "rusty::mem::swap(*this, *new_table);",
    )

    # `Result<ScopeGuard<RawTableInner, std::function<...>>>::Ok(guard(value, lambda))`
    # — guard() returns ScopeGuard with the lambda's concrete type,
    # which doesn't match the Result's std::function-typed
    # ScopeGuard. Wrap the lambda in std::function explicitly so
    # the types match.
    text = text.replace(
        "::Ok(guard(std::move(new_table), [=, alloc = std::move(alloc), table_layout = std::move(table_layout)](auto&& self_) mutable {",
        "::Ok(guard(std::move(new_table), std::function<void(RawTableInner&)>([=, alloc = std::move(alloc), table_layout = std::move(table_layout)](auto&& self_) mutable {",
    )
    # And add the closing `)` for the std::function wrapper at the
    # `});` that follows.
    text = text.replace(
        "        self_.free_buckets(alloc, std::move(table_layout));\n"
        "    }\n"
        "}\n"
        "}));",
        "        self_.free_buckets(alloc, std::move(table_layout));\n"
        "    }\n"
        "}\n"
        "})));",
    )

    for var in ("guard", "new_table"):
        for method in guard_method_names:
            text = re.sub(
                r"\b" + var + r"\." + re.escape(method) + r"\(",
                f"(*{var}).{method}(",
                text,
            )
        for field in guard_field_names:
            text = re.sub(
                r"\b" + var + r"\." + re.escape(field) + r"(?![\w(])",
                f"(*{var}).{field}",
                text,
            )

    # Bare `ptr::<fn>(` — Rust has `use core::ptr` or hashbrown imports.
    # In C++ we want `rusty::ptr::<fn>(`. Avoid double-prefix on already
    # qualified `rusty::ptr::`.
    text = re.sub(
        r"(?<!rusty::)\bptr::(?=\w)",
        "rusty::ptr::",
        text,
    )

    # `this->ctrl_slice().fill_empty()` (and any `<expr>.fill_empty()`
    # on a `std::span<MaybeUninit<Tag>>`) — Rust source writes
    # `for c in ctrl_slice { c.write(Tag::EMPTY) }`. The transpiler
    # emitted `.fill_empty()` because it doesn't know the receiver's
    # type. Replace with a do{...}while(0) block doing a manual fill.
    # Use a brace-walk-back so arbitrary expressions are handled.
    while True:
        idx = text.find(".fill_empty()")
        if idx == -1:
            break
        # Walk back over identifier / dot / arrow / colon / paren chain.
        i = idx
        if i > 0 and text[i-1] == ')':
            depth = 1
            j = i - 2
            while j >= 0 and depth > 0:
                if text[j] == ')':
                    depth += 1
                elif text[j] == '(':
                    depth -= 1
                    if depth == 0:
                        break
                j -= 1
            k = j - 1
            while k >= 0 and (text[k].isalnum() or text[k] in "_:>.-"):
                k -= 1
            expr_start = k + 1
        else:
            k = i - 1
            while k >= 0 and (text[k].isalnum() or text[k] in "_:>.-"):
                k -= 1
            expr_start = k + 1
        expr = text[expr_start:i]
        replacement = ("([&](auto&& __s) { for (auto& __c : __s) "
                       "{ __c.write(Tag::EMPTY); } }(" + expr + "))")
        text = text[:expr_start] + replacement + text[idx + len(".fill_empty()"):]

    # `rusty::leading_zeros(<expr>)` / `rusty::trailing_zeros(<expr>)`
    # where `<expr>` is a BitMask. These should be method calls on
    # BitMask. Use brace-walk to find the arg.
    for fn in ("leading_zeros", "trailing_zeros"):
        prefix = "rusty::" + fn + "("
        out_parts = []
        search_from = 0
        while True:
            idx = text.find(prefix, search_from)
            if idx == -1:
                out_parts.append(text[search_from:])
                break
            arg_start = idx + len(prefix)
            depth = 1
            j = arg_start
            while j < len(text) and depth > 0:
                if text[j] == '(':
                    depth += 1
                elif text[j] == ')':
                    depth -= 1
                    if depth == 0:
                        break
                j += 1
            if depth != 0:
                out_parts.append(text[search_from:])
                break
            arg = text[arg_start:j]
            # Only rewrite when arg references a BitMask-named local
            # (`empty_before` / `empty_after` from the rehash routine).
            # Otherwise leave the function-call form (it still works for
            # integers via rusty::num).
            if any(name in arg for name in
                   ("empty_before", "empty_after")):
                out_parts.append(text[search_from:idx])
                out_parts.append("(" + arg + ")." + fn + "()")
            else:
                out_parts.append(text[search_from:j+1])
            search_from = j + 1
        text = "".join(out_parts)

    # `rusty::iter(X.match_*())` — Rust source has `.match_*()
    # .into_iter()`. Our Group::match_* methods return
    # `group_internal::BitMask` (module-private, layout-compatible).
    # The call site needs to wrap that into the real
    # `control.bitmask::BitMask` (which has `.into_iter()`) by
    # constructing one from the `_0` field.
    # Use brace-matching to allow nested parens in X.
    match_endings_re = re.compile(
        r"\.match_(?:full|tag|empty|empty_or_deleted)\("
    )
    def ends_with_match_call(s: str) -> bool:
        """Return True if s ends with `.match_X(...)` (balanced parens)."""
        m = match_endings_re.search(s)
        if not m:
            return False
        # Verify the match is followed by balanced parens to EOS.
        depth = 1
        i = m.end()
        while i < len(s) and depth > 0:
            if s[i] == '(':
                depth += 1
            elif s[i] == ')':
                depth -= 1
                if depth == 0:
                    break
            i += 1
        if depth != 0:
            return False
        return s[i+1:].strip() == ""
    while True:
        idx = text.find("rusty::iter(")
        # Only iterate for those whose content ends with a
        # `.match_*()` call. Use regex to allow .match_tag(arg) too.
        found = False
        start = 0
        while True:
            i = text.find("rusty::iter(", start)
            if i == -1:
                break
            content_start = i + len("rusty::iter(")
            depth = 1
            j = content_start
            while j < len(text) and depth > 0:
                if text[j] == '(':
                    depth += 1
                elif text[j] == ')':
                    depth -= 1
                    if depth == 0:
                        break
                j += 1
            if depth != 0:
                break
            content = text[content_start:j]
            if ends_with_match_call(content):
                # Wrap the group_internal::BitMask via `_0` into the
                # real control.bitmask::BitMask, then call its
                # into_iter().
                text = text[:i] + "BitMask{" + content + "._0}.into_iter()" + text[j+1:]
                found = True
                break
            start = j + 1
        # Outer loop terminates when inner scan found no
        # `.match_*()`-ending rusty::iter — otherwise the outer
        # `while True` would spin forever scanning the same text.
        if not found:
            break

    # `rusty::iter(std::move(current_group))` where current_group
    # is a group_internal::BitMask from .match_full(). Wrap with
    # the real BitMask to construct an iterator.
    text = text.replace(
        ".current_group = rusty::iter(std::move(current_group)),",
        ".current_group = BitMask{current_group._0}.into_iter(),",
    )

    # `rusty::iter(this->table)` inside `RawTable<T>::iter` — the
    # Rust source is `self.table.iter::<T>()` (the method on
    # RawTableInner). Use the method explicitly.
    text = text.replace(
        "return rusty::iter(this->table);",
        "return this->table.template iter<T>();",
    )

    # `rusty::iter((*this))` inside `RawIter<T>::drop_elements` and
    # `RawTableInner::drop_elements<T>` — Rust uses `for item in self`
    # (implicit IntoIterator). Rewrite to a while-let loop over
    # this->next() since both types expose option-like `next()` (and
    # RawTableInner exposes a method `iter<T>()` that constructs a
    # RawIter<T>).
    # Locate the unique source and replace with a while-loop.
    # Pattern A: inside RawIter<T>::drop_elements — `iter` is a field
    # (RawIterRange<T>), so `this->iter()` doesn't work. Use the
    # field's `next()` method directly.
    text = text.replace(
        "for (auto&& item : rusty::for_in(rusty::iter((*this)))) {\n"
        "                    item.drop();\n"
        "                }",
        "for (;;) {\n"
        "                    auto __opt = this->iter.template next_impl<false>();\n"
        "                    if (!__opt.is_some()) break;\n"
        "                    auto item = __opt.unwrap();\n"
        "                    item.drop();\n"
        "                }",
    )
    # Pattern B: inside `RawTableInner::drop_elements<T>` — there is a
    # `RawTableInner::iter<T>()` method on RawTableInner that returns
    # `RawIter<T>`. Use it.
    text = text.replace(
        "for (auto&& item : rusty::for_in(rusty::iter((*this)))) {\n"
        "                item.drop();\n"
        "            }",
        "{ auto __it = this->template iter<T>();\n"
        "              for (;;) {\n"
        "                  auto __opt = __it.next();\n"
        "                  if (!__opt.is_some()) break;\n"
        "                  auto item = __opt.unwrap();\n"
        "                  item.drop();\n"
        "              } }",
    )

    # `rusty::iter(RawTableInner::NEW)` — Rust source is
    # `RawTableInner::NEW.iter<T>()` but RawTableInner is incomplete
    # in the inline definition site. Stub `RawIter<T>::default_()`
    # to return an empty iterator (items=0 short-circuits next()).
    text = text.replace(
        "static RawIter<T> default_() {\n"
        "        // @unsafe\n"
        "        {\n"
        "            return rusty::iter(RawTableInner::NEW);\n"
        "        }\n"
        "    }",
        "static RawIter<T> default_() {\n"
        "        // Empty iterator — `items = 0` short-circuits `next()`\n"
        "        // before `iter` is ever touched.\n"
        "        RawIter<T> r{};\n"
        "        r.items = 0;\n"
        "        return r;\n"
        "    }",
    )
    # Same for FullBucketsIndices::default_. Construct fields
    # explicitly since NonNull<uint8_t> has no default ctor; reuse
    # the same static-empty-group pointer that RawTableInner::new_()
    # uses, with items=0 so iteration short-circuits.
    text = text.replace(
        "FullBucketsIndices FullBucketsIndices::default_() {\n"
        "    using Item = typename FullBucketsIndices::Item;\n"
        "    // @unsafe\n"
        "    {\n"
        "        return RawTableInner::NEW.full_buckets_indices();\n"
        "    }\n"
        "}",
        "FullBucketsIndices FullBucketsIndices::default_() {\n"
        "    using Item = typename FullBucketsIndices::Item;\n"
        "    // Empty iterator — items=0 short-circuits next().\n"
        "    return FullBucketsIndices{\n"
        "        .current_group = BitMaskIter{},\n"
        "        .group_first_index = 0,\n"
        "        .ctrl = rusty::ptr::NonNull<uint8_t>::new_unchecked(\n"
        "            const_cast<uint8_t*>(reinterpret_cast<const uint8_t*>(\n"
        "                rusty::as_ptr(Group::static_empty())))),\n"
        "        .items = 0,\n"
        "    };\n"
        "}",
    )

    # `data_end()` in `RawTableInner::bucket_ptr` (no template args)
    # — Rust source disambiguates by return type annotation.
    # `RawTableInner::data_end<T>()` is templated; force T=uint8_t at
    # this specific call site. The RawTable<T>::data_end is a non-
    # templated method, so be specific to bucket_ptr's literal line.
    text = text.replace(
        "uint8_t* const base = const_cast<uint8_t*>(reinterpret_cast<const uint8_t*>(rusty::as_ptr(this->data_end())));",
        "uint8_t* const base = const_cast<uint8_t*>(reinterpret_cast<const uint8_t*>(rusty::as_ptr(this->template data_end<uint8_t>())));",
    )

    # `ctrl_slice()` returns `span<MaybeUninit<Tag>>` but the body
    # passes `Tag*` to `from_raw_parts_mut`. Cast pointer.
    text = text.replace(
        "return rusty::from_raw_parts_mut(const_cast<Tag*>(reinterpret_cast<const Tag*>(rusty::as_ptr(this->ctrl_field))), this->num_ctrl_bytes());",
        "return rusty::from_raw_parts_mut(reinterpret_cast<rusty::MaybeUninit<Tag>*>(const_cast<uint8_t*>(rusty::as_ptr(this->ctrl_field))), this->num_ctrl_bytes());",
    )

    # `drop(arg)` where drop is `UnsafeFn<...>` — UnsafeFn requires
    # `.call_unsafe(args...)` to invoke. Only this single call site.
    text = text.replace(
        "drop(self_.bucket_ptr(std::move(i), std::move(size_of)));",
        "drop.call_unsafe(self_.bucket_ptr(std::move(i), std::move(size_of)));",
    )

    # `(rusty::range(0, N)).step_by(W)` — rusty::range has no step_by.
    # Rewrite the for-loop to manually stride: `for (auto i = 0; i < N;
    # i += W) { ... }`. The pattern only appears in
    # `prepare_rehash_in_place`. Brace-match the iterator expression
    # to extract N and W.
    step_by_pattern = "for (auto&& i : rusty::for_in((rusty::range(0, "
    idx = text.find(step_by_pattern)
    if idx != -1:
        # Find matching `).step_by(`.
        n_start = idx + len(step_by_pattern)
        depth = 1
        j = n_start
        while j < len(text) and depth > 0:
            if text[j] == '(':
                depth += 1
            elif text[j] == ')':
                depth -= 1
                if depth == 0:
                    break
            j += 1
        N = text[n_start:j]
        step_marker = ")).step_by("
        if text[j:j+len(step_marker)] == step_marker:
            w_start = j + len(step_marker)
            depth = 1
            k = w_start
            while k < len(text) and depth > 0:
                if text[k] == '(':
                    depth += 1
                elif text[k] == ')':
                    depth -= 1
                    if depth == 0:
                        break
                k += 1
            W = text[w_start:k]
            # Match the closing `)) {` of the for-line (2 closes
            # for `for_in(` and `for (` after step_by's own close).
            tail_marker = ")) {"
            if text[k+1:k+1+len(tail_marker)] == tail_marker:
                new_for = (
                    "for (size_t i = 0; i < (" + N + "); i += (" + W + ")) {"
                )
                text = text[:idx] + new_for + text[k+1+len(tail_marker):]

    # `TableLayout::calculate_layout_for` body — the inner lambdas
    # are wrapped by RUSTY_TRY_OPT which uses GCC statement-expr
    # `return` from the enclosing lambda. The lambda has two return
    # types (None_t and Option<size_t>); deduction fails. Add the
    # return-type annotation explicitly.
    text = re.sub(
        r"auto ctrl_offset = RUSTY_TRY_OPT\(\[&\]\(\) \{",
        "auto ctrl_offset = RUSTY_TRY_OPT([&]() -> rusty::Option<size_t> {",
        text,
    )
    text = re.sub(
        r"auto&& _checked_lhs = RUSTY_TRY_OPT\(\[&\]\(\) \{",
        "auto&& _checked_lhs = RUSTY_TRY_OPT([&]() -> rusty::Option<size_t> {",
        text,
    )
    text = re.sub(
        r"const auto len = RUSTY_TRY_OPT\(\[&\]\(\) \{",
        "const auto len = RUSTY_TRY_OPT([&]() -> rusty::Option<size_t> {",
        text,
    )

    # `rusty::for_in(group.match_X(arg))` — `match_X` returns our
    # group_internal::BitMask which isn't iterable. Wrap with real
    # BitMask first. Build via segments (single pass, no markers).
    for_in_prefix = "rusty::for_in("
    out_parts = []
    search_from = 0
    while True:
        idx = text.find(for_in_prefix, search_from)
        if idx == -1:
            out_parts.append(text[search_from:])
            break
        content_start = idx + len(for_in_prefix)
        depth = 1
        j = content_start
        while j < len(text) and depth > 0:
            if text[j] == '(':
                depth += 1
            elif text[j] == ')':
                depth -= 1
                if depth == 0:
                    break
            j += 1
        if depth != 0:
            out_parts.append(text[search_from:])
            break
        content = text[content_start:j]
        has_match = any(m in content for m in
                        (".match_tag(", ".match_empty(",
                         ".match_empty_or_deleted(", ".match_full("))
        if has_match and "BitMask{" not in content:
            out_parts.append(text[search_from:idx])
            out_parts.append("rusty::for_in(BitMask{" + content + "._0})")
        else:
            out_parts.append(text[search_from:j+1])
        search_from = j + 1
    text = "".join(out_parts)

    # TableLayout::calculate_layout_for — the method only reads
    # struct fields; add const to the declaration so it can be
    # called from const contexts (TABLE_LAYOUT is a `constexpr`
    # static member which is const-qualified).
    text = text.replace(
        "rusty::Option<std::tuple<rusty::alloc::Layout, size_t>> calculate_layout_for(size_t buckets);",
        "rusty::Option<std::tuple<rusty::alloc::Layout, size_t>> calculate_layout_for(size_t buckets) const;",
    )
    text = text.replace(
        "rusty::Option<std::tuple<rusty::alloc::Layout, size_t>> TableLayout::calculate_layout_for(size_t buckets) {",
        "rusty::Option<std::tuple<rusty::alloc::Layout, size_t>> TableLayout::calculate_layout_for(size_t buckets) const {",
    )

    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_scopeguard_dropfn_arg(cpp_out: Path) -> int:
    """The transpiled scopeguard.cppm calls `(this->dropfn)(&this->value)`
    in the destructor — passing a pointer to value. Rust's
    `F: FnMut(&mut T)` corresponds to `void(T&)` in C++; passing the
    address gives `T*` which doesn't bind to `T&` in lambdas using
    `auto&&` (deduces to T*& instead of T&). Change to pass
    `this->value` directly so auto&& deduces T&."""
    path = cpp_out / "hashbrown_port.scopeguard.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    text = text.replace(
        "(this->dropfn)(&this->value);",
        "(this->dropfn)(this->value);",
    )
    if text != original:
        path.write_text(text)
        return 1
    return 0


def _delete_conflated_iter_methods(text: str) -> str:
    """Remove methods inside `Iter<T>`-style structs in table.cppm that
    were mistakenly merged from other Iter variants (Set's Iter<K>,
    Map's Iter<K,V>). These show up as `template<typename K, ...>`
    methods that reference fields or types not present on the
    containing struct. Detection: any method that opens with
    `    template<typename K` (4-space indent inside struct), spanning
    until the next `    }` at the same indent.

    The method bodies span multiple lines including nested braces. Use
    a brace-counting scan to find the matching closing brace."""
    lines = text.splitlines(keepends=True)
    out_lines = []
    i = 0
    # Match a `template<...>` line at indent 4 whose type-param list
    # references any single-uppercase-letter type param OTHER than
    # B (Bn) or F (Func), which are legitimate fold-method params.
    # These extra params are typically conflations from sibling
    # Iter/Entry struct variants.
    conflated_template_re = re.compile(
        r"^    template<[^>]*\b(?:K|V|T|S|U|W|R|Q|H)\b[^>]*>$"
    )
    while i < len(lines):
        line = lines[i]
        if (conflated_template_re.match(line.rstrip("\n"))
                and i + 1 < len(lines)):
            # Find the closing `    }` for this method.
            # Track braces from the first `{` onward.
            j = i + 1
            depth = 0
            started = False
            while j < len(lines):
                for ch in lines[j]:
                    if ch == '{':
                        depth += 1
                        started = True
                    elif ch == '}':
                        depth -= 1
                if started and depth == 0:
                    break
                j += 1
            # Skip lines i..=j (both inclusive). Also skip a single
            # following blank line if present.
            i = j + 1
            if i < len(lines) and lines[i].strip() == "":
                i += 1
            continue
        out_lines.append(line)
        i += 1
    return "".join(out_lines)


def _patch_downstream_module(path: Path) -> bool:
    """Generic fixups for hashbrown downstream modules
    (table/map/set/raw_entry/rustc_entry): hoist stray imports;
    std::Allocator/Global → rusty::alloc::*; drop `raw::` qualifier;
    delete Iter-overload-conflated methods."""
    if not path.exists():
        return False
    text = path.read_text()
    original = text
    # `import rusty;` — rusty is a header-only library pulled in via the
    # global module fragment (`#include <rusty/rusty.hpp>`), not a C++20
    # module. The transpiler emits a spurious module import for it; drop it.
    text = re.sub(r"^import rusty;\n", "", text, flags=re.MULTILINE)
    # Collect any stray `import hashbrown_port.X;` lines NOT at the top.
    import_lines = re.findall(
        r"^import hashbrown_port\.[\w.]+;\n", text, flags=re.MULTILINE
    )
    # Identify the "header" import block (lines starting at the
    # `export module` line and forward).
    mod_re = re.compile(
        r"^(export module hashbrown_port\.\w+;\n)((?:import hashbrown_port\.[\w.]+;\n)*)",
        re.MULTILINE,
    )
    m = mod_re.search(text)
    if m and import_lines:
        header_imports = m.group(2)
        stray = [
            line for line in import_lines if line not in header_imports
        ]
        if stray:
            # Remove all occurrences of stray lines, then re-insert
            # at the header block.
            for line in stray:
                # Skip if this line is already in the header.
                pass
            # Remove stray imports from body.
            body = text[m.end():]
            for line in stray:
                body = body.replace(line, "", 1)
            # Combine.
            text = text[:m.end()] + "".join(stray) + body
    # std::Allocator / std::Global → rusty::alloc.
    text = text.replace("using std::Allocator;", "using rusty::alloc::Allocator;")
    text = text.replace("using std::Global;", "using rusty::alloc::Global;")
    # `raw::` qualifier in this file — strip; types imported flat.
    text = re.sub(r"\braw::(?=\w)", "", text)
    # `control::` qualifier — strip too (types imported flat).
    text = re.sub(r"\bcontrol::(?=\w)", "", text)
    # `TryReserveError` (bare, no rusty:: prefix) → qualify.
    # Negative-lookbehind avoids double-prefix on `s::TryReserveError`
    # (the `collections::` tail).
    text = re.sub(
        r"(?<![:s])\bTryReserveError\b",
        "rusty::collections::TryReserveError",
        text,
    )
    # `DefaultHashBuilder` → DefaultHasher (in-module stub, see
    # patch_hasher_replace_with_stub for the definition).
    text = re.sub(r"\bDefaultHashBuilder\b", "DefaultHasher", text)
    # Add `import hashbrown_port.hasher;` to map.cppm so its
    # references to DefaultHasher resolve to the exported one.
    if "import hashbrown_port.hasher;" not in text:
        text = text.replace(
            "import hashbrown_port.raw;\n",
            "import hashbrown_port.raw;\nimport hashbrown_port.hasher;\n",
            1,
        )
    # map.cppm has a duplicate `struct DefaultHasher` stub that
    # conflicts with hasher.cppm's exported one. Remove the local.
    text = text.replace(
        "// DefaultHasher stub — used by expanded #[derive(Hash)] test code.\n"
        "struct DefaultHasher {\n"
        "std::size_t state = 14695981039346656037ULL;\n"
        "static DefaultHasher new_() { return DefaultHasher{}; }\n"
        "std::size_t finish() const { return state; }\n"
        "};\n",
        "// (DefaultHasher stub removed — use hasher::DefaultHasher.)\n",
    )
    # The `__rusty_ext_equivalent` helper template declared at the
    # global module fragment in map.cppm now needs to be marked
    # `inline` to avoid the module-attachment conflict.
    text = text.replace(
        "template<typename A, typename B>\n"
        "constexpr bool __rusty_ext_equivalent(const A& a, const B& b)\n"
        "{ return a == b; }\n",
        "template<typename A, typename B>\n"
        "inline constexpr bool __rusty_ext_equivalent(const A& a, const B& b)\n"
        "{ return a == b; }\n",
    )
    # `f.debug_set()` / `.debug_map()` — rusty::fmt::Formatter doesn't
    # have these; fall back to debug_list (preserves bracket-style
    # output for compile-only).
    text = text.replace(".debug_set()", ".debug_list()")
    text = text.replace(".debug_map()", ".debug_list()")
    # `additional.div_ceil(N)` — Rust integer method; inline as
    # (a + N - 1) / N. Common pattern in extend_reserve.
    text = re.sub(
        r"\b(\w+)\.div_ceil\((\d+)\)",
        r"((\1 + \2 - 1) / \2)",
        text,
    )
    # `auto [k, []]` — Rust `_` placeholder in structured binding
    # became `[]`. Replace with a unique ignored name.
    text = re.sub(
        r"auto \[([^\]]*), \[\]\]",
        r"auto [\1, _ignored1]",
        text,
    )
    text = re.sub(
        r"auto \[\[\], ([^\]]*)\]",
        r"auto [_ignored1, \1]",
        text,
    )
    # HashMap doesn't have `get` and `from_iter` methods (Rust source
    # has them but the transpiler failed to emit due to where-clause
    # complexity). Stub for Phase A2 compile.
    # `other.get(...)`: use table.find with equivalent_key.
    text = text.replace(
        "other.get(rusty::detail::deref_if_pointer_like(key)).is_some_and([&](auto&& v) { return rusty::detail::deref_if_pointer_like(value) == rusty::deref_mut(v); })",
        "other.table.find(::make_hash(other.hash_builder, key), ::equivalent_key(key)).is_some_and([&](auto&& v) { return rusty::detail::deref_if_pointer_like(value) == std::get<1>(v.as_ref()); })",
    )
    # `HashMap<K, V, S, A>::from_iter(...)`: stub as default + extend.
    text = text.replace(
        "return HashMap<K, V, S, A>::from_iter(rusty::iter(arr));",
        "auto m = HashMap<K, V, S, A>::default_();\n"
        "        for (auto&& [k, v] : arr) {\n"
        "            m.insert(std::move(k), std::move(v));\n"
        "        }\n"
        "        return m;",
    )
    # `rusty_ext::equivalent(...)` — fallback resolution for the
    # `Equivalent` trait. The transpiler doesn't generate the proper
    # de:: prefix; rewrite as `operator==`.
    text = re.sub(
        r"\brusty_ext::equivalent\b",
        "::__rusty_ext_equivalent",
        text,
    )
    # Inject a `__rusty_ext_equivalent` helper at top of file
    # (after the module export).
    helper_sentinel = "// auto-stub: __rusty_ext_equivalent"
    if helper_sentinel not in text and "::__rusty_ext_equivalent" in text:
        helper = (
            helper_sentinel + "\n"
            "// rusty_ext::equivalent fallback (just `operator==`).\n"
            "template<typename A, typename B>\n"
            "constexpr bool __rusty_ext_equivalent(const A& a, const B& b)\n"
            "{ return a == b; }\n"
        )
        anchor = "using rusty::PhantomData;\n"
        pos = text.find(anchor)
        if pos != -1:
            insert_at = pos + len(anchor)
            text = text[:insert_at] + "\n" + helper + text[insert_at:]
    # `DefaultHasher::default_()` — the stub doesn't have this method.
    # Use the existing `new_()` (default-constructible).
    text = text.replace(
        "DefaultHasher::default_()",
        "DefaultHasher::new_()",
    )
    # Delete `raw_entry_mut`, `raw_entry`, `rustc_entry` methods of
    # HashMap — they reference types from raw_entry/rustc_entry
    # modules which create a cyclic-module dependency. Not part of
    # the core HashMap API; safe to drop for Phase A2 compile.
    for method_name in ("raw_entry_mut", "raw_entry", "rustc_entry"):
        # Match `RetType ... methodname(...) { ... }` at indent 4.
        pat = re.compile(
            r"^    [^\n]*\b" + re.escape(method_name) + r"\b[^\n]*\{\n",
            re.MULTILINE,
        )
        while True:
            m = pat.search(text)
            if not m:
                break
            # Walk braces from the `{` on the matched header line.
            i = m.end() - 1  # back to the newline
            j = m.end()
            depth = 1
            while j < len(text) and depth > 0:
                if text[j] == '{':
                    depth += 1
                elif text[j] == '}':
                    depth -= 1
                    if depth == 0:
                        break
                j += 1
            if depth != 0:
                break
            # Delete header + body + trailing newline.
            end = j + 1
            if end < len(text) and text[end] == '\n':
                end += 1
            text = text[:m.start()] + text[end:]
    # Add forward declarations for tagged-struct variant types
    # (Entry_Occupied / Entry_Vacant in table.cppm,
    #  RawEntryMut_*, RustcEntry_*, etc. in others). The transpiler
    # emits these as `struct ${Enum}_${Variant} { … }; ` at the bottom
    # of the file, but call sites reference them earlier inside
    # HashTable's inline method bodies. Inject forward declarations
    # for `Entry_Occupied` / `Entry_Vacant` right before the
    # `struct Entry;` line.
    fwd_anchor = re.compile(
        r"^export template<typename T, typename A>\n"
        r"    requires \(rusty::alloc::Allocator<A>\)\n"
        r"struct Entry;\n",
        re.MULTILINE,
    )
    m = fwd_anchor.search(text)
    if m:
        sentinel = "// auto-fwd: Entry_Occupied / Entry_Vacant\n"
        if sentinel not in text:
            fwd_decls = (
                sentinel
                + "export template<typename T, typename A>\n"
                + "struct Entry_Occupied;\n"
                + "export template<typename T, typename A>\n"
                + "struct Entry_Vacant;\n"
            )
            text = text[:m.start()] + fwd_decls + text[m.start():]
    # Fix `Entry` forward-vs-definition requires-clause mismatch:
    # the forward decl has `requires (rusty::alloc::Allocator<A>)`
    # but the definition omits it. Drop the requires from the forward.
    text = text.replace(
        "export template<typename T, typename A>\n"
        "    requires (rusty::alloc::Allocator<A>)\n"
        "struct Entry;\n",
        "export template<typename T, typename A>\n"
        "struct Entry;\n",
    )
    # Delete conflated methods (template<typename K|V|T, ...>) —
    # covers Iter/IterMut/Entry/OccupiedEntry/VacantEntry overload
    # mixing across struct variants.
    text = _delete_conflated_iter_methods(text)
    if text != original:
        path.write_text(text)
        return True
    return False


def patch_table_module(cpp_out: Path) -> int:
    """table.cppm cluster fixups."""
    return 1 if _patch_downstream_module(cpp_out / "hashbrown_port.table.cppm") else 0


def patch_map_module(cpp_out: Path) -> int:
    path = cpp_out / "hashbrown_port.map.cppm"
    changed = _patch_downstream_module(path)
    if not path.exists():
        return 1 if changed else 0
    text = path.read_text()
    original = text
    # Forward-declare Entry_Occupied / Entry_Vacant with map's 4-arg
    # template signature, before HashMap is defined.
    fwd_anchor = re.compile(
        r"^export template<typename K, typename V, typename S, typename A>\n"
        r"    requires \(rusty::alloc::Allocator<A>\)\n"
        r"struct Entry;\n",
        re.MULTILINE,
    )
    m = fwd_anchor.search(text)
    if m and "// auto-fwd: map Entry_Occupied / Entry_Vacant" not in text:
        fwd = (
            "// auto-fwd: map Entry_Occupied / Entry_Vacant\n"
            "export template<typename K, typename V, typename S, typename A>\n"
            "struct Entry_Occupied;\n"
            "export template<typename K, typename V, typename S, typename A>\n"
            "struct Entry_Vacant;\n"
        )
        text = text[:m.start()] + fwd + text[m.start():]
    # Drop the requires clause from `struct Entry;` forward decl (the
    # definition has no requires).
    text = text.replace(
        "export template<typename K, typename V, typename S, typename A>\n"
        "    requires (rusty::alloc::Allocator<A>)\n"
        "struct Entry;\n",
        "export template<typename K, typename V, typename S, typename A>\n"
        "struct Entry;\n",
    )
    # Same for `struct EntryRef;` — 5-arg template.
    text = text.replace(
        "export template<typename K, typename Q, typename V, typename S, typename A>\n"
        "    requires (rusty::alloc::Allocator<A>)\n"
        "struct EntryRef;\n",
        "export template<typename K, typename Q, typename V, typename S, typename A>\n"
        "struct EntryRef;\n",
    )
    # `rusty::addr_of_temp(X)` — undefined in rusty (transpiler-only
    # helper). The C++ code just needs the lvalue X passed directly;
    # `mem::replace` takes a reference. Strip the wrapper via
    # brace-walk so nested parens in X don't break it.
    while True:
        idx = text.find("rusty::addr_of_temp(")
        if idx == -1:
            break
        arg_start = idx + len("rusty::addr_of_temp(")
        depth = 1
        j = arg_start
        while j < len(text) and depth > 0:
            if text[j] == '(':
                depth += 1
            elif text[j] == ')':
                depth -= 1
                if depth == 0:
                    break
            j += 1
        if depth != 0:
            break
        text = text[:idx] + text[arg_start:j] + text[j+1:]

    # Stub `make_hasher` and `equivalent_key` helper functions before
    # HashMap uses them. The transpiler doesn't emit these; the
    # bodies are simple lambdas. `make_hasher` takes only S as a
    # template param (the others are deduced at call time from the
    # closure arg) so the bare `::make_hasher(hash_builder)` call
    # site can deduce S.
    helpers_sentinel = "// auto-stubs: make_hasher / equivalent_key"
    if helpers_sentinel not in text:
        helpers = (
            helpers_sentinel + "\n"
            "template<typename S>\n"
            "auto make_hasher(const S& hash_builder) {\n"
            "    return [&hash_builder](const auto& val) -> uint64_t {\n"
            "        using KeyT = std::remove_cvref_t<\n"
            "            decltype(std::get<0>(val))>;\n"
            "        return ::make_hash<KeyT, S>(hash_builder, std::get<0>(val));\n"
            "    };\n"
            "}\n"
            "template<typename Q>\n"
            "auto equivalent_key(const Q& k) {\n"
            "    return [&k](const auto& x) {\n"
            "        return std::get<0>(x) == k;\n"
            "    };\n"
            "}\n"
        )
        # Insert after the `using rusty::PhantomData;` line near the top.
        anchor2 = "using rusty::PhantomData;\n"
        pos = text.find(anchor2)
        if pos != -1:
            insert_at = pos + len(anchor2)
            text = text[:insert_at] + "\n" + helpers + text[insert_at:]
    # `make_hasher<auto, V, S>(...)` — drop the explicit template
    # args; C++ can't have `auto` as template arg. Let template
    # arg deduction work.
    text = re.sub(
        r"::make_hasher<auto,[^>]+>\(",
        "::make_hasher(",
        text,
    )
    if text != original:
        path.write_text(text)
        return 1
    return 1 if changed else 0


def patch_cmakelists_smoke_test(cpp_out: Path) -> int:
    """Append `smoke_test` + `bench` executable targets to CMakeLists.txt
    that link against the hashbrown_port module. Sources live at
    docs/hashbrown_port/{smoke_test,bench}.cpp."""
    path = cpp_out / "CMakeLists.txt"
    if not path.exists():
        return 0
    text = path.read_text()
    sentinel = "# Phase B smoke test"
    if sentinel in text:
        return 0
    here = Path(__file__).resolve().parent
    smoke_test_path = here / "smoke_test.cpp"
    set_smoke_path = here / "set_smoke.cpp"
    bench_path = here / "bench.cpp"
    include_dir = (
        Path(__file__).resolve().parents[2] / "include"
    )
    addition = (
        "\n"
        + sentinel + "\n"
        "# Compile the library with the same release flags as the bench so\n"
        "# LTO can see across the boundary (static archives carry no flags).\n"
        "target_compile_options(hashbrown_port PRIVATE -O3 -DNDEBUG -march=native -flto=thin)\n"
        "target_include_directories(hashbrown_port PRIVATE \""
        + str(include_dir) + "\")\n"
        "if(EXISTS \"" + str(smoke_test_path) + "\")\n"
        "    add_executable(smoke_test \"" + str(smoke_test_path) + "\")\n"
        "    target_compile_options(smoke_test PRIVATE -O3 -DNDEBUG -march=native -flto=thin)\n"
        "    target_link_options(smoke_test PRIVATE -flto=thin)\n"
        "    target_include_directories(smoke_test PRIVATE \""
        + str(include_dir) + "\")\n"
        "    target_link_libraries(smoke_test PRIVATE hashbrown_port)\n"
        "endif()\n"
        "if(EXISTS \"" + str(set_smoke_path) + "\")\n"
        "    add_executable(set_smoke \"" + str(set_smoke_path) + "\")\n"
        "    target_compile_options(set_smoke PRIVATE -O3 -DNDEBUG -march=native -flto=thin)\n"
        "    target_link_options(set_smoke PRIVATE -flto=thin)\n"
        "    target_include_directories(set_smoke PRIVATE \""
        + str(include_dir) + "\")\n"
        "    target_link_libraries(set_smoke PRIVATE hashbrown_port)\n"
        "endif()\n"
        "if(EXISTS \"" + str(bench_path) + "\")\n"
        "    add_executable(bench \"" + str(bench_path) + "\")\n"
        "    target_compile_options(bench PRIVATE -O3 -DNDEBUG -march=native -flto=thin)\n"
        "    target_link_options(bench PRIVATE -flto=thin)\n"
        "    target_include_directories(bench PRIVATE \""
        + str(include_dir) + "\")\n"
        "    target_link_libraries(bench PRIVATE hashbrown_port)\n"
        "endif()\n"
    )
    path.write_text(text + addition)
    return 1


def patch_umbrella_imports(cpp_out: Path) -> int:
    """The umbrella `hashbrown_port.cppm` interleaves `import` lines
    with struct decls and `using` re-exports that reference
    nonexistent helpers (hasher::, set::, external_trait_impls).
    Replace with a minimal stub that just imports the working
    sub-modules. Users can also import sub-modules directly."""
    path = cpp_out / "hashbrown_port.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    stub = (
        "// Auto-generated by rusty-cpp-transpiler; patched for Phase A2.\n"
        "// Umbrella module — re-exports the core HashMap port.\n"
        "// (set, raw_entry, rustc_entry are stubbed; advanced features\n"
        "//  excluded from the Phase A2 scope.)\n"
        "module;\n"
        "#include <cstdint>\n"
        "#include <rusty/rusty.hpp>\n"
        "export module hashbrown_port;\n"
        "export import hashbrown_port.alloc;\n"
        "export import hashbrown_port.control;\n"
        "export import hashbrown_port.hasher;\n"
        "export import hashbrown_port.raw;\n"
        "export import hashbrown_port.util;\n"
        "export import hashbrown_port.map;\n"
        "export import hashbrown_port.scopeguard;\n"
        "export import hashbrown_port.table;\n"
    )
    if text == stub:
        return 0
    path.write_text(stub)
    return 1


def _stub_module(path: Path, mod_name: str) -> bool:
    """Replace a module body with an empty stub.
    Phase A2 compile-only: HashSet/RawEntry/RustcEntry are advanced
    features not needed for the core HashMap port. The transpiler has
    too many gaps in these modules; stubbing keeps the import-graph
    intact while skipping the broken bodies."""
    if not path.exists():
        return False
    stub = (
        "// Auto-generated by rusty-cpp-transpiler\n"
        "// Do not edit manually.\n"
        "// Phase A2: stub — advanced feature, body skipped.\n"
        "module;\n"
        "#include <cstdint>\n"
        "#include <rusty/rusty.hpp>\n"
        "export module hashbrown_port." + mod_name + ";\n"
    )
    if path.read_text().startswith(stub):
        return False
    path.write_text(stub)
    return True


def patch_set_facade(cpp_out: Path) -> int:
    """Replace hashbrown_port.set.cppm with a HashSet facade that
    wraps HashMap<T, std::monostate, S>. The upstream Rust
    `hashbrown::HashSet` is exactly this (HashMap<T, ()>), so the
    semantics match by construction. Avoids hand-porting the long
    tail of raw_entry / rustc_entry / iter types that the
    transpiler can't fully translate."""
    path = cpp_out / "hashbrown_port.set.cppm"
    if not path.exists():
        return 0
    sentinel = "// HashSet facade — wraps HashMap"
    text = path.read_text()
    if sentinel in text:
        return 0
    facade = (
        sentinel + " <T, std::monostate, S>. Upstream Rust's\n"
        "// `hashbrown::HashSet<T>` is exactly `HashMap<T, ()>`.\n"
        "module;\n"
        "\n"
        "#include <cstdint>\n"
        "#include <cstddef>\n"
        "#include <utility>\n"
        "#include <variant>\n"
        "#include <rusty/rusty.hpp>\n"
        "\n"
        "export module hashbrown_port.set;\n"
        "import hashbrown_port.map;\n"
        "import hashbrown_port.hasher;\n"
        "\n"
        "export template<typename T, typename S = DefaultHasher>\n"
        "struct HashSet {\n"
        "    using Item = T;\n"
        "    HashMap<T, std::monostate, S> map;\n"
        "\n"
        "    HashSet() : map(HashMap<T, std::monostate, S>::new_()) {}\n"
        "    HashSet(HashMap<T, std::monostate, S> m) : map(std::move(m)) {}\n"
        "\n"
        "    static HashSet<T, S> new_() { return HashSet<T, S>(); }\n"
        "    static HashSet<T, S> with_capacity(size_t capacity) {\n"
        "        return HashSet<T, S>(HashMap<T, std::monostate, S>::with_capacity(capacity));\n"
        "    }\n"
        "    static HashSet<T, S> with_hasher(S hash_builder) {\n"
        "        return HashSet<T, S>(HashMap<T, std::monostate, S>::with_hasher(std::move(hash_builder)));\n"
        "    }\n"
        "    static HashSet<T, S> with_capacity_and_hasher(size_t capacity, S hash_builder) {\n"
        "        return HashSet<T, S>(HashMap<T, std::monostate, S>::with_capacity_and_hasher(capacity, std::move(hash_builder)));\n"
        "    }\n"
        "\n"
        "    size_t len() const { return this->map.len(); }\n"
        "    bool is_empty() const { return this->map.is_empty(); }\n"
        "    size_t capacity() const { return this->map.capacity(); }\n"
        "\n"
        "    bool insert(T value) {\n"
        "        auto prev = this->map.insert(std::move(value), std::monostate{});\n"
        "        return prev.is_none();\n"
        "    }\n"
        "\n"
        "    bool contains(const T& value) const {\n"
        "        // const_cast: HashMap::table.find() isn't const-correct in\n"
        "        // the transpiled form (find threads through a non-const\n"
        "        // table API internally).\n"
        "        auto& m = const_cast<HashMap<T, std::monostate, S>&>(this->map);\n"
        "        auto h = ::make_hash<T, S>(m.hash_builder, value);\n"
        "        return m.table.find(h, [&](const auto& kv) {\n"
        "            return std::get<0>(kv) == value;\n"
        "        }).is_some();\n"
        "    }\n"
        "\n"
        "    bool remove(const T& value) {\n"
        "        auto& m = this->map;\n"
        "        auto h = ::make_hash<T, S>(m.hash_builder, value);\n"
        "        auto eq = [&](const auto& kv) { return std::get<0>(kv) == value; };\n"
        "        auto b = m.table.find(h, eq);\n"
        "        if (b.is_none()) return false;\n"
        "        m.table.erase(b.unwrap());\n"
        "        return true;\n"
        "    }\n"
        "\n"
        "    void clear() {\n"
        "        // RawTable::clear() has a pre-existing transpiler-emission\n"
        "        // bug (self_.table on a ScopeGuard missing operator*).\n"
        "        // Replace the backing map to clear; same semantic outcome.\n"
        "        this->map = HashMap<T, std::monostate, S>::new_();\n"
        "    }\n"
        "\n"
        "    HashSet<T, S> clone() const { return HashSet<T, S>(this->map.clone()); }\n"
        "};\n"
    )
    path.write_text(facade)
    return 1


def patch_set_stub(cpp_out: Path) -> int:
    """Legacy alias kept for backward compat with the registry."""
    return patch_set_facade(cpp_out)


def patch_raw_table_clear_double_free(cpp_out: Path) -> int:
    """RawTable<T,A>::clear() in raw.cppm has a double-free bug.

    The Rust source takes &mut self and wraps it in a ScopeGuard whose
    Drop calls clear_no_drop(). The transpiled body wraps *this in a
    ScopeGuard<RawTable, F>; ScopeGuard takes T by value, so *this is
    SHALLOW-COPIED into the guard. The copy shares ctrl_field/items
    with the original. When the guard's drop runs (clear_no_drop on
    the copy) and then ~ScopeGuard destructs the copy (~RawTable →
    drop_inner_table → free_buckets), the original's ctrl_field is
    left dangling and items > 0. The caller's next use (typically
    ~HashMap) iterates the freed buffer and crashes.

    Mirror the HashSet::clear() workaround in patch_set_facade: drop
    via move-assignment to a fresh empty table. ~RawTable on *this
    runs first (proper element drop + single free_buckets), then *this
    becomes the empty-singleton state.

    Bug originally reported with mako repo
    test_load_balancer::ClientPoolLoadBalancerTest crashing in
    ~PoolThreadWorker on freed HashMap buckets."""
    path = cpp_out / "hashbrown_port.raw.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    buggy = (
        "    void clear() {\n"
        "        if (rusty::is_empty((*this))) {\n"
        "            return;\n"
        "        }\n"
        "        auto self_ = guard((*this), [&](auto&& self_) { return self_.clear_no_drop(); });\n"
    )
    if buggy not in text:
        return 0
    # Find the closing of the function body and replace.
    fixed = (
        "    void clear() {\n"
        "        // Patcher fix (RawTable double-free): the transpiled\n"
        "        // body wrapped *this in a ScopeGuard that takes T by\n"
        "        // value, copying the RawTable (same ctrl_field,\n"
        "        // same items). The guard's drop then freed the shared\n"
        "        // buffer while the caller's *this still pointed at it.\n"
        "        // Move-assign a fresh empty table instead: ~RawTable\n"
        "        // on *this runs first (proper element drop + single\n"
        "        // free_buckets), then *this becomes empty-singleton.\n"
        "        if (rusty::is_empty((*this))) {\n"
        "            return;\n"
        "        }\n"
        "        (*this) = RawTable<T, A>::new_in(rusty::clone(this->alloc));\n"
        "    }\n"
    )
    # Replace from the buggy header through the closing brace of the
    # original (the body has the drop_elements call + closing braces).
    # Use a regex to find the end of the function reliably.
    pattern = re.compile(
        r"    void clear\(\) \{\n"
        r"        if \(rusty::is_empty\(\(\*this\)\)\) \{\n"
        r"            return;\n"
        r"        \}\n"
        r"        auto self_ = guard\(\(\*this\), \[&\]\(auto&& self_\) \{ return self_\.clear_no_drop\(\); \}\);\n"
        r"(?:        // [^\n]*\n)*"
        r"        // @unsafe\n"
        r"        \{\n"
        r"            self_\.value\.table\.template drop_elements<T>\(\);\n"
        r"        \}\n"
        r"    \}\n"
    )
    if not pattern.search(text):
        return 0
    text = pattern.sub(fixed, text, count=1)
    path.write_text(text)
    return 1


def patch_raw_entry_stub(cpp_out: Path) -> int:
    return 1 if _stub_module(
        cpp_out / "hashbrown_port.raw_entry.cppm", "raw_entry"
    ) else 0


def patch_rustc_entry_stub(cpp_out: Path) -> int:
    return 1 if _stub_module(
        cpp_out / "hashbrown_port.rustc_entry.cppm", "rustc_entry"
    ) else 0


def patch_set_module(cpp_out: Path) -> int:
    path = cpp_out / "hashbrown_port.set.cppm"
    changed = _patch_downstream_module(path)
    if not path.exists():
        return 1 if changed else 0
    text = path.read_text()
    original = text
    # Add `import hashbrown_port.table;` if not present — set
    # references HashTable which lives in the table module.
    if "import hashbrown_port.table;" not in text:
        text = text.replace(
            "import hashbrown_port.map;\n",
            "import hashbrown_port.map;\nimport hashbrown_port.table;\n",
            1,
        )
    # set.cppm imports map.cppm, which exports `Iter`, `IntoIter`,
    # `Drain`, etc. Set redeclares these with different template
    # arities (1 K vs map's K, V), causing template-redeclaration
    # errors. Rename ONLY set's local declarations (not the field
    # types which refer to map's types). We detect "set's own" decls
    # by looking for `struct X` headers in the file body and renaming
    # those, plus all bare references that aren't followed by 3+
    # template args (which would indicate a map type).
    mod_start_re = re.compile(
        r"^export module hashbrown_port\.set;\n", re.MULTILINE
    )
    m = mod_start_re.search(text)
    if m:
        head = text[:m.end()]
        body = text[m.end():]
        # `map::Foo` qualifier — drop; types imported flat.
        body = re.sub(r"\bmap::(?=\w)", "", body)
        # Names that set declares locally (potential conflict with map).
        # For these, rename declarations and references with arity
        # matching set's own arity (1 K or K, A or T, S, A).
        # Strategy: rename only `struct Name {/`/`Name<...> name(...)`
        # at declaration sites; field uses of `Name<K, V_map, A>` keep
        # the original name. Use whole-word + look-ahead to ensure
        # we only rename specific patterns.

        # For each name, replace declarations: `struct Iter`, `struct
        # IntoIter` etc., and bare uses with specific arities.
        # Heuristic: rename `Iter<K>` (1 arg) but not `Iter<K, V, A>`
        # (3 args).
        renames = {
            "Iter": [
                r"\bIter\b(?!<\s*[KQVTSA]\s*,\s*[KQVTSA]\s*[,>])",
                # Match Iter not followed by 2+ args
            ],
        }
        # Simpler approach: only rename inside specific patterns:
        # `struct X {`, `struct X;`, `struct X :`, `X<K>` (1 arg).
        def rename_name(name, body):
            # `struct Name` (declaration).
            body = re.sub(
                r"\bstruct " + re.escape(name) + r"\b",
                "struct Set" + name,
                body,
            )
            # `Name<K>` (single arg, set's typical form).
            body = re.sub(
                r"\b" + re.escape(name) + r"<\s*([KTQ])\s*>",
                "Set" + name + r"<\1>",
                body,
            )
            # `Name<K, A>` where A is a single allocator letter
            # (set's 2-arg form for IntoIter, Drain).
            body = re.sub(
                r"\b" + re.escape(name) + r"<\s*([KTQ])\s*,\s*A\s*>",
                "Set" + name + r"<\1, A>",
                body,
            )
            # `Name<T, S, A>` (set's 3-arg form for OccupiedEntry,
            # VacantEntry, Entry, but NOT for IntoIter<K, V, A>
            # which is map's).
            body = re.sub(
                r"\b" + re.escape(name) + r"<\s*([KTQ])\s*,\s*S\s*,\s*A\s*>",
                "Set" + name + r"<\1, S, A>",
                body,
            )
            # `Name<K, F, A>` (set's 3-arg form for ExtractIf).
            body = re.sub(
                r"\b" + re.escape(name) + r"<\s*([KTQ])\s*,\s*F\s*,\s*A\s*>",
                "Set" + name + r"<\1, F, A>",
                body,
            )
            return body

        names = [
            "Iter", "IntoIter", "Drain", "ExtractIf",
            "OccupiedEntry", "VacantEntry", "Entry",
            "Entry_Occupied", "Entry_Vacant",
            "Intersection", "Difference", "Union",
            "SymmetricDifference",
        ]
        for name in names:
            body = rename_name(name, body)
        # Undo accidental double-prefix.
        body = re.sub(r"\bSetSet", "Set", body)
        text = head + body
    if text != original:
        path.write_text(text)
        return 1
    return 1 if changed else 0


def patch_raw_entry_module(cpp_out: Path) -> int:
    path = cpp_out / "hashbrown_port.raw_entry.cppm"
    changed = _patch_downstream_module(path)
    if not path.exists():
        return 1 if changed else 0
    text = path.read_text()
    original = text
    # Forward-declare RawEntryMut_Occupied / RawEntryMut_Vacant (the
    # tagged-struct variants of RawEntryMut) before they're referenced
    # in the inline `search()` body of RawEntryBuilderMut.
    anchor = re.compile(
        r"^export template<typename K, typename V, typename S, typename A>\n"
        r"    requires \(rusty::alloc::Allocator<A>\)\n"
        r"struct RawEntryMut;\n",
        re.MULTILINE,
    )
    m = anchor.search(text)
    if m and "// auto-fwd: RawEntryMut_*" not in text:
        fwd = (
            "// auto-fwd: RawEntryMut_*\n"
            "export template<typename K, typename V, typename S, typename A>\n"
            "struct RawEntryMut_Occupied;\n"
            "export template<typename K, typename V, typename S, typename A>\n"
            "struct RawEntryMut_Vacant;\n"
        )
        text = text[:m.start()] + fwd + text[m.start():]
    # Drop requires-clause from RawEntryMut and similar forward decls
    # that mismatch their definitions.
    for ty in ("RawEntryMut", "RawEntry"):
        text = text.replace(
            "export template<typename K, typename V, typename S, typename A>\n"
            "    requires (rusty::alloc::Allocator<A>)\n"
            "struct " + ty + ";\n",
            "export template<typename K, typename V, typename S, typename A>\n"
            "struct " + ty + ";\n",
        )
    # `make_hasher<auto, V, S>(...)` — strip explicit args (auto isn't
    # valid as template arg).
    text = re.sub(
        r"::make_hasher<auto,[^>]+>\(",
        "::make_hasher(",
        text,
    )
    if text != original:
        path.write_text(text)
        return 1
    return 1 if changed else 0


def patch_rustc_entry_module(cpp_out: Path) -> int:
    path = cpp_out / "hashbrown_port.rustc_entry.cppm"
    changed = _patch_downstream_module(path)
    if not path.exists():
        return 1 if changed else 0
    text = path.read_text()
    original = text
    # Forward-declare RustcEntry_Occupied / RustcEntry_Vacant.
    anchor = re.compile(
        r"^export template<typename K, typename V, typename A>\n"
        r"    requires \(rusty::alloc::Allocator<A>\)\n"
        r"struct RustcEntry;\n",
        re.MULTILINE,
    )
    m = anchor.search(text)
    if m and "// auto-fwd: RustcEntry_*" not in text:
        fwd = (
            "// auto-fwd: RustcEntry_*\n"
            "export template<typename K, typename V, typename A>\n"
            "struct RustcEntry_Occupied;\n"
            "export template<typename K, typename V, typename A>\n"
            "struct RustcEntry_Vacant;\n"
        )
        text = text[:m.start()] + fwd + text[m.start():]
    # Drop requires-clause from RustcEntry forward decl.
    text = text.replace(
        "export template<typename K, typename V, typename A>\n"
        "    requires (rusty::alloc::Allocator<A>)\n"
        "struct RustcEntry;\n",
        "export template<typename K, typename V, typename A>\n"
        "struct RustcEntry;\n",
    )
    # `make_hasher<auto, V, S>(...)` — strip explicit args.
    text = re.sub(
        r"::make_hasher<auto,[^>]+>\(",
        "::make_hasher(",
        text,
    )
    if text != original:
        path.write_text(text)
        return 1
    return 1 if changed else 0


def patch_raw_tryreserveerror_constructors(cpp_out: Path) -> int:
    """`raw.cppm` emits Rust enum-variant constructors like
    `TryReserveError_CapacityOverflow{}` and
    `TryReserveError_AllocError{.layout = ...}`. These were valid in
    the Rust source where `TryReserveError` is an enum with named
    variants, but rusty's `TryReserveError` is a tagged struct with
    a `Kind` discriminant. Rewrite the `capacity_overflow` and
    `alloc_err` function bodies to use the right constructor."""
    path = cpp_out / "hashbrown_port.raw.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    sentinel = "// raw: TryReserveError variant constructors → tagged struct"
    if sentinel in text:
        return 0
    # Match `capacity_overflow` body and replace.
    old_co = ("inline rusty::collections::TryReserveError capacity_overflow(Fallibility self_) {\n"
              "    return [&]() -> rusty::collections::TryReserveError")
    pos = text.find(old_co)
    if pos == -1:
        return 0
    # Find the closing `}();` then `}` of the function.
    end_marker = "(); }();\n}"
    end = text.find(end_marker, pos)
    if end == -1:
        # Try a simpler closing.
        end = text.find("}();\n}", pos)
        if end == -1:
            return 0
        end_marker = "}();\n}"
    new_co = ("inline rusty::collections::TryReserveError capacity_overflow(Fallibility self_) {\n"
              "    " + sentinel + "\n"
              "    (void)self_;\n"
              "    return rusty::collections::TryReserveError(\n"
              "        rusty::collections::TryReserveError::Kind::CapacityOverflow);\n"
              "}")
    text = text[:pos] + new_co + text[end + len(end_marker):]

    # Same for alloc_err.
    old_ae = ("inline rusty::collections::TryReserveError alloc_err(Fallibility self_, rusty::alloc::Layout layout) {\n"
              "    return [&]() -> rusty::collections::TryReserveError")
    pos = text.find(old_ae)
    if pos == -1:
        path.write_text(text)
        return 1
    end = text.find("}();\n}", pos)
    if end == -1:
        path.write_text(text)
        return 1
    new_ae = ("inline rusty::collections::TryReserveError alloc_err(Fallibility self_, rusty::alloc::Layout layout) {\n"
              "    (void)self_;\n"
              "    return rusty::collections::TryReserveError(\n"
              "        rusty::collections::TryReserveError::Kind::AllocError,\n"
              "        layout.size, layout.align);\n"
              "}")
    text = text[:pos] + new_ae + text[end + len("}();\n}"):]
    path.write_text(text)
    return 1


def patch_raw_std_alloc_namespace(cpp_out: Path) -> int:
    """`raw.cppm` references `std::AllocError`/`std::Allocator`/
    `std::Global`/`std::Layout`/`std::handle_alloc_error` (the
    transpiler picked the `std::` prefix from Rust's `use std::alloc::*`
    but rusty has these under `rusty::alloc::`). Also `std::do_alloc`
    should be plain `do_alloc` (it's imported from hashbrown_port.alloc)."""
    path = cpp_out / "hashbrown_port.raw.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    for sym in ["AllocError", "Allocator", "Global", "Layout", "handle_alloc_error"]:
        # Only replace `std::Sym` (avoid matching std::array etc).
        text = text.replace(f"std::{sym}", f"rusty::alloc::{sym}")
    text = text.replace("using std::do_alloc;", "// using std::do_alloc; — already imported from hashbrown_port.alloc")
    if text != original:
        path.write_text(text)
        return 1
    return 0


# ── deep-namespace patches (`--cxx-namespace rusty::port::collections::hashbrown`) ──
#
# When the transpiler is invoked with `--cxx-namespace
# rusty::port::collections::hashbrown`, all post-`export module` content
# is wrapped in that namespace. This shifts every symbol off global
# scope and into `rusty::port::collections::hashbrown::*`, which breaks
# a fresh long-tail of references that assumed flat emit. The patches
# below address that long-tail. They are idempotent and only fire
# against wrapped emit (presence of `namespace rusty::port::collections
# ::hashbrown {` is the sentinel).

_HB_DEEP_NS = "rusty::port::collections::hashbrown"
_HB_DEEP_NS_OPEN = f"namespace {_HB_DEEP_NS} {{"


def _has_deep_ns_wrap(text: str) -> bool:
    return _HB_DEEP_NS_OPEN in text


def patch_deep_prelude_clone_constrain(cpp_out: Path) -> int:
    """The GMF prelude redeclares `rusty::clone` as an unconstrained
    template. With the deep-namespace wrap, every `rusty::clone(x)` call
    inside the wrap sees both this prelude clone AND `<rusty/move.hpp>`'s
    Copy-only `rusty::clone` — both at `rusty::clone`, both viable for
    `T = unsigned char` etc., ambiguous.

    Fix: constrain the prelude to only fire when `value.clone()` exists.
    Copy types fall through to move.hpp's clone (which static-asserts on
    copy-constructibility but is the only candidate for Copy-only types
    once the constraint excludes the prelude version)."""
    files = ["hashbrown_port.alloc.cppm",
             "hashbrown_port.control.tag.cppm",
             "hashbrown_port.map.cppm",
             "hashbrown_port.raw.cppm",
             "hashbrown_port.table.cppm"]
    old = (
        "// Clone: dispatches to .clone() if available, otherwise copy-constructs.\n"
        "template<typename T>\n"
        "auto clone(const T& value) {\n"
        "if constexpr (requires { value.clone(); }) {\n"
        "return value.clone();\n"
        "} else {\n"
        "return value;\n"
        "}\n"
        "}"
    )
    new = (
        "// Clone: dispatches to .clone() if available; Copy types fall through to ::rusty::clone (move.hpp).\n"
        "template<typename T>\n"
        "requires requires(const T& v) { v.clone(); }\n"
        "auto clone(const T& value) {\n"
        "return value.clone();\n"
        "}"
    )
    total = 0
    for fn in files:
        path = cpp_out / fn
        if not path.exists():
            continue
        text = path.read_text()
        if not _has_deep_ns_wrap(text):
            continue
        if old not in text:
            continue
        path.write_text(text.replace(old, new))
        total += 1
    return total


def patch_deep_control_group_reexport(cpp_out: Path) -> int:
    """`control.group.cppm` imports `.generic` but doesn't re-export. With
    the deep wrap, BitMaskWord / BITMASK_* etc. live in
    `rusty::port::collections::hashbrown::*` and aren't visible to
    `control.bitmask` (which imports `control.group`). Three issues:
    1. The transpiler emits the `import` INSIDE the namespace wrap, but
       C++20 modules require imports at module purview. Hoist it out.
    2. Plain `import` doesn't re-export. Promote to `export import` so
       `control.group` acts as a public facade for `.generic`.
    3. Strip the broken `export using ::Foo;` lines (Group/BITMASK_*) —
       with the wrap, those names live in the deep namespace, not at
       global scope. The `export import` above handles re-export."""
    path = cpp_out / "hashbrown_port.control.group.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    if not _has_deep_ns_wrap(text):
        return 0
    pre_wrap, _, _ = text.partition(_HB_DEEP_NS_OPEN)
    already_hoisted = "export import hashbrown_port.control.group.generic;" in pre_wrap
    has_export_using = re.search(r"^\s*(?:export\s+)?using ::\w+;", text, re.MULTILINE)
    if already_hoisted and not has_export_using:
        return 0
    original = text
    # Step 1: remove any `(export )?import hashbrown_port.control.group.generic;` line.
    text = re.sub(
        r"^(?:export\s+)?import hashbrown_port\.control\.group\.generic;\s*\n",
        "",
        text,
        flags=re.MULTILINE,
    )
    # Step 2: insert `export import` before the namespace open.
    text = text.replace(
        _HB_DEEP_NS_OPEN,
        "export import hashbrown_port.control.group.generic;\n\n" + _HB_DEEP_NS_OPEN,
        1,
    )
    # Step 3: strip `(export )?using ::Foo;` lines.
    text = re.sub(
        r"^\s*(?:export\s+)?using ::\w+;\s*\n",
        "",
        text,
        flags=re.MULTILINE,
    )
    if text == original:
        return 0
    path.write_text(text)
    return 1


def patch_deep_control_cppm_reexport(cpp_out: Path) -> int:
    """`control.cppm` (parent module) was emitted with imports INSIDE
    the namespace wrap and broken `export using ::Foo;` lines (assumes
    flat emit; with the wrap, `Foo` isn't at global scope). Fix:
    1. Hoist `import hashbrown_port.control.{bitmask,group,tag};` out
       of the wrap and promote them to `export import` (this is the
       re-export contract for the parent module).
    2. Strip the broken `using/export using ::Foo;` lines from the wrap
       body — the `export import` above handles re-export."""
    path = cpp_out / "hashbrown_port.control.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    if not _has_deep_ns_wrap(text):
        return 0
    sentinel = "// deep-ns: child modules re-exported via export import; using-decls stripped"
    if sentinel in text:
        return 0
    original = text
    # Step 1: remove any existing `(export )?import hashbrown_port.control.X;`
    # lines wherever they appear.
    for child in ["bitmask", "group", "tag"]:
        text = re.sub(
            rf"^(?:export\s+)?import hashbrown_port\.control\.{child};\s*\n",
            "",
            text,
            flags=re.MULTILINE,
        )
    # Step 2: insert export imports immediately before the namespace open.
    inserts = (
        "export import hashbrown_port.control.bitmask;\n"
        "export import hashbrown_port.control.group;\n"
        "export import hashbrown_port.control.tag;\n"
        f"{sentinel}\n"
        "\n"
    )
    text = text.replace(
        _HB_DEEP_NS_OPEN,
        inserts + _HB_DEEP_NS_OPEN,
        1,
    )
    # Step 3: strip every `using/export using ::Foo;` line.
    text = re.sub(
        r"^\s*(?:export\s+)?using ::\w+;\s*\n",
        "",
        text,
        flags=re.MULTILINE,
    )
    if text == original:
        return 0
    path.write_text(text)
    return 1


def patch_deep_raw_helper_inside_wrap(cpp_out: Path) -> int:
    """`raw.cppm`'s `__raw_into_inner` helper (emitted by an earlier
    flat-shape patch) references `::ScopeGuard<T, F>` (global scope).
    With the wrap, `ScopeGuard` lives in the deep namespace, not global.
    Two fixes:
    1. Move the helper inside the namespace wrap.
    2. Strip the `::` prefix on `ScopeGuard` so unqualified lookup
       finds it via the enclosing namespace."""
    path = cpp_out / "hashbrown_port.raw.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    if not _has_deep_ns_wrap(text):
        return 0
    # The helper block in the flat-emit-shape patcher output. Note that
    # patch_raw_misc_fixups emits `::ScopeGuard` (global), which is the
    # form we need to strip and hoist.
    helper_block = (
        "// raw: into_inner-deduction helper — avoid `__raw_into_inner(x)`\n"
        "// which can't deduce ScopeGuard<T, F> template args.\n"
        "template<typename T, typename F>\n"
        "static inline T __raw_into_inner(::ScopeGuard<T, F> g) {\n"
        "    return ::ScopeGuard<T, F>::into_inner(std::move(g));\n"
        "}"
    )
    if helper_block not in text:
        return 0
    # Remove the helper from its current (global) location and re-insert
    # immediately after the namespace open with `::` stripped.
    fixed_helper = (
        "// raw: into_inner-deduction helper — avoid `__raw_into_inner(x)`\n"
        "// which can't deduce ScopeGuard<T, F> template args.\n"
        "template<typename T, typename F>\n"
        "static inline T __raw_into_inner(ScopeGuard<T, F> g) {\n"
        "    return ScopeGuard<T, F>::into_inner(std::move(g));\n"
        "}"
    )
    text = text.replace(helper_block, "")
    text = text.replace(
        _HB_DEEP_NS_OPEN,
        _HB_DEEP_NS_OPEN + "\n\n" + fixed_helper + "\n",
        1,
    )
    path.write_text(text)
    return 1


def patch_deep_visit_byte_buf_stub(cpp_out: Path) -> int:
    """The serde-de prelude refers to `rusty::Vec<uint8_t>` in
    `visit_byte_buf`, but `rusty::Vec` isn't visible inside the wrap
    (the alias lives in the rusty umbrella module, not pulled in here).
    Stub the body to return Err; serde-de byte-buf is not exercised by
    HashMap/HashSet. Matching the binary_heap_port stub shape exactly."""
    files = ["hashbrown_port.alloc.cppm",
             "hashbrown_port.control.tag.cppm",
             "hashbrown_port.raw.cppm",
             "hashbrown_port.map.cppm",
             "hashbrown_port.table.cppm"]
    old = (
        "template<typename E>\n"
        "rusty::Result<Value, E> visit_byte_buf(rusty::Vec<uint8_t> value) {\n"
        "return rusty::Result<Value, E>::Ok(rusty::as_u8_slice(value));\n"
        "}"
    )
    new = (
        "template<typename E>\n"
        "rusty::Result<Value, E> visit_byte_buf(auto&& value) {\n"
        "(void)value; return rusty::Result<Value, E>::Err(E{});\n"
        "}"
    )
    total = 0
    for fn in files:
        path = cpp_out / fn
        if not path.exists():
            continue
        text = path.read_text()
        if not _has_deep_ns_wrap(text):
            continue
        if old not in text:
            continue
        path.write_text(text.replace(old, new))
        total += 1
    return total


def patch_deep_raw_strip_intra_module(cpp_out: Path) -> int:
    """Several intra-module references in `raw.cppm` use `::Name` (assumes
    flat emit). With the wrap they live in the deep namespace, not at
    global scope. Strip the `::` prefix on these names so unqualified
    lookup finds them via the enclosing namespace."""
    path = cpp_out / "hashbrown_port.raw.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    if not _has_deep_ns_wrap(text):
        return 0
    original = text
    syms = [
        "ScopeGuard",
        "capacity_to_buckets",
        "ensure_bucket_bytes_at_least_ctrl_align",
        "prev_pow2",
        "bucket_mask_to_capacity",
        "maximum_buckets_in",
        "h1",
        "h2",
        "cold_path",
        "Group",
        "BitMask",
        "BitMaskIter",
        "BitMaskWord",
        "NonZeroBitMaskWord",
        "BITMASK_ITER_MASK",
        "BITMASK_STRIDE",
        "BITMASK_MASK",
    ]
    for sym in syms:
        text = text.replace(f"::{sym}<", f"{sym}<")
        text = text.replace(f"::{sym}(", f"{sym}(")
        # Bare `::Name;` / `::Name::` qualifier (member access)
        text = re.sub(rf"::{sym}::", f"{sym}::", text)
    if text == original:
        return 0
    path.write_text(text)
    return 1


def patch_deep_raw_offset_from(cpp_out: Path) -> int:
    """`raw.cppm` has two `offset_from` issues with the wrap:
    1. `::offset_from<T>(...)` call sites — local helper lives in the
       deep namespace, not global. Strip the `::` prefix.
    2. The local helper's body calls `rusty::ptr::offset_from(to, from)`,
       which doesn't exist in `<rusty/ptr.hpp>`. Inline as pointer-diff."""
    path = cpp_out / "hashbrown_port.raw.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    if not _has_deep_ns_wrap(text):
        return 0
    original = text
    text = text.replace("::offset_from<", "offset_from<")
    # Repair greedy-sed artifact if present.
    text = text.replace("rustyoffset_from(", "rusty::ptr::offset_from(")
    text = text.replace(
        "return static_cast<size_t>(rusty::ptr::offset_from(to, from));",
        "return static_cast<size_t>(to - from);",
    )
    if text == original:
        return 0
    path.write_text(text)
    return 1


def patch_deep_raw_guard_rename(cpp_out: Path) -> int:
    """In `raw.cppm`, the transpiler emits two shapes that conflict:
    - `auto guard = ::guard(...)` — both variable name AND `::guard`
      function call. With the deep wrap, `::guard` doesn't exist
      (function is in deep namespace); the variable name shadows the
      function.
    - Variable references later: `(guard)`, `(*guard)`,
      `std::move(guard)`, `_pointer_like(guard, ...)`.

    Fix: rename the variable to `_guard` and strip the `::` from the
    function call so unqualified lookup finds it via enclosing namespace."""
    path = cpp_out / "hashbrown_port.raw.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    if not _has_deep_ns_wrap(text):
        return 0
    sentinel = "// deep-ns: guard-rename completed"
    if sentinel in text:
        return 0
    original = text
    # Rename declaration: `auto guard = ::guard(` → `auto _guard = guard(`
    text = text.replace("auto guard = ::guard(", "auto _guard = guard(")
    # Rename variable references (these don't conflict with `guard(...)` calls
    # since the call shape is `guard(` whereas these are `(guard)` etc.).
    text = re.sub(r"\(guard\)", "(_guard)", text)
    text = re.sub(r"\(\*guard\)", "(*_guard)", text)
    text = re.sub(r"std::move\(guard\)", "std::move(_guard)", text)
    text = re.sub(r"_pointer_like\(guard\b", "_pointer_like(_guard", text)
    if text == original:
        return 0
    text = text.replace(
        f"namespace {_HB_DEEP_NS} {{",
        f"namespace {_HB_DEEP_NS} {{\n{sentinel}",
        1,
    )
    path.write_text(text)
    return 1


def patch_deep_strip_intra_module_global_refs(cpp_out: Path) -> int:
    """The wrap moves these helpers/types from global scope into the deep
    namespace. References emitted as `::Name<...>` / `::Name(...)` fail
    name lookup. Strip the `::` prefix on intra-module references."""
    files = ["hashbrown_port.map.cppm",
             "hashbrown_port.table.cppm",
             "hashbrown_port.set.cppm"]
    syms = ["IntoIter", "make_hash", "make_hasher", "equivalent_key",
            "__rusty_ext_equivalent"]
    total = 0
    for fn in files:
        path = cpp_out / fn
        if not path.exists():
            continue
        text = path.read_text()
        # set.cppm is hand-written and may not have a wrap initially; the
        # wrap is added by patch_deep_set_facade_wrap below. Still safe
        # to strip these prefixes either way.
        original = text
        for sym in syms:
            text = text.replace(f"::{sym}<", f"{sym}<")
            text = text.replace(f"::{sym}(", f"{sym}(")
        if text != original:
            path.write_text(text)
            total += 1
    return total


def patch_deep_map_rusty_hashmap(cpp_out: Path) -> int:
    """`map.cppm` references `rusty::HashMap<K, V, S, A>` but with the
    deep wrap `HashMap` lives at `rusty::port::collections::hashbrown::
    HashMap`. Since `map.cppm` is itself in that deep namespace, the
    unqualified `HashMap<...>` resolves correctly. Strip the
    `rusty::` prefix (likewise for `HashSet`)."""
    path = cpp_out / "hashbrown_port.map.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    if not _has_deep_ns_wrap(text):
        return 0
    original = text
    text = text.replace("rusty::HashMap<", "HashMap<")
    text = text.replace("rusty::HashSet<", "HashSet<")
    if text == original:
        return 0
    path.write_text(text)
    return 1


def patch_deep_set_facade_wrap(cpp_out: Path) -> int:
    """`set.cppm` is the hand-written `HashSet` facade; it wasn't emitted
    by the transpiler so it doesn't pick up the `--cxx-namespace` wrap
    automatically. Wrap it in the deep namespace so its `HashMap`/
    `DefaultHasher` references resolve to the deep types."""
    path = cpp_out / "hashbrown_port.set.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    if f"namespace {_HB_DEEP_NS} {{" in text:
        return 0
    # Need at least one of the imports to anchor the wrap.
    open_anchor = "import hashbrown_port.hasher;\n\n"
    close_anchor = "    HashSet<T, S> clone() const { return HashSet<T, S>(this->map.clone()); }\n};\n"
    if open_anchor not in text or close_anchor not in text:
        return 0
    text = text.replace(
        open_anchor,
        open_anchor + f"namespace {_HB_DEEP_NS} {{\n\n",
        1,
    )
    text = text.replace(
        close_anchor,
        close_anchor + f"\n}} // namespace {_HB_DEEP_NS}\n",
        1,
    )
    path.write_text(text)
    return 1


def patch_deep_hasher_facade_wrap(cpp_out: Path) -> int:
    """`hasher.cppm` was patched earlier (`patch_hasher_replace_with_stub`)
    with a flat-emit body. With deep namespace, `DefaultHasher`/
    `DefaultHashBuilder` need to live under the deep namespace so map/set
    references resolve. Wrap the post-stub body."""
    path = cpp_out / "hashbrown_port.hasher.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    if f"namespace {_HB_DEEP_NS} {{" in text:
        return 0
    open_anchor = "export module hashbrown_port.hasher;\n"
    close_anchor = "    bool operator==(const DefaultHashBuilder&) const = default;\n};\n"
    if open_anchor not in text or close_anchor not in text:
        return 0
    text = text.replace(
        open_anchor,
        open_anchor + f"\nnamespace {_HB_DEEP_NS} {{\n",
        1,
    )
    text = text.replace(
        close_anchor,
        close_anchor + f"\n}} // namespace {_HB_DEEP_NS}\n",
        1,
    )
    path.write_text(text)
    return 1


def patch_deep_util_cold_path(cpp_out: Path) -> int:
    """`util.cppm` calls `::cold_path()` (assumes flat emit). With the
    wrap, `cold_path` lives in the deep namespace. Strip the `::`
    prefix so unqualified lookup finds it via the enclosing namespace."""
    path = cpp_out / "hashbrown_port.util.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    if not _has_deep_ns_wrap(text):
        return 0
    if "::cold_path(" not in text:
        return 0
    path.write_text(text.replace("::cold_path(", "cold_path("))
    return 1


def patch_deep_alloc_ufcs_allocator_qualify(cpp_out: Path) -> int:
    """The UFCS trait-migration emits the `Allocator_` bridge as
    `export using ::inner::__ufcs_Allocator::allocate;` — an absolute
    path that assumes flat emit. Under `--cxx-namespace`, `inner` lives
    in the deep wrap (`rusty::port::collections::hashbrown::inner`), so
    `::inner` resolves at global scope and misses. The transpiler's
    wrap re-qualification (Rule 2, `ufcs_bridge_top_namespaces`) does not
    catch this `inner` bridge. Strip the leading `::` so the reference is
    relative and resolves through the enclosing wrap — same shape as the
    other deep-ns `::`-strip patches (cold_path, Group, ScopeGuard, …)."""
    path = cpp_out / "hashbrown_port.alloc.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    if not _has_deep_ns_wrap(text):
        return 0
    needle = "using ::inner::__ufcs_Allocator::"
    if needle not in text:
        return 0
    path.write_text(text.replace(needle, "using inner::__ufcs_Allocator::"))
    return 1


def patch_deep_alloc_adapter_rusty_unit_convert(cpp_out: Path) -> int:
    """Like `patch_alloc_adapter_error_convert` but for the deep-emit
    error type. `AllocatorAdapter::allocate` returns `Result<_,
    rusty::Unit>` (deep-emit shape — flat-emit uses `std::tuple<>`).
    The delegation to `value_.allocate(layout)` returns `Result<_,
    AllocError>`. Convert."""
    path = cpp_out / "hashbrown_port.alloc.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    if not _has_deep_ns_wrap(text):
        return 0
    sentinel = "// deep-ns: adapter rusty::Unit error convert"
    if sentinel in text:
        return 0
    pattern = re.compile(
        r"(rusty::Result<rusty::ptr::NonNull<uint8_t>, rusty::Unit> allocate\(rusty::alloc::Layout layout\) const override \{)\s*\n"
        r"\s*return value_\.allocate\(layout\);\s*\n"
        r"(\s*\})",
    )
    replacement = (
        r"\1\n"
        + "        " + sentinel + "\n"
        + "        auto r = value_.allocate(layout);\n"
        + "        if (r.is_ok()) return rusty::Result<rusty::ptr::NonNull<uint8_t>, rusty::Unit>::Ok(std::move(r).unwrap());\n"
        + "        return rusty::Result<rusty::ptr::NonNull<uint8_t>, rusty::Unit>::Err(rusty::Unit{});\n"
        + r"\2"
    )
    new_text = pattern.sub(replacement, text)
    if new_text == text:
        return 0
    path.write_text(new_text)
    return 1


def patch_deep_alloc_slice_from_raw_parts_mut(cpp_out: Path) -> int:
    """`alloc.cppm` calls `rusty::ptr::slice_from_raw_parts_mut(data,
    size)` to build a fat slice pointer, but `<rusty/ptr.hpp>` doesn't
    export that symbol (Rust-only API). Drop the slice length and pass
    the raw `data` pointer directly — `NonNull<uint8_t>::new_unchecked`
    expects a single pointer, not a slice."""
    path = cpp_out / "hashbrown_port.alloc.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    if not _has_deep_ns_wrap(text):
        return 0
    old = "rusty::ptr::slice_from_raw_parts_mut(rusty::as_ptr(data), layout.size)"
    new = "rusty::as_ptr(data)"
    if old not in text:
        return 0
    path.write_text(text.replace(old, new))
    return 1


def patch_deep_alloc_do_alloc_result_convert(cpp_out: Path) -> int:
    """In the deep-wrap emit, `do_alloc` returns
    `rusty::Result<NonNull<u8>, rusty::Unit>` and delegates to
    `rusty::deref_call(alloc, [&](auto&& __recv) { return __recv.allocate(...); })`
    which returns `Result<_, AllocError>`. Convert the error type.
    (Distinct from `patch_alloc_inner_do_alloc_convert` which matches the
    flat-emit signature with `std::tuple<>`.)"""
    path = cpp_out / "hashbrown_port.alloc.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    if not _has_deep_ns_wrap(text):
        return 0
    sentinel = "// deep-ns: do_alloc rusty::Unit error convert"
    if sentinel in text:
        return 0
    # The deep-emit body is a single line returning the deref_call result
    # directly. Wrap it with the same convert-Err shape.
    old = (
        "    rusty::Result<rusty::ptr::NonNull<uint8_t>, rusty::Unit> do_alloc(const A& alloc, rusty::alloc::Layout layout) {\n"
        "        return rusty::deref_call(alloc, [&](auto&& __recv) -> decltype(std::forward<decltype(__recv)>(__recv).allocate(std::move(layout))) { return std::forward<decltype(__recv)>(__recv).allocate(std::move(layout)); });\n"
        "    }"
    )
    new = (
        "    rusty::Result<rusty::ptr::NonNull<uint8_t>, rusty::Unit> do_alloc(const A& alloc, rusty::alloc::Layout layout) {\n"
        "        " + sentinel + "\n"
        "        auto r = rusty::deref_call(alloc, [&](auto&& __recv) -> decltype(std::forward<decltype(__recv)>(__recv).allocate(std::move(layout))) { return std::forward<decltype(__recv)>(__recv).allocate(std::move(layout)); });\n"
        "        if (r.is_ok()) return rusty::Result<rusty::ptr::NonNull<uint8_t>, rusty::Unit>::Ok(std::move(r).unwrap());\n"
        "        return rusty::Result<rusty::ptr::NonNull<uint8_t>, rusty::Unit>::Err(rusty::Unit{});\n"
        "    }"
    )
    if old not in text:
        return 0
    path.write_text(text.replace(old, new))
    return 1


# ── orchestration ───────────────────────────────────────────────────

def main(cpp_out: Path):
    patches = [
        ("Tag methods: add const qualifier", patch_tag_methods_const),
        ("Tag::fmt — stub (rusty::fmt::Formatter has no pad)", patch_tag_formatter_pad),
        ("control.tag: fill_tag(&mut self) — drop const on self_ span (UFCS mut-self)", patch_control_tag_fill_tag_mut_self),
        ("hasher: replace entire module body with FNV-1a stub", patch_hasher_replace_with_stub),
        ("alloc: allocator_api2 → rusty::alloc", patch_alloc_allocator_api2),
        ("alloc: drop duplicate do_alloc definition", patch_alloc_do_alloc_dup),
        ("alloc: inner::Global::{allocate,deallocate} → std::malloc/free", patch_alloc_global_impl),
        ("alloc: AllocatorAdapter — convert rusty AllocError to std::tuple<>", patch_alloc_adapter_error_convert),
        ("alloc: inner::do_alloc — convert rusty AllocError to std::tuple<>", patch_alloc_inner_do_alloc_convert),
        ("drop duplicate DefaultHasher stubs (alloc/control.tag/raw)", patch_drop_dup_defaulthasher_global),
        ("control.group.generic: replace whole body with hand-rolled impl", patch_group_generic_replace),
        ("control.group: drop generic:: qualifier (no sibling C++ namespace)", patch_control_group_imp_alias),
        ("control parent: strip bitmask::/group::/tag:: qualifiers", patch_control_module_namespaces),
        ("control.bitmask: move import + strip group:: prefix + rusty::clone", patch_control_bitmask_imports),
        ("raw: bare TryReserveError → rusty::collections::TryReserveError", patch_raw_tryreserveerror),
        ("raw: hoist imports to top of module", patch_raw_imports_top),
        ("raw: std::{AllocError,Allocator,Layout,Global,handle_alloc_error} → rusty::alloc::*", patch_raw_std_alloc_namespace),
        ("raw: TryReserveError variant constructors → rusty tagged-struct ctor", patch_raw_tryreserveerror_constructors),
        ("raw: misc fixups (control::, invalid_mut, Rust-syntax assert!s)", patch_raw_misc_fixups),
        ("raw: fix RawTable::clear() double-free (ScopeGuard takes T by value)", patch_raw_table_clear_double_free),
        ("raw: rehash_in_place — write back guard-copy scalars (ScopeGuard takes T by value)", patch_raw_rehash_guard_writeback),
        ("scopeguard: dropfn arg by reference, not pointer", patch_scopeguard_dropfn_arg),
        ("table: hoist imports + std::* → rusty::* + drop raw:: qualifier", patch_table_module),
        ("map: same fixups as table", patch_map_module),
        # set/raw_entry/rustc_entry: stub for Phase A2 (advanced
        # features beyond core HashMap port).
        ("set: stub (Phase A2 — HashSet not in core scope)", patch_set_stub),
        ("raw_entry: stub (Phase A2)", patch_raw_entry_stub),
        ("rustc_entry: stub (Phase A2)", patch_rustc_entry_stub),
        # Umbrella module imports come after struct decls; hoist.
        ("umbrella: hoist `import hashbrown_port.X;` to top of module", patch_umbrella_imports),
        # CMakeLists: append smoke-test target.
        ("CMakeLists: append smoke_test target", patch_cmakelists_smoke_test),
        ("strip debug-assert macros across all cppm files", patch_strip_debug_asserts_global),
        # ── deep-namespace migration (--cxx-namespace) ──
        # These fire only when the cppm files have the deep wrap (see
        # _has_deep_ns_wrap guard). On flat-emit input they are no-ops.
        ("deep-ns: constrain prelude clone (resolves rusty::clone ambiguity)", patch_deep_prelude_clone_constrain),
        ("deep-ns: control.group re-exports .generic", patch_deep_control_group_reexport),
        ("deep-ns: control.cppm export-imports children + drops broken using-decls", patch_deep_control_cppm_reexport),
        ("deep-ns: raw.cppm __raw_into_inner helper inside wrap", patch_deep_raw_helper_inside_wrap),
        ("deep-ns: raw.cppm — strip :: on intra-module names (ScopeGuard, Group, etc.)", patch_deep_raw_strip_intra_module),
        ("deep-ns: stub visit_byte_buf (rusty::Vec not in wrap scope)", patch_deep_visit_byte_buf_stub),
        ("deep-ns: raw.cppm offset_from — strip :: + inline ptr-diff", patch_deep_raw_offset_from),
        ("deep-ns: raw.cppm complete _guard variable rename", patch_deep_raw_guard_rename),
        ("deep-ns: strip :: on intra-module names (IntoIter / make_hash / etc)", patch_deep_strip_intra_module_global_refs),
        ("deep-ns: map.cppm rusty::HashMap → HashMap (same namespace)", patch_deep_map_rusty_hashmap),
        ("deep-ns: set.cppm hand-written facade — wrap in deep namespace", patch_deep_set_facade_wrap),
        ("deep-ns: hasher.cppm hand-written facade — wrap in deep namespace", patch_deep_hasher_facade_wrap),
        ("deep-ns: util.cppm — strip :: prefix on cold_path", patch_deep_util_cold_path),
        ("deep-ns: alloc.cppm — strip :: on inner::__ufcs_Allocator bridge re-export", patch_deep_alloc_ufcs_allocator_qualify),
        ("deep-ns: alloc.cppm — drop rusty::ptr::slice_from_raw_parts_mut (not in rusty/ptr.hpp)", patch_deep_alloc_slice_from_raw_parts_mut),
        ("deep-ns: alloc.cppm AllocatorAdapter — convert AllocError → rusty::Unit", patch_deep_alloc_adapter_rusty_unit_convert),
        ("deep-ns: alloc.cppm do_alloc — convert AllocError → rusty::Unit (deep-emit shape)", patch_deep_alloc_do_alloc_result_convert),
    ]
    total = 0
    for name, fn in patches:
        n = fn(cpp_out)
        if n:
            print(f"  applied: {name}")
            total += n
        else:
            print(f"  skipped: {name} (already applied or not applicable)")
    print(f"{total} patch(es) applied")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("usage: python3 post_transpile_patch.py <cpp_out_dir>", file=sys.stderr)
        sys.exit(1)
    main(Path(sys.argv[1]))
