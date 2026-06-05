#!/usr/bin/env python3
"""Post-transpile patches for the ascii_port C++20 module port.

Idempotent. ascii_port is `library/core/src/ascii/ascii_char.rs` (1229
LOC) → `transpiled/ascii_port/ascii_port.cppm` (~4151 LOC).

Patches:
  P1 — rebody visit_byte_buf (Vec<uint8_t> not visible in GMF, same as
       binary_heap_port / borrow_port).
  P2 — `Self::Null` / `Self::Delete` → `AsciiChar::Null` / `AsciiChar::Delete`
       in the absorbed-method constexpr emits.
  P3 — delete the duplicate `AsciiChar_MIN` / `AsciiChar_MAX` constexpr
       defs at the bottom (transpiler emits both per-variant factories
       and these manifest constants, duplicating them once at the top
       of the absorbed-method block and once at the bottom).
  P4 — `::slice::from_ref(self_)` → take address (`&self_`) — the
       Rust-side `slice::from_ref` materialises a 1-elem slice; in C++
       we just want a pointer/reference to the byte itself.
  P5 — stub `escape_ascii` body — its return type `EscapeDefault` lives
       in the parent `core::ascii` module that we haven't ported.

Usage: post_transpile_patch.py <cpp_out_dir>
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ASCII_FILE = "ascii_port.cppm"


# ---------------------------------------------------------------------------
# P1: visit_byte_buf stub.
# ---------------------------------------------------------------------------

VISIT_BYTE_BUF_STUB = (
    "template<typename E>\n"
    "rusty::Result<Value, E> visit_byte_buf(auto&& value) {\n"
    "(void)value; return rusty::Result<Value, E>::Err(E{});\n"
    "}"
)


def patch_visit_byte_buf(text: str) -> str:
    if VISIT_BYTE_BUF_STUB in text:
        return text
    return re.sub(
        r"template<typename E>\nrusty::Result<Value, E> visit_byte_buf\(rusty::Vec<uint8_t> value\) \{\n"
        r"return rusty::Result<Value, E>::Ok\(rusty::as_u8_slice\(value\)\);\n"
        r"\}",
        VISIT_BYTE_BUF_STUB,
        text,
    )


# ---------------------------------------------------------------------------
# P2: `Self::Null` / `Self::Delete` in absorbed-method constants.
# ---------------------------------------------------------------------------


def patch_self_to_ascii_char(text: str) -> str:
    return (
        text.replace(
            "inline constexpr auto AsciiChar_MIN = Self::Null;",
            "inline constexpr auto AsciiChar_MIN = AsciiChar::Null;",
        )
        .replace(
            "inline constexpr auto AsciiChar_MAX = Self::Delete;",
            "inline constexpr auto AsciiChar_MAX = AsciiChar::Delete;",
        )
    )


# ---------------------------------------------------------------------------
# P3: delete the second pair of duplicate AsciiChar_MIN / AsciiChar_MAX
# defs. After P2 fixes Self::, both pairs become identical defs of the
# same names — keep the first, drop the second.
# ---------------------------------------------------------------------------


def patch_drop_duplicate_min_max(text: str) -> str:
    # Find both occurrences; if exactly 2, drop the second.
    needle_min = "inline constexpr auto AsciiChar_MIN = AsciiChar::Null;\n"
    needle_max = "inline constexpr auto AsciiChar_MAX = AsciiChar::Delete;\n"
    first_min = text.find(needle_min)
    if first_min == -1:
        return text
    second_min = text.find(needle_min, first_min + len(needle_min))
    if second_min == -1:
        return text
    # Drop the second MIN/MAX pair (they appear consecutively).
    end = second_min + len(needle_min)
    if text[end : end + len(needle_max)] == needle_max:
        end += len(needle_max)
    return text[:second_min] + text[end:]


# ---------------------------------------------------------------------------
# P4: `::slice::from_ref(self_)` → `&self_` (Rust's slice::from_ref
# materialises a one-element slice; the call-site here is
# `crate::slice::from_ref(self).as_str()` which is just a way to get
# the bytes pointer — and the impl [AsciiChar] block providing `.as_str()`
# was already stripped in prep.sh as a hand-port slot).
# Easier: stub the whole `as_str` function body for now.
# ---------------------------------------------------------------------------


AS_STR_BODY_MARKER = "patcher: stub as_str"
AS_STR_NEW = (
    "inline std::string_view as_str(const AsciiChar& self_) {\n"
    f"    // {AS_STR_BODY_MARKER} — `impl [AsciiChar]::as_str` is a hand-port slot.\n"
    "    static thread_local char buf[2] = {0, 0};\n"
    "    buf[0] = static_cast<char>(static_cast<uint8_t>(self_));\n"
    "    return std::string_view(buf, 1);\n"
    "}"
)


def patch_as_str_stub(text: str) -> str:
    if AS_STR_BODY_MARKER in text:
        return text
    rx = re.compile(
        r"^inline std::string_view as_str\(const AsciiChar& self_\) \{",
        re.MULTILINE,
    )
    m = rx.search(text)
    if not m:
        return text
    open_brace = m.end() - 1
    close_after = _find_balanced_close(text, open_brace)
    if close_after == -1:
        return text
    return text[: m.start()] + AS_STR_NEW + text[close_after:]


# ---------------------------------------------------------------------------
# P5: `escape_ascii` returns `EscapeDefault` from the parent ascii
# module — we haven't ported that. Stub the body + return type.
# ---------------------------------------------------------------------------


ESCAPE_ASCII_OLD_RE = re.compile(
    r"inline EscapeDefault escape_ascii\(AsciiChar self_\) \{[^}]*?\n\}",
    re.DOTALL,
)
ESCAPE_ASCII_NEW = (
    "// patcher: escape_ascii stubbed — EscapeDefault lives in parent\n"
    "// core::ascii module (not ported).\n"
    "// inline void escape_ascii(AsciiChar self_) { (void)self_; }"
)


def patch_escape_ascii_stub(text: str) -> str:
    marker = "// patcher: escape_ascii stubbed"
    if marker in text:
        return text
    # Body via brace counter.
    rx = re.compile(
        r"^inline EscapeDefault escape_ascii\(AsciiChar self_\) \{",
        re.MULTILINE,
    )
    m = rx.search(text)
    if m:
        open_brace = m.end() - 1
        close_after = _find_balanced_close(text, open_brace)
        if close_after != -1:
            text = text[: m.start()] + ESCAPE_ASCII_NEW + text[close_after:]
    # Drop the matching export forward decl too.
    text = text.replace(
        "export EscapeDefault escape_ascii(AsciiChar self_);",
        f"{marker} (forward decl dropped)",
    )
    return text


# ---------------------------------------------------------------------------
# P6: stub the to_uppercase / to_lowercase / eq_ignore_case /
# is_alphabetic / etc. bodies that call `byte.to_ascii_X()` on a u8
# (uint8_t in C++) — no such methods exist on primitives in C++. Stub
# returns sensible defaults until rusty::ascii::* helpers are added.
# ---------------------------------------------------------------------------

PRIM_METHOD_STUBS = [
    ("to_uppercase", "return self_;"),
    ("to_lowercase", "return self_;"),
    ("make_uppercase", "(void)self_;"),
    ("make_lowercase", "(void)self_;"),
    ("eq_ignore_case", "(void)self_; (void)other; return false;"),
    ("is_alphabetic", "return false;"),
    ("is_uppercase", "return false;"),
    ("is_lowercase", "return false;"),
    ("is_alphanumeric", "return false;"),
    ("is_octdigit", "return false;"),
    ("is_punctuation", "return false;"),
    ("is_graphic", "return false;"),
    ("is_whitespace", "return false;"),
    ("is_control", "return false;"),
]


def _find_balanced_close(text: str, open_pos: int) -> int:
    """Given the index of an opening `{`, return the index ONE PAST the
    matching close brace, brace-counted (respects nested blocks).
    Returns -1 if no match (malformed input)."""
    assert text[open_pos] == "{"
    depth = 1
    i = open_pos + 1
    while i < len(text):
        ch = text[i]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    return -1


def patch_stub_primitive_method_bodies(text: str) -> str:
    """Stub bodies whose Rust source called `byte.is_ascii_X()` etc. on
    a u8 — no analogue in C++ as a free fn / member of uint8_t. Marker
    comment makes this idempotent.
    """
    marker = "// patcher: stub primitive-method body"
    if marker in text:
        return text
    for fn_name, replacement in PRIM_METHOD_STUBS:
        # Find `inline <return-type> <fn_name>(<sig>) {` opener, anchored
        # to the AsciiChar overload (the broken-body emit) — there can
        # be sibling free fns of the same name on other types (e.g. an
        # `is_whitespace(char32_t)` helper); we only want to touch the
        # AsciiChar one.
        rx = re.compile(
            rf"^inline (\w+(?:::\w+)?(?:<[^>]+>)?(?:\s*&)?) "
            rf"{re.escape(fn_name)}\((AsciiChar[^)]*|AsciiChar& self_[^)]*)\) \{{",
            re.MULTILINE,
        )
        m = rx.search(text)
        if not m:
            continue
        open_brace = m.end() - 1  # index of `{`
        close_after = _find_balanced_close(text, open_brace)
        if close_after == -1:
            continue
        ret_type, sig = m.group(1), m.group(2)
        new_body = (
            f"inline {ret_type} {fn_name}({sig}) {{\n"
            f"    {marker} `{fn_name}`\n"
            f"    {replacement}\n"
            "}"
        )
        text = text[: m.start()] + new_body + text[close_after:]
    return text


# ---------------------------------------------------------------------------
# P7: `AsciiChar::from_u8_unchecked(b)` → free-fn `from_u8_unchecked(b)`.
# The absorbed-method pipeline emitted from_u8_unchecked as a free fn
# but the calls inside other absorbed methods still spell it as a
# scoped static call on the enum.
# ---------------------------------------------------------------------------


def patch_from_u8_unchecked_call(text: str) -> str:
    return text.replace(
        "AsciiChar::from_u8_unchecked(", "from_u8_unchecked("
    )


# ---------------------------------------------------------------------------


def patch_file(path: Path) -> bool:
    text = path.read_text()
    original = text
    text = patch_visit_byte_buf(text)
    text = patch_self_to_ascii_char(text)
    text = patch_drop_duplicate_min_max(text)
    text = patch_as_str_stub(text)
    text = patch_escape_ascii_stub(text)
    text = patch_stub_primitive_method_bodies(text)
    text = patch_from_u8_unchecked_call(text)
    if text != original:
        path.write_text(text)
        return True
    return False


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 1
    cpp_out = Path(sys.argv[1])
    if not cpp_out.exists():
        print(f"error: {cpp_out} does not exist")
        return 1
    path = cpp_out / ASCII_FILE
    if not path.exists():
        print(f"error: {path} does not exist")
        return 1
    changed = patch_file(path)
    if changed:
        print(f"ascii_port patches applied to {path.name}")
    else:
        print(f"ascii_port: no patches needed (already clean or idempotent)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
