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


# ── orchestration ───────────────────────────────────────────────────

def main(cpp_out: Path):
    patches = [
        ("Tag methods: add const qualifier", patch_tag_methods_const),
        ("Tag::fmt — stub (rusty::fmt::Formatter has no pad)", patch_tag_formatter_pad),
        ("hasher: replace entire module body with FNV-1a stub", patch_hasher_replace_with_stub),
        ("alloc: allocator_api2 → rusty::alloc", patch_alloc_allocator_api2),
        ("alloc: drop duplicate do_alloc definition", patch_alloc_do_alloc_dup),
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
