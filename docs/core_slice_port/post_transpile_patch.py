#!/usr/bin/env python3
"""Post-transpile patches for the core_slice_port C++20 module port.

Idempotent. core_slice_port is `library/core/src/slice/*` (15421 LOC
Rust collapsed via prep.sh + collapse.py) → `core_slice_port.cppm`
(8773 LOC C++).

Patches:
  P1 — `std::ops::Bound` → `rusty::Bound` (Bound is at rusty::, not
        rusty::ops::; matches rusty/array.hpp definition).
  P2 — `std::ops::ControlFlow` → `rusty::ops::ControlFlow`.
  P3 — `std::convert::` → `rusty::convert::`.
  P4 — `std::ops::` → `rusty::ops::` (catch-all for remaining
        std::ops:: references; runs AFTER specific Bound/ControlFlow
        rewrites so they take precedence).

Usage: post_transpile_patch.py <cpp_out_dir>
"""

from __future__ import annotations

import sys
from pathlib import Path

SLICE_FILE = "core_slice_port.cppm"


def patch_bound(text: str) -> str:
    # Bound is defined as a free template alias at `rusty::Bound` in
    # array.hpp; it is NOT under rusty::ops::.
    return text.replace("std::ops::Bound", "rusty::Bound")


def patch_control_flow(text: str) -> str:
    return text.replace("std::ops::ControlFlow", "rusty::ops::ControlFlow")


def patch_convert(text: str) -> str:
    return text.replace("std::convert::", "rusty::convert::")


def patch_remaining_std_ops(text: str) -> str:
    # Catch-all after the specific Bound/ControlFlow rewrites above.
    return text.replace("std::ops::", "rusty::ops::")


def patch_std_range(text: str) -> str:
    # The Rust `range::Range`, `range::RangeInclusive` paths leak through
    # as `std::range::Range` etc. They should be rusty::ops::*.
    return text.replace("std::range::", "rusty::ops::")


def patch_std_ptr(text: str) -> str:
    return text.replace("std::ptr::", "rusty::ptr::")


def patch_std_ascii(text: str) -> str:
    return text.replace("std::ascii::", "rusty::ascii::")


def patch_size_of(text: str) -> str:
    # `size_of::<T>()` and `size_of<T>()` are emitted from `mem::size_of`;
    # neither exists in C++; map to `sizeof(T)`.
    import re
    text = re.sub(r"\bsize_of<([^>]+)>\(\)", r"sizeof(\1)", text)
    return text


def patch_strip_orphan_imports(text: str) -> str:
    # `import core_slice_port.index;` etc. — auto-namespace artifacts
    # for submodules that don't exist post-collapse. They appear inside
    # a `module;`/`export module` body where `import` isn't a keyword,
    # so the parser errors. Strip them.
    import re
    return re.sub(r"^import\s+core_slice_port\.\w+;\s*\n",
                  "", text, flags=re.MULTILINE)


def patch_strip_using_simd(text: str) -> str:
    # `using std::simd;` etc. — Rust's portable_simd has no analogue.
    import re
    return re.sub(r"^using std::simd(?:::\w+)?;\s*\n",
                  "", text, flags=re.MULTILINE)


def patch_std_ub_checks(text: str) -> str:
    # Residual `std::ub_checks` reference that prep.sh's macro-strip
    # didn't catch (the use of `ub_checks` outside the macro syntax).
    import re
    return re.sub(r"^using std::ub_checks;\s*\n",
                  "", text, flags=re.MULTILINE)


def patch_strip_using_orphan(text: str) -> str:
    # `using std::ascii;` / `using std::range;` / `using rusty::ascii::EscapeDefault;`
    # — no analogue in our infra. Strip.
    import re
    patterns = [
        r"^using std::ascii;\s*\n",
        r"^using std::range;\s*\n",
        r"^using rusty::ascii::EscapeDefault;\s*\n",
    ]
    for pat in patterns:
        text = re.sub(pat, "", text, flags=re.MULTILINE)
    return text


def patch_make_slice_to_as_slice(text: str) -> str:
    # The transpiler emits `make_slice()` but the rusty::slice
    # iterator headers expose `as_slice()`. Rewrite the call sites.
    return text.replace("->make_slice()", "->as_slice()").replace(
        ".make_slice()", ".as_slice()"
    )


def patch_global_from_raw_parts(text: str) -> str:
    # `::from_raw_parts_mut<T>(...)` — leading `::` looks past the
    # module-scope decl. Strip.
    return text.replace("::from_raw_parts_mut<", "from_raw_parts_mut<").replace(
        "::from_raw_parts<", "from_raw_parts<"
    )


def patch_rusty_ext_leading_colon(text: str) -> str:
    # Same sibling-port pattern as borrow_port P2: leading `::` looks
    # past the auto-namespace and never finds the global rusty_ext.
    # Must avoid touching `::de::rusty_ext::deserialize` and similar
    # nested references where stripping `::` would corrupt the path.
    # Anchor on whitespace/lparen/lbracket before the `::`.
    import re
    return re.sub(r"(?<![:\w])::rusty_ext::", "rusty_ext::", text)


def patch_usize_repeat_u8(text: str) -> str:
    # `usize::repeat_u8(N)` is a Rust integer method that broadcasts a
    # byte across all bytes of a size_t. There's no analogue in our
    # infra; inline the constant computation.
    import re
    def repl(m: "re.Match[str]") -> str:
        byte = int(m.group(1))
        # Replicate byte across sizeof(size_t) bytes.
        n = 0
        for _ in range(8):  # assume 64-bit size_t
            n = (n << 8) | byte
        return f"size_t(0x{n:016x}ULL)"
    return re.sub(r"usize::repeat_u8\((\d+)\)", repl, text)


def patch_unreachable_in_if(text: str) -> str:
    # The transpiler emits `if (rusty::intrinsics::unreachable() && X)`
    # as a placeholder where it couldn't lower an if-let / pattern-match
    # condition. `unreachable()` returns `void` so it can't be used as a
    # bool. Note `if constexpr (false)` alone does NOT fully discard the
    # branch when the condition isn't template-dependent — the branch's
    # body is still type-checked at template definition, surfacing ADL
    # collisions with POSIX `read`/`write`. So we rewrite the condition
    # AND `#if 0`-out the body via a separate brace-counted patch in
    # patch_unreachable_branch_block. Here we just normalize the head.
    import re
    text = re.sub(
        r"if \(rusty::intrinsics::unreachable\(\)[^{]*?\{",
        "if (false) {",
        text,
    )
    return text


def patch_unreachable_branch_block(text: str) -> str:
    # After `patch_unreachable_in_if`, walk the file and for every
    # `if (false) {` (originating from an unreachable-emit), brace-count
    # the body and prefix `#if 0`/postfix `#endif` so type checking is
    # skipped entirely.
    out: list[str] = []
    i = 0
    needle = "if (false) {"
    while True:
        idx = text.find(needle, i)
        if idx == -1:
            out.append(text[i:])
            break
        # Append everything up to and including the `if (false) {`.
        out.append(text[i:idx])
        out.append(needle)
        out.append("\n#if 0\n")
        j = idx + len(needle)
        depth = 1
        while j < len(text) and depth > 0:
            ch = text[j]
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
                if depth == 0:
                    break
            j += 1
        # j points at the closing `}`. Append body inside #if 0.
        out.append(text[idx + len(needle):j])
        out.append("\n#endif\n")
        out.append(text[j])  # the closing `}`
        i = j + 1
    return "".join(out)


def patch_len_self_placeholder(text: str) -> str:
    # The transpiler emits `/* len!(self) */` as a placeholder for the
    # `len!()` macro from rustc's iter!() expansion. Stub with `0` so
    # the call typechecks (the body is from iterator-macro expansion
    # and would be hand-port territory regardless).
    return text.replace("/* len!(self) */", "0")


def patch_qualify_get_disjoint_mut_error(text: str) -> str:
    # Two facts about the transpiler's emit:
    # (a) It emits the type definition inside `namespace core_slice_port`
    #     as `export enum class core_slice_port::GetDisjointMutError`
    #     — a qualified declarator, which is illegal C++. Strip.
    # (b) It emits a free function `get_disjoint_check_valid` whose
    #     body lives inside many nested anonymous namespaces (orphan
    #     impl emit) where GetDisjointMutError is not visible.
    #     Qualify all use sites with `core_slice_port::`.
    import re

    # (a) strip qualifier from the type decl/definition lines.
    for needle, repl in [
        ("export enum class core_slice_port::GetDisjointMutError",
         "export enum class GetDisjointMutError"),
        ("export constexpr core_slice_port::GetDisjointMutError core_slice_port::GetDisjointMutError_IndexOutOfBounds",
         "export constexpr GetDisjointMutError GetDisjointMutError_IndexOutOfBounds"),
        ("export constexpr core_slice_port::GetDisjointMutError core_slice_port::GetDisjointMutError_OverlappingIndices",
         "export constexpr GetDisjointMutError GetDisjointMutError_OverlappingIndices"),
        ("inline constexpr core_slice_port::GetDisjointMutError core_slice_port::GetDisjointMutError_IndexOutOfBounds",
         "inline constexpr GetDisjointMutError GetDisjointMutError_IndexOutOfBounds"),
        ("inline constexpr core_slice_port::GetDisjointMutError core_slice_port::GetDisjointMutError_OverlappingIndices",
         "inline constexpr GetDisjointMutError GetDisjointMutError_OverlappingIndices"),
    ]:
        text = text.replace(needle, repl)

    # (b) qualify use sites — but only AFTER line ~6300 (the orphan-impl
    # block). Cheaper proxy: only qualify inside the body of
    # `get_disjoint_check_valid` function. Find the function DEFINITION
    # (not the forward decl earlier in the file) by anchoring on the
    # opening `{` after the parameter list.
    #
    # First search for the forward decl `... size_t len);` and start
    # the body search AFTER it.
    fwd_decl_marker = "get_disjoint_check_valid(const std::array<I, rusty::sanitize_array_capacity<N>()>& indices, size_t len);"
    fwd_idx = text.find(fwd_decl_marker)
    search_start = (fwd_idx + len(fwd_decl_marker)) if fwd_idx != -1 else 0
    func_start = text.find(
        "rusty::Result<rusty::Unit, core_slice_port::GetDisjointMutError> get_disjoint_check_valid(",
        search_start,
    )
    if func_start == -1:
        # The forward-decl pass above replaced bare with qualified; if
        # the definition hasn't been qualified yet, search for the bare
        # form (which only appears once after the forward-decl strip).
        func_start = text.find(
            "rusty::Result<rusty::Unit, GetDisjointMutError> get_disjoint_check_valid(",
            search_start,
        )
    if func_start != -1:
        # Walk forward to find matching close `}` at brace depth 0.
        brace_depth = 0
        in_body = False
        i = func_start
        end = -1
        while i < len(text):
            ch = text[i]
            if ch == "{":
                brace_depth += 1
                in_body = True
            elif ch == "}":
                brace_depth -= 1
                if in_body and brace_depth == 0:
                    end = i + 1
                    break
            i += 1
        if end != -1:
            body = text[func_start:end]
            body = re.sub(
                r"(?<![:\w])GetDisjointMutError(?![:\w])",
                "core_slice_port::GetDisjointMutError",
                body,
            )
            body = re.sub(
                r"(?<![:\w])GetDisjointMutError_IndexOutOfBounds(?![:\w])",
                "core_slice_port::GetDisjointMutError_IndexOutOfBounds",
                body,
            )
            body = re.sub(
                r"(?<![:\w])GetDisjointMutError_OverlappingIndices(?![:\w])",
                "core_slice_port::GetDisjointMutError_OverlappingIndices",
                body,
            )
            # Also qualify rusty_ext::is_in_bounds / is_overlapping which
            # resolve only inside an open `core_slice_port::rusty_ext`
            # block — the orphan-impl function is at deeper namespace
            # nesting and ADL can't find them.
            body = re.sub(
                r"(?<![:\w])rusty_ext::(is_in_bounds|is_overlapping)\b",
                r"core_slice_port::rusty_ext::\1",
                body,
            )
            text = text[:func_start] + body + text[end:]
    return text


def patch_stub_orphan_impls(text: str) -> str:
    # The transpiler emits methods of types defined in other TUs (e.g.
    # `impl X for usize { fn is_in_bounds(...) }`) as free-standing
    # functions referencing `this`/`(*this)` with a `const` qualifier —
    # neither is legal C++ outside a member context. Wrap each block
    # in `#if 0 / #endif`. Sibling-port pattern from borrow_port P3.
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


def patch_qualify_iter_ascii_bare(text: str) -> str:
    # `iter::FlatMap<...>` and `ascii::EscapeDefault` emitted bare
    # inside the orphan-impl block at the bottom of the file. Qualify
    # with core_slice_port:: so they resolve.
    return text.replace("iter::FlatMap", "core_slice_port::iter::FlatMap").replace(
        "ascii::EscapeDefault", "rusty::ascii::EscapeDefault"
    )


def patch_visit_byte_buf(text: str) -> str:
    # Same as borrow_port P1: stub visit_byte_buf because rusty::Vec
    # isn't visible from the global module fragment.
    import re
    return re.sub(
        r"template<typename E>\nrusty::Result<Value, E> visit_byte_buf\(rusty::Vec<uint8_t> value\) \{\n"
        r"return rusty::Result<Value, E>::Ok\(rusty::as_u8_slice\(value\)\);\n"
        r"\}",
        "template<typename E>\n"
        "rusty::Result<Value, E> visit_byte_buf(auto&& value) {\n"
        "(void)value; return rusty::Result<Value, E>::Err(E{});\n"
        "}",
        text,
    )


def patch_file(path: Path) -> bool:
    text = path.read_text()
    original = text
    text = patch_bound(text)
    text = patch_control_flow(text)
    text = patch_convert(text)
    text = patch_remaining_std_ops(text)
    text = patch_std_range(text)
    text = patch_std_ptr(text)
    text = patch_std_ascii(text)
    text = patch_size_of(text)
    text = patch_strip_orphan_imports(text)
    text = patch_strip_using_simd(text)
    text = patch_std_ub_checks(text)
    text = patch_strip_using_orphan(text)
    text = patch_make_slice_to_as_slice(text)
    text = patch_global_from_raw_parts(text)
    text = patch_rusty_ext_leading_colon(text)
    text = patch_usize_repeat_u8(text)
    text = patch_visit_byte_buf(text)
    text = patch_unreachable_in_if(text)
    text = patch_unreachable_branch_block(text)
    text = patch_len_self_placeholder(text)
    text = patch_qualify_get_disjoint_mut_error(text)
    text = patch_qualify_iter_ascii_bare(text)
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
    path = cpp_out / SLICE_FILE
    if not path.exists():
        print(f"error: {path} does not exist")
        return 1
    changed = patch_file(path)
    if changed:
        print(f"core_slice_port patches applied to {path.name}")
    else:
        print(f"core_slice_port: no patches needed (already clean or idempotent)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
