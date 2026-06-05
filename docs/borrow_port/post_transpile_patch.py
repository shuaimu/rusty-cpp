#!/usr/bin/env python3
"""Post-transpile patches for the borrow_port C++20 module port.

Idempotent. Bridges the residual 3 categories of compile errors after
the Phase 3a/3b transpiler trait machinery landed (commits 9fe6506 →
fdefbff). Patterns are copied from sibling ports:

  P1  — rebody visit_byte_buf (Vec<uint8_t> not visible in GMF, same
        as binary_heap_port P2)

  P2  — rewrite the leading `::rusty_ext::` calls inside Cow::clone_from
        to unqualified `rusty_ext::` so they resolve via the in-namespace
        block at the bottom of the module (the leading `::` looks past
        the wrapping `borrow_port` auto-namespace and never finds the
        global `::rusty_ext`).

  P3  — wrap the orphan-impl `T to_owned()` / `void clone_into(...)`
        block (where the host type lives in another TU) in `#if 0 …
        #endif`, same pattern as rc_port's patch_stub_orphan_impls.

Usage: post_transpile_patch.py <cpp_out_dir>
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

BORROW_FILE = "borrow_port.cppm"


# ---------------------------------------------------------------------------
# P0: give the ToOwnedTraits<B> primary template a body matching the
# blanket impl semantics (`impl<T: Clone> ToOwned for T { type Owned = T; }`).
# Without a body, `Cow_Owned<std::string_view>` fails to instantiate at
# module-build time because clang evaluates the field type
# `typename ToOwnedTraits<B>::Owned _0;` eagerly. Downstream specializations
# (emitted by future `impl ToOwned for str` etc.) win over this default.
# ---------------------------------------------------------------------------


def patch_to_owned_traits_primary_body(text: str) -> str:
    needle = "template <class B> struct ToOwnedTraits;"
    if needle not in text:
        return text
    return text.replace(
        needle,
        "template <class B> struct ToOwnedTraits { using Owned = B; };",
    )


# ---------------------------------------------------------------------------
# P1: rebody visit_byte_buf — its arg uses rusty::Vec<uint8_t> which
# isn't visible in the GMF (module imports kick in after `export module`).
# Identical pattern to binary_heap_port's P2.
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
# P2: drop the leading `::` from `::rusty_ext::clone_into(...)` so it
# resolves via the in-namespace `rusty_ext` block. Anchored to
# `::rusty_ext::clone_into` to avoid touching the visitor-template
# references like `::de::rusty_ext::deserialize` (those live in
# never-instantiated visitor templates and don't surface as errors).
# ---------------------------------------------------------------------------


def patch_rusty_ext_clone_into(text: str) -> str:
    return text.replace("::rusty_ext::clone_into", "rusty_ext::clone_into")


# ---------------------------------------------------------------------------
# P3: orphan-impl methods of T at namespace scope. The transpiler
# emits `T to_owned()` / `void clone_into(T&)` as free functions
# referencing `(*this)` and `T` — invalid C++ outside a member context.
# Wrap the block in `#if 0 … #endif`. Pattern copied from rc_port.
# ---------------------------------------------------------------------------


def patch_stub_orphan_impls(text: str) -> str:
    if "#if 0  // patcher: orphan-impl block stubbed" in text:
        return text
    lines = text.splitlines(keepends=True)
    n = len(lines)
    out: list[str] = []
    i = 0
    changed = False
    while i < n:
        if lines[i].startswith("// TODO orphan impl:"):
            j = i + 1
            while j < n:
                line = lines[j]
                if (
                    line.startswith("// TODO orphan impl:")
                    or line.startswith("export ")
                    or line.startswith("} // namespace")
                    or line.startswith("// Extension trait")
                ):
                    break
                j += 1
            out.append("#if 0  // patcher: orphan-impl block stubbed\n")
            out.extend(lines[i:j])
            out.append("#endif  // patcher: end orphan-impl stub\n")
            i = j
            changed = True
            continue
        out.append(lines[i])
        i += 1
    if changed:
        return "".join(out)
    return text


# ---------------------------------------------------------------------------
# Apply all patches in sequence. Idempotent.
# ---------------------------------------------------------------------------


def patch_file(path: Path) -> bool:
    text = path.read_text()
    original = text
    text = patch_to_owned_traits_primary_body(text)
    text = patch_visit_byte_buf(text)
    text = patch_rusty_ext_clone_into(text)
    text = patch_stub_orphan_impls(text)
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
    path = cpp_out / BORROW_FILE
    if not path.exists():
        print(f"error: {path} does not exist")
        return 1
    changed = patch_file(path)
    if changed:
        print(f"borrow_port patches applied to {path.name}")
    else:
        print(f"borrow_port: no patches needed (already clean or idempotent)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
