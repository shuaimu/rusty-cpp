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
    text = text.replace("std::range::", "rusty::ops::")
    # The transpiler also emits bare `range::Range<T>` etc. when the
    # `use range::*;` is processed. Resolve to rusty::ops::Range.
    # Anchor on whitespace/`<`/`(`/`,` so we don't mangle `Foo::range::`
    # (no such path exists today, but defensive).
    import re
    text = re.sub(r"(?<![:\w])range::Range\b", "rusty::ops::Range", text)
    text = re.sub(r"(?<![:\w])range::RangeInclusive\b",
                  "rusty::ops::RangeInclusive", text)
    return text


def patch_std_ptr(text: str) -> str:
    return text.replace("std::ptr::", "rusty::ptr::")


def patch_std_ascii(text: str) -> str:
    return text.replace("std::ascii::", "rusty::ascii::")


def patch_size_of(text: str) -> str:
    # `size_of::<T>()` and `size_of<T>()` are emitted from `mem::size_of`;
    # neither exists in C++; map to `sizeof(T)`.
    # Use angle-bracket balancing to handle nested template args like
    # `size_of<std::array<size_t, 4>>()`.
    out: list[str] = []
    i = 0
    while True:
        idx = text.find("size_of<", i)
        if idx == -1:
            out.append(text[i:])
            break
        # Word-boundary check: char before `size_of` must not be ident-ish.
        if idx > 0 and (text[idx - 1].isalnum() or text[idx - 1] == "_"):
            out.append(text[i:idx + 1])
            i = idx + 1
            continue
        out.append(text[i:idx])
        # Walk past `size_of<` and bracket-balance to the matching `>`.
        j = idx + len("size_of<")
        depth = 1
        while j < len(text) and depth > 0:
            ch = text[j]
            if ch == "<":
                depth += 1
            elif ch == ">":
                depth -= 1
                if depth == 0:
                    break
            j += 1
        if depth != 0 or j + 2 > len(text) or text[j + 1:j + 3] != "()":
            # Not a `size_of<...>()` form — emit as-is.
            out.append(text[idx:j + 1])
            i = j + 1
            continue
        inner = text[idx + len("size_of<"):j]
        out.append(f"sizeof({inner})")
        i = j + len(">()")
    return "".join(out)


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


def _wrap_block_starting_at(text: str, anchor: str, kind: str) -> str:
    """Wrap a self-contained item starting with `anchor` in `#if 0 ... #endif`.

    Walks from the anchor through the opening brace, then brace-counts to
    the matching close. Idempotent: if the wrap markers are already
    present around the anchor, returns text unchanged.

    Also walks BACKWARDS from the anchor over any preceding lines that
    are part of the item header (`template<...>`, `requires(...)`,
    `[[attributes]]`, `// comments`, `///` doc comments, blank lines).
    Without this, wrapping a function body alone leaves an orphan template
    head outside the `#if 0` block, which the compiler treats as an
    extraneous template parameter list.
    """
    idx = text.find(anchor)
    if idx == -1:
        return text
    # First normalize `idx` to the start of its own line so subsequent
    # walk-back logic doesn't get confused by leading whitespace before
    # the anchor.
    line_nl = text.rfind("\n", 0, idx)
    if line_nl != -1:
        idx = line_nl + 1
    else:
        idx = 0
    # Walk backwards past header-attached lines so the wrap includes the
    # full item header.
    new_idx = idx
    while new_idx > 0:
        # Find the start of the previous line.
        prev_nl = text.rfind("\n", 0, new_idx - 1)
        line_start = prev_nl + 1 if prev_nl != -1 else 0
        prev_line = text[line_start:new_idx].rstrip("\n")
        stripped = prev_line.lstrip()
        # Match: template head, requires clause, attribute, doc comment,
        # regular comment, or pure blank line that immediately precedes the
        # header (we keep the blank line OUT of the wrap actually — stop).
        if (
            stripped.startswith("template<")
            or stripped.startswith("template <")
            or stripped.startswith("export template<")
            or stripped.startswith("export template <")
            or stripped.startswith("requires ")
            or stripped.startswith("requires(")
            or stripped.startswith("[[")
            or stripped.startswith("///")
            or stripped.startswith("//")
        ):
            new_idx = line_start
            continue
        break
    idx = new_idx
    # Idempotency check: look backwards for an already-emitted marker.
    head = text[:idx]
    last_marker = head.rfind("#if 0  // patcher: stub ")
    last_endif = head.rfind("#endif  // patcher: end stub")
    if last_marker != -1 and (last_endif == -1 or last_endif < last_marker):
        return text
    # Re-find the anchor relative to the (possibly-shifted) idx so brace
    # walking starts after the original anchor's body, not the extended
    # header region.
    body_anchor = text.find(anchor, idx)
    j = body_anchor + len(anchor)
    if anchor.endswith("{"):
        depth = 1
    else:
        while j < len(text) and text[j] != "{":
            j += 1
        if j >= len(text):
            return text
        depth = 1
        j += 1
    while j < len(text) and depth > 0:
        if text[j] == "{":
            depth += 1
        elif text[j] == "}":
            depth -= 1
        j += 1
    # Eat optional trailing `;` after closing `}`.
    if j < len(text) and text[j] == ";":
        j += 1
    return (
        text[:idx]
        + f"#if 0  // patcher: stub {kind}\n"
        + text[idx:j]
        + f"\n#endif  // patcher: end stub {kind}\n"
        + text[j:]
    )


def patch_stub_broken_extension_items(text: str) -> str:
    # Items at the bottom of the cppm that reference symbols our port
    # infrastructure can't yet materialize. Wrapping in `#if 0` keeps
    # the file compilable; once the missing infra lands the wraps can
    # be peeled off in reverse-dependency order.
    text = _wrap_block_starting_at(
        text,
        "export struct EscapeAscii {",
        "EscapeAscii — references core_slice_port::iter::FlatMap which is not ported",
    )
    text = _wrap_block_starting_at(
        text,
        "rusty::Option<std::tuple<Direction, size_t>> split_point_of(const auto& range) {",
        "split_point_of — None_t to std::tuple<Direction, size_t> conversion fails in the match-arm IIFE",
    )
    text = _wrap_block_starting_at(
        text,
        "void copy_from_slice_impl(std::span<T> dest, std::span<const T> src) {",
        "copy_from_slice_impl — `[[noreturn]]` attribute inside SafeFn template arg is a parse error",
    )
    text = _wrap_block_starting_at(
        text,
        "export rusty::Option<size_t> memrchr(uint8_t x, std::span<const uint8_t> text) {",
        "memrchr — uses .align_to<>() on std::span which has no analogue",
    )
    text = _wrap_block_starting_at(
        text,
        "std::span<const T> from_ref(const T& s) {",
        "from_ref — array::from_ref unresolved; replaced by std::span construction",
    )
    text = _wrap_block_starting_at(
        text,
        "std::span<T> from_mut(T& s) {",
        "from_mut — array::from_mut unresolved; replaced by std::span construction",
    )
    # The two Bound-tuple lowering helpers below are emitted with a `_`
    # placeholder param + lambdas that reference `start`/`end` before
    # they're bound. The Rust source matched on the tuple but the
    # transpiler lost the bindings during match-arm lowering. Stub them
    # — slice_port's hot path uses `into_slice_range` (a wrapper that
    # we can shim manually).
    text = _wrap_block_starting_at(
        text,
        "export rusty::range<size_t> into_range_unchecked(size_t len, std::tuple<ops::Bound<size_t>, ops::Bound<size_t>> _) {",
        "into_range_unchecked — match-arm bindings dropped; lambdas reference undeclared start/end",
    )
    text = _wrap_block_starting_at(
        text,
        "export rusty::Option<rusty::range<size_t>> try_into_slice_range(size_t len, std::tuple<ops::Bound<size_t>, ops::Bound<size_t>> _) {",
        "try_into_slice_range — match-arm bindings dropped; lambdas reference undeclared start/end",
    )
    text = _wrap_block_starting_at(
        text,
        "export rusty::range<size_t> into_slice_range(size_t len, std::tuple<ops::Bound<size_t>, ops::Bound<size_t>> _) {",
        "into_slice_range — match-arm bindings dropped; companion of into_range_unchecked",
    )
    # The ASCII / is_ascii_simple / SSE2 path uses primitive intrinsics
    # (uint8_t::is_ascii(), as_chunks, x86 SSE2) that have no rusty
    # equivalents yet. Stub the cluster.
    text = _wrap_block_starting_at(
        text,
        "export bool is_ascii_simple(std::span<const uint8_t> bytes) {",
        "is_ascii_simple — uint8_t.is_ascii() method-call has no analogue",
    )
    text = _wrap_block_starting_at(
        text,
        "bool is_ascii(std::span<const uint8_t> s) {",
        "is_ascii(s) duplicate#1 — empty body",
    )
    text = _wrap_block_starting_at(
        text,
        "bool is_ascii_sse2(std::span<const uint8_t> bytes) {",
        "is_ascii_sse2 — uses x86 SSE2 intrinsics + as_chunks",
    )
    text = _wrap_block_starting_at(
        text,
        "bool is_ascii(std::span<const uint8_t> bytes) {",
        "is_ascii(bytes) duplicate#2 — empty body",
    )
    # The loongarch64 is_ascii has full body but uses bytes[i].is_ascii()
    # on a uint8_t (no method analogue in C++). Stub it — the simple
    # path's stub already removes the redefinition conflict.
    # Note: this anchor is identical to the prior one — only the FIRST
    # wrap finds it; the helper bails on idempotency afterward. To stub
    # the SECOND occurrence we search after the first wrap.
    second_anchor = "bool is_ascii(std::span<const uint8_t> bytes) {"
    first_idx = text.find(second_anchor)
    if first_idx != -1:
        next_idx = text.find(second_anchor, first_idx + len(second_anchor))
        if next_idx != -1:
            # Build wrap by stub_block helper on prefix+suffix split.
            prefix = text[:first_idx + len(second_anchor)]
            rest = text[first_idx + len(second_anchor):]
            rest = _wrap_block_starting_at(
                rest, second_anchor,
                "is_ascii(bytes) duplicate#3 — uses uint8_t.is_ascii() method",
            )
            text = prefix + rest
    # The is_in_bounds overloads for rusty::ops::Range / RangeInclusive
    # try to call `rusty::range::from(self_)` which uses `rusty::range`
    # as a namespace; rusty::range<T> is a template. Stub these
    # specializations — the primary RangeFrom/RangeTo/RangeFull/etc.
    # overloads cover what the hot path needs.
    text = _wrap_block_starting_at(
        text,
        "export bool is_in_bounds(const rusty::ops::Range<size_t>& self_, size_t len) {",
        "is_in_bounds(Range) — uses rusty::range::from which treats rusty::range as a namespace",
    )
    text = _wrap_block_starting_at(
        text,
        "export bool is_in_bounds(const rusty::ops::RangeInclusive<size_t>& self_, size_t len) {",
        "is_in_bounds(RangeInclusive) — same rusty::range_inclusive namespace issue",
    )
    # The cluster of template <typename T> { get / get_mut / get_unchecked
    # / get_unchecked_mut / index / index_mut } for the inclusive range
    # types: they use `into_slice_range()`, `exhausted`, `start`, `end`,
    # `last`, `usize::checked_sub` — none of which exist on rusty's
    # range stubs. Stub each method body.
    inclusive_anchors = [
        # rusty::range_inclusive<size_t>
        "rusty::Option<std::span<const T>> get(rusty::range_inclusive<size_t> self_, std::span<const T> slice) {",
        "rusty::Option<std::span<T>> get_mut(rusty::range_inclusive<size_t> self_, std::span<T> slice) {",
        "std::add_pointer_t<std::add_const_t<std::span<const T>>> get_unchecked(rusty::range_inclusive<size_t> self_, std::add_pointer_t<std::add_const_t<std::span<const T>>> slice) {",
        "std::add_pointer_t<std::span<T>> get_unchecked_mut(rusty::range_inclusive<size_t> self_, std::add_pointer_t<std::span<T>> slice) {",
        "std::span<const T> index(rusty::range_inclusive<size_t> self_, std::span<const T> slice) {",
        "std::span<T> index_mut(rusty::range_inclusive<size_t> self_, std::span<T> slice) {",
        # rusty::ops::RangeToInclusive<size_t>
        "rusty::Option<std::span<const T>> get(rusty::ops::RangeToInclusive<size_t> self_, std::span<const T> slice) {",
        "rusty::Option<std::span<T>> get_mut(rusty::ops::RangeToInclusive<size_t> self_, std::span<T> slice) {",
        "std::add_pointer_t<std::add_const_t<std::span<const T>>> get_unchecked(rusty::ops::RangeToInclusive<size_t> self_, std::add_pointer_t<std::add_const_t<std::span<const T>>> slice) {",
        "std::add_pointer_t<std::span<T>> get_unchecked_mut(rusty::ops::RangeToInclusive<size_t> self_, std::add_pointer_t<std::span<T>> slice) {",
        "std::span<const T> index(rusty::ops::RangeToInclusive<size_t> self_, std::span<const T> slice) {",
        "std::span<T> index_mut(rusty::ops::RangeToInclusive<size_t> self_, std::span<T> slice) {",
    ]
    for anchor in inclusive_anchors:
        text = _wrap_block_starting_at(
            text, anchor,
            "inclusive-range overload — uses into_slice_range/exhausted/last/usize::checked_sub",
        )
    # The EscapeAscii method-definition block (after the forward decls)
    # — these reference EscapeAscii::* members that no longer exist
    # once the struct is stubbed.
    for sig in [
        "rusty::Option<uint8_t> EscapeAscii::next() {",
        "std::tuple<size_t, rusty::Option<size_t>> EscapeAscii::size_hint() const {",
        "auto EscapeAscii::try_fold(Acc init, Fold fold) {",
        "Acc EscapeAscii::fold(Acc init, Fold fold) const {",
        "rusty::Option<uint8_t> EscapeAscii::last() const {",
        "rusty::Option<uint8_t> EscapeAscii::next_back() {",
        "rusty::fmt::Result EscapeAscii::fmt(rusty::fmt::Formatter& f) const {",
    ]:
        text = _wrap_block_starting_at(text, sig, f"EscapeAscii::* — orphan to stubbed struct")
    return text


def patch_qualify_bare_idents(text: str) -> str:
    # Bare identifiers in the extension-trait region at the bottom of
    # the cppm that the compiler can't resolve. The clang error hints
    # at the qualified name; apply the suggestion.
    #
    # Use word-boundary anchors to avoid mangling unrelated occurrences
    # (e.g., we want to qualify `USIZE_BYTES` but not the substring
    # `LO_USIZE`).
    import re
    replacements = [
        # name → qualified form. Order matters when prefixes overlap
        # — but these are word-distinct so order doesn't.
        (r"(?<![:\w])USIZE_BYTES\b", "core_slice_port::USIZE_BYTES"),
        (r"(?<![:\w])LO_USIZE\b", "core_slice_port::LO_USIZE"),
        (r"(?<![:\w])HI_USIZE\b", "core_slice_port::HI_USIZE"),
        # `::memchr_naive(...)` / `::memchr_aligned(...)` — the leading `::`
        # looks at the global namespace where these don't live; drop it
        # so the unqualified name resolves to the namespace-local one.
        (r"::memchr_naive\(", "memchr_naive("),
        (r"::memchr_aligned\(", "memchr_aligned("),
        # contains_zero_byte called as bare `contains_zero_byte(x)` from
        # within the namespace — should resolve, but if it's emitted at
        # global scope (orphan-impl drift), prefix with namespace.
        (r"(?<![:\w])contains_zero_byte\(", "core_slice_port::contains_zero_byte("),
    ]
    for pat, repl in replacements:
        text = re.sub(pat, repl, text)
    return text


def patch_pull_template_head_into_wrap(text: str) -> str:
    # Fixes orphan template heads outside `#if 0` wraps. The wrap helper
    # `_wrap_block_starting_at` now walks backwards over template/requires
    # lines, but for files that were already wrapped by an earlier patcher
    # run, the markers are baked in at the wrong position. Walk every
    # `#if 0  // patcher: stub ...` marker and pull any directly-preceding
    # `template<...>`, `requires(...)`, `[[...]]`, or doc/comment lines
    # INTO the wrap.
    import re
    lines = text.splitlines(keepends=True)
    changed = False
    i = 0
    while i < len(lines):
        line = lines[i]
        if line.lstrip().startswith("#if 0  // patcher: stub "):
            # Walk backwards past header-attached lines.
            j = i - 1
            while j >= 0:
                stripped = lines[j].lstrip()
                if (
                    stripped.startswith("template<")
                    or stripped.startswith("template <")
                    or stripped.startswith("requires ")
                    or stripped.startswith("requires(")
                    or stripped.startswith("[[")
                    or stripped.startswith("///")
                    or stripped.startswith("//")
                ):
                    j -= 1
                    continue
                break
            first_header = j + 1
            if first_header < i:
                # Move `#if 0` line up to before `first_header`.
                marker = lines[i]
                # Remove the existing #if 0 line and re-insert it.
                new_lines = (
                    lines[:first_header]
                    + [marker]
                    + lines[first_header:i]
                    + lines[i + 1:]
                )
                lines = new_lines
                changed = True
                # `i` stays the same because the count is preserved; but
                # we already advance past this region.
                # Next marker is searched from the next line after the
                # wrap header, which is `i + 1` in new layout.
                i = i + 1
                continue
        i += 1
    if changed:
        return "".join(lines)
    return text


def patch_strip_extra_qualification(text: str) -> str:
    # The transpiler emits a few decls/defs WITH the `core_slice_port::`
    # qualifier even though they sit inside `namespace core_slice_port {
    # ... }`. clang treats this as a member-of-namespace declaration —
    # which fails because the forward decl never created the member.
    # Strip the extra qualifier on the LHS of these decls/defs.
    import re
    # Bare forward decl lines for namespace-local constants.
    patterns = [
        (r"^extern const size_t core_slice_port::(LO_USIZE|HI_USIZE|USIZE_BYTES);",
         r"extern const size_t \1;"),
        (r"^constexpr size_t core_slice_port::(LO_USIZE|HI_USIZE|USIZE_BYTES) =",
         r"constexpr size_t \1 ="),
        # `bool core_slice_port::contains_zero_byte(size_t x);` and the
        # matching definition at body-emit time.
        (r"^bool core_slice_port::contains_zero_byte\(",
         r"bool contains_zero_byte("),
    ]
    for pat, repl in patterns:
        text = re.sub(pat, repl, text, flags=re.MULTILINE)
    return text


def patch_std_intrinsics(text: str) -> str:
    # `std::intrinsics::assume` → `rusty::intrinsics::assume`. Same for
    # other std::intrinsics:: functions.
    text = text.replace("std::intrinsics::", "rusty::intrinsics::")
    # `unchecked_sub` is Rust's `intrinsics::unchecked_sub`. C++ has no
    # unchecked-arithmetic intrinsic — use normal `-` (UB on overflow is
    # implicit and well-defined for unsigned).
    text = text.replace(
        "rusty::intrinsics::unchecked_sub(", "rusty::intrinsics::__unchecked_sub("
    )
    # Then provide the alias in the function call form: actually simpler
    # to inline. Reverse the above by replacing with a lambda.
    import re
    text = re.sub(
        r"rusty::intrinsics::__unchecked_sub\(([^,]+),\s*([^)]+)\)",
        r"((\1) - (\2))",
        text,
    )
    return text


def patch_rusty_range_from(text: str) -> str:
    # The transpiler emits `rusty::range::from(x)`, `rusty::range_from::
    # from(x)`, `rusty::range_inclusive::from(x)`, etc. — treating
    # `rusty::range` as a namespace/class when it's actually a template
    # type. These calls are Rust's `Self::from(other_range)` conversion
    # constructors. C++ doesn't need the conversion (the inner range type
    # is already what we want), so strip the wrapper.
    import re
    for name in (
        "range",
        "range_from",
        "range_to",
        "range_full",
        "range_inclusive",
        "range_to_inclusive",
    ):
        text = re.sub(
            r"rusty::" + name + r"::from\(([^()]+(?:\([^()]*\))?)\)",
            r"\1",
            text,
        )
        # Also handle `rusty::range_inclusive::into_slice_range(x, y)` etc.
        # by stripping the namespace-qualified call and forwarding to a
        # local helper. For now we just remove the call (fall through to
        # the IIFE's fallback else branch).
        text = re.sub(
            r"rusty::" + name + r"::into_slice_range\(([^()]+(?:\([^()]*\))?)\)",
            r"\1",
            text,
        )
    return text


def patch_strip_leading_colon_calls(text: str) -> str:
    # `::contains_zero_byte(...)`, `::const_min(...)`, `::ptr_rotate_*`,
    # `::into_slice_range(...)`, `::try_into_slice_range(...)` all live
    # inside `namespace core_slice_port { ... }` and need unqualified
    # lookup. Leading `::` reroutes to global ns where they don't exist.
    import re
    names = [
        "contains_zero_byte",
        "const_min",
        "ptr_rotate_memmove",
        "ptr_rotate_gcd",
        "ptr_rotate_swap",
        "into_slice_range",
        "try_into_slice_range",
        "slice_index_fail",
        "get_offset_len_noubcheck",
        "get_offset_len_mut_noubcheck",
    ]
    for name in names:
        text = re.sub(r"(?<![:\w])::" + re.escape(name) + r"\(",
                      name + "(", text)
    return text


def patch_slice_from_raw_parts(text: str) -> str:
    # `ptr::slice_from_raw_parts<T>` doesn't exist in rusty::ptr — Rust
    # uses it to materialize a slice from a (data, len) pair. C++ uses
    # the std::span constructor directly. Inline the construction.
    # Same for the `_mut` variant.
    # Replace the whole body of the locally-defined from_raw_parts(_mut)
    # functions with a direct std::span construction. Anchoring on the
    # full function header avoids depth-tracking through std::move(len)'s
    # nested parens.
    needle_const = (
        "std::span<const T> from_raw_parts(std::add_pointer_t<std::add_const_t<T>> data, size_t len) {"
    )
    body_const = (
        "    // patcher: body inlined — Rust's ptr::slice_from_raw_parts is\n"
        "    // not exposed in rusty::ptr; std::span ctor is the C++ analogue.\n"
        "    return std::span<const T>(data, len);\n"
        "}"
    )
    text = _replace_function_body(text, needle_const, body_const)

    needle_mut = (
        "std::span<T> from_raw_parts_mut(std::add_pointer_t<T> data, size_t len) {"
    )
    body_mut = (
        "    // patcher: body inlined — see from_raw_parts above.\n"
        "    return std::span<T>(data, len);\n"
        "}"
    )
    text = _replace_function_body(text, needle_mut, body_mut)
    return text


def _replace_function_body(text: str, header: str, new_body: str) -> str:
    """Find `header` then brace-count to its closing `}` and replace.

    `header` must end with the opening `{`. Idempotent: if the file already
    contains `new_body` at this site, leaves text unchanged.
    """
    idx = text.find(header)
    if idx == -1:
        return text
    body_start = idx + len(header)
    j = body_start
    depth = 1
    while j < len(text) and depth > 0:
        if text[j] == "{":
            depth += 1
        elif text[j] == "}":
            depth -= 1
        j += 1
    return text[:idx] + header + "\n" + new_body + text[j:]


def patch_array_from_ref(text: str) -> str:
    # `array::from_ref(s)` / `array::from_mut(s)` collides with
    # `std::array` template. The conversion is "wrap a single ref in a
    # 1-element slice" → use std::span directly.
    import re
    text = re.sub(
        r"return array::from_ref\(([^)]+)\);",
        r"return std::span<const T>(&\1, 1);",
        text,
    )
    text = re.sub(
        r"return array::from_mut\(([^)]+)\);",
        r"return std::span<T>(&\1, 1);",
        text,
    )
    return text


def patch_cfg_macro_placeholder(text: str) -> str:
    # The transpiler leaves `/* cfg!(feature = "...") */` as a placeholder
    # comment in expression position. That's a parse error when negated:
    # `!/* cfg!(...) */ && ...`. Rewrite the negated form to `false` (the
    # safe default — `optimize_for_size` cfg is disabled in our build).
    import re
    text = re.sub(
        r"!\s*/\*\s*cfg!\([^*]*\)\s*\*/",
        "false",
        text,
    )
    return text


def patch_usize_repeat_u8_var(text: str) -> str:
    # The literal form is handled by `patch_usize_repeat_u8`. For the
    # variable form `usize::repeat_u8(std::move(x))`, emit a runtime
    # expression that broadcasts the byte into a size_t. Note we must
    # not eagerly evaluate `x` here — wrap the whole call in a lambda
    # so move semantics still apply.
    import re
    def repl(m: "re.Match[str]") -> str:
        arg = m.group(1)
        return (
            f"([&] {{ auto __b = static_cast<size_t>({arg}); "
            f"return (__b * static_cast<size_t>(0x0101010101010101ULL)); }}())"
        )
    return re.sub(r"usize::repeat_u8\(([^()]+(?:\([^()]*\))?)\)", repl, text)


def patch_add_is_overlapping_decl(text: str) -> str:
    # `core_slice_port::rusty_ext::is_overlapping` is called from the
    # get_disjoint_check_valid body (qualified via the earlier
    # patch_qualify_get_disjoint_mut_error pass) but the rusty_ext
    # namespace inside core_slice_port only forward-declares
    # `is_in_bounds`. Add forward decls for `is_overlapping` for each
    # range-like type the get_disjoint_check path needs.
    needle = "    export bool is_in_bounds(const rusty::ops::RangeInclusive<size_t>& self_, size_t len);\n"
    if needle not in text:
        return text
    additions = (
        "    // patcher: is_overlapping fwd-decls for get_disjoint_check_valid call sites.\n"
        "    template <class L, class R>\n"
        "    bool is_overlapping(const L& self_, const R& other);\n"
    )
    if "patcher: is_overlapping fwd-decls" in text:
        return text
    return text.replace(needle, needle + additions)


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
    text = patch_stub_broken_extension_items(text)
    text = patch_qualify_bare_idents(text)
    text = patch_strip_extra_qualification(text)
    text = patch_strip_leading_colon_calls(text)
    text = patch_std_intrinsics(text)
    text = patch_rusty_range_from(text)
    text = patch_slice_from_raw_parts(text)
    text = patch_cfg_macro_placeholder(text)
    text = patch_usize_repeat_u8_var(text)
    text = patch_pull_template_head_into_wrap(text)
    text = patch_add_is_overlapping_decl(text)
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
