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

    static Group load(const Tag* ptr) {
        GroupWord w;
        std::memcpy(&w, ptr, sizeof(w));
        return Group{w};
    }

    static Group load_aligned(const Tag* ptr) {
        return load(ptr);
    }

    void store_aligned(Tag* ptr) const {
        std::memcpy(ptr, &_0, sizeof(_0));
    }

    BitMask match_tag(Tag tag) const {
        GroupWord cmp = _0 ^ repeat_tag(tag);
        // x - 0x01... will overflow into the high bit if the byte was 0.
        GroupWord r = (cmp - 0x0101010101010101ULL) & ~cmp & 0x8080808080808080ULL;
        return BitMask{static_cast<BitMaskWord>(r)};
    }

    BitMask match_empty() const {
        return match_tag(Tag::EMPTY);
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
    # Match bare TryReserveError (not already qualified).
    text = re.sub(
        r"(?<![\w:])TryReserveError\b",
        "rusty::collections::TryReserveError",
        text,
    )
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
