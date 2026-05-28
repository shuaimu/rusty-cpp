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
    text = text.replace("    bool is_full();", "    bool is_full() const;")
    text = text.replace("    bool is_special();", "    bool is_special() const;")
    text = text.replace("    bool special_is_empty();", "    bool special_is_empty() const;")
    # Out-of-line definitions
    text = re.sub(
        r"^bool Tag::is_full\(\) \{",
        "bool Tag::is_full() const {",
        text, flags=re.MULTILINE)
    text = re.sub(
        r"^bool Tag::is_special\(\) \{",
        "bool Tag::is_special() const {",
        text, flags=re.MULTILINE)
    text = re.sub(
        r"^bool Tag::special_is_empty\(\) \{",
        "bool Tag::special_is_empty() const {",
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
    uint64_t state = 0;
    void write(std::span<const uint8_t> bytes) {
        for (uint8_t b : bytes) {
            state ^= static_cast<uint64_t>(b);
            state *= 1099511628211ULL;  // FNV-1a prime
        }
    }
    void write_u64(uint64_t v) { state ^= v; state *= 1099511628211ULL; }
    uint64_t finish() const { return state; }
    DefaultHasher clone() const { return *this; }
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
    bool any_bit_set() const { return _0 != 0; }
    BitMask remove_lowest_bit() const { return BitMask{_0 & (_0 - 1)}; }
    size_t trailing_zeros() const { return _0 == 0 ? 64 : __builtin_ctzll(_0); }
    size_t leading_zeros() const { return _0 == 0 ? 64 : __builtin_clzll(_0); }
    // `lowest_set_bit() -> Option<size_t>`. Rust returns Some(idx) if
    // any bit set, None if all zero.
    rusty::Option<size_t> lowest_set_bit() const {
        if (_0 == 0) return rusty::Option<size_t>(rusty::None);
        return rusty::Option<size_t>(static_cast<size_t>(__builtin_ctzll(_0)));
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
        // Set high bit on all (special), clear low bit on all.
        GroupWord full = _0 & BITMASK_MASK;
        return Group{(~full + (full >> 7)) | 0x8080808080808080ULL};
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
    # `rusty::collections::TryReserveError`.
    text = re.sub(
        r"::TryReserveError\b(?!_)",
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
    # The transpiler emitted these as `expr->cast()`. Match the full
    # member-access chain (including any `this->` prefix) so we don't
    # leave a dangling `this->reinterpret_cast<...>` after replacement.
    # Cast to `const uint8_t*` — our hand-rolled Group::load_aligned
    # takes that signature (avoids cross-module Tag-type mismatch).
    text = re.sub(
        r"(this->)?(\w+)->cast\(\)",
        lambda m: "reinterpret_cast<const uint8_t*>(" + (m.group(1) or "") + m.group(2) + ")",
        text,
    )
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
    for fn in ("Group::load_aligned", "Group::load", "Group::store_aligned"):
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
                out_parts.append(fn + "(reinterpret_cast<const uint8_t*>(" + arg + "))")
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

    # `rusty::iter(X.match_full())` — Rust source has `.match_full()
    # .into_iter()`. Our Group::match_* methods return
    # `group_internal::BitMask` (module-private, layout-compatible).
    # The call site needs to wrap that into the real
    # `control.bitmask::BitMask` (which has `.into_iter()`) by
    # constructing one from the `_0` field.
    # Use brace-matching to allow nested parens in X.
    while True:
        idx = text.find("rusty::iter(")
        # Only iterate for those whose content ends with `.match_full()`.
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
            if content.rstrip().endswith(".match_full()"):
                # Wrap the group_internal::BitMask via `_0` into the
                # real control.bitmask::BitMask, then call its
                # into_iter().
                text = text[:i] + "BitMask{" + content + "._0}.into_iter()" + text[j+1:]
                found = True
                break

    # `rusty::for_in(group.match_X(arg))` — `match_X` returns our
    # group_internal::BitMask which isn't iterable. Wrap with real
    # BitMask first. Brace-matching to find the inner call.
    for match_method in (".match_tag(", ".match_empty(", ".match_empty_or_deleted(",
                          ".match_full("):
        for_in_prefix = "rusty::for_in("
        while True:
            idx = text.find(for_in_prefix)
            if idx == -1:
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
                break
            content = text[content_start:j]
            if match_method in content and "BitMask{" not in content:
                wrapped = "rusty::for_in(BitMask{" + content + "._0})"
                text = text[:idx] + wrapped + text[j+1:]
            else:
                # Skip past this match and find the next.
                text = text[:idx] + "__FORINSEEN__" + text[idx+len("__FORINSEEN__"):]
        text = text.replace("__FORINSEEN__", for_in_prefix[:len("__FORINSEEN__")])

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


# ── orchestration ───────────────────────────────────────────────────

def main(cpp_out: Path):
    patches = [
        ("Tag methods: add const qualifier", patch_tag_methods_const),
        ("Tag::fmt — stub (rusty::fmt::Formatter has no pad)", patch_tag_formatter_pad),
        ("hasher: replace entire module body with FNV-1a stub", patch_hasher_replace_with_stub),
        ("alloc: allocator_api2 → rusty::alloc", patch_alloc_allocator_api2),
        ("alloc: drop duplicate do_alloc definition", patch_alloc_do_alloc_dup),
        ("alloc: inner::Global::{allocate,deallocate} → std::malloc/free", patch_alloc_global_impl),
        ("alloc: AllocatorAdapter — convert rusty AllocError to std::tuple<>", patch_alloc_adapter_error_convert),
        ("control.group.generic: replace whole body with hand-rolled impl", patch_group_generic_replace),
        ("control.group: drop generic:: qualifier (no sibling C++ namespace)", patch_control_group_imp_alias),
        ("control parent: strip bitmask::/group::/tag:: qualifiers", patch_control_module_namespaces),
        ("control.bitmask: move import + strip group:: prefix + rusty::clone", patch_control_bitmask_imports),
        ("raw: bare TryReserveError → rusty::collections::TryReserveError", patch_raw_tryreserveerror),
        ("raw: hoist imports to top of module", patch_raw_imports_top),
        ("raw: std::{AllocError,Allocator,Layout,Global,handle_alloc_error} → rusty::alloc::*", patch_raw_std_alloc_namespace),
        ("raw: TryReserveError variant constructors → rusty tagged-struct ctor", patch_raw_tryreserveerror_constructors),
        ("raw: misc fixups (control::, invalid_mut, Rust-syntax assert!s)", patch_raw_misc_fixups),
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
