"""Shared post-transpile-patch helpers for rustc collection-test ports.

Each port (binary_heap_tests_port, linked_list_tests_port, vec_tests_port,
…) has its own `post_transpile_patch.py` that imports these helpers and
applies the patches appropriate for its collection.

Each helper is idempotent and a no-op if the marker is missing.
"""

from __future__ import annotations
import re
from pathlib import Path
from typing import Iterable


# ─────────────────────────────────────────────────────────────────────────
# GMF / module-level injections
# ─────────────────────────────────────────────────────────────────────────


def inject_test_runner_include(text: str) -> str:
    """Add `#include <rusty/test_runner.hpp>` after rusty/rusty.hpp."""
    marker = '#include <rusty/rusty.hpp>'
    inject = '#include <rusty/rusty.hpp>\n#include <rusty/test_runner.hpp>'
    if '#include <rusty/test_runner.hpp>' in text or marker not in text:
        return text
    return text.replace(marker, inject, 1)


def inject_module_imports(text: str, module_name: str, imports: list[str]) -> str:
    """Inject `import X;` lines after `export module <module_name>;`.

    `imports` is a list of module-name strings (e.g. `["vec_port.vec",
    "binary_heap_port"]`). Idempotent — if all imports are already present,
    no-op.
    """
    marker = f'export module {module_name};'
    if marker not in text:
        return text
    missing = [m for m in imports if f'import {m};' not in text]
    if not missing:
        return text
    inject = marker + '\n\n// patcher-injected module imports:\n'
    for m in missing:
        inject += f'import {m};\n'
    return text.replace(marker, inject.rstrip(), 1)


def inject_using_decls_in_namespace(
    text: str, namespace: str, using_lines: list[str]
) -> str:
    """Inject `using` decls (or arbitrary code) inside the test namespace."""
    marker = f'namespace {namespace} {{'
    if marker not in text:
        return text
    if any(line in text for line in using_lines):
        # At least one is already present — assume idempotent.
        return text
    inject = marker + '\n\n// patcher-injected:\n' + '\n'.join(using_lines) + '\n'
    return text.replace(marker, inject, 1)


# ─────────────────────────────────────────────────────────────────────────
# Common rewrites
# ─────────────────────────────────────────────────────────────────────────


def rewrite_rusty_vec_to_global(text: str) -> str:
    """Rewrite `rusty::Vec` → `::Vec` (the vec_port.vec module exports it)."""
    return re.sub(r'\brusty::Vec\b', '::Vec', text)


def strip_const_on_movable_locals(text: str, type_name: str) -> str:
    """Strip `const` from `const auto NAME = ::TYPE{...}` patterns where
    the Vec/Box value is later moved. `std::move(const T&)` selects the
    copy ctor (shallow copy) → double-free at scope exit."""
    return re.sub(
        rf'const auto (\w+) = ::{re.escape(type_name)}\{{',
        rf'auto \1 = ::{type_name}{{',
        text,
    )


def constrain_prelude_clone(text: str) -> str:
    """Constrain the GMF prelude `clone` template to only fire when
    `value.clone()` exists. Without the constraint, both the prelude clone
    AND `<rusty/move.hpp>`'s Copy-only clone match for types like Vec<int>
    (which has no inherent `.clone()`), causing ambiguity."""
    old = (
        '// Clone: dispatches to .clone() if available, otherwise copy-constructs.\n'
        'template<typename T>\n'
        'auto clone(const T& value) {\n'
        'if constexpr (requires { value.clone(); }) {\n'
        'return value.clone();\n'
        '} else {\n'
        'return value;\n'
        '}\n'
        '}'
    )
    new = (
        '// Clone: dispatches to .clone() if available; Copy types fall through to ::rusty::clone (move.hpp).\n'
        'template<typename T>\n'
        'requires requires(const T& v) { v.clone(); }\n'
        'auto clone(const T& value) {\n'
        'return value.clone();\n'
        '}'
    )
    return text.replace(old, new)


def stub_visit_byte_buf(text: str) -> str:
    """Stub `visit_byte_buf(::Vec<uint8_t>)` GMF helper. `::Vec` is not
    visible in the GMF (only inside the module purview after `import
    vec_port.vec;`). Replace with a generic-arg rebody returning Err."""
    old = (
        'template<typename E>\n'
        'rusty::Result<Value, E> visit_byte_buf(::Vec<uint8_t> value) {\n'
        'return rusty::Result<Value, E>::Ok(rusty::as_u8_slice(value));\n'
        '}'
    )
    new = (
        'template<typename E>\n'
        'rusty::Result<Value, E> visit_byte_buf(auto&&) {\n'
        'return rusty::Result<Value, E>::Err(E{});\n'
        '}'
    )
    return text.replace(old, new)


# ─────────────────────────────────────────────────────────────────────────
# Test-body stubbing
# ─────────────────────────────────────────────────────────────────────────


def stub_test_body(text: str, test_name: str, reason: str) -> str:
    """Replace the body of a TEST_CASE("name") { ... } block with a printf
    skip-message + early return. Uses brace-balanced scan to find the close."""
    pattern = re.compile(
        r'(TEST_CASE\("' + re.escape(test_name) + r'"\)\s*\{)',
        re.MULTILINE,
    )
    m = pattern.search(text)
    if not m:
        return text
    open_idx = m.end() - 1
    depth = 1
    i = open_idx + 1
    n = len(text)
    in_str = False
    in_line_comment = False
    in_block_comment = False
    while i < n and depth > 0:
        ch = text[i]
        prev = text[i - 1] if i > 0 else ''
        if in_line_comment:
            if ch == '\n':
                in_line_comment = False
        elif in_block_comment:
            if ch == '/' and prev == '*':
                in_block_comment = False
        elif in_str:
            if ch == '"' and prev != '\\':
                in_str = False
        else:
            if ch == '/' and i + 1 < n and text[i + 1] == '/':
                in_line_comment = True
            elif ch == '/' and i + 1 < n and text[i + 1] == '*':
                in_block_comment = True
            elif ch == '"':
                in_str = True
            elif ch == '{':
                depth += 1
            elif ch == '}':
                depth -= 1
                if depth == 0:
                    close_idx = i
                    break
        i += 1
    else:
        return text

    if '/* PATCHED: skipped */' in text[open_idx:close_idx]:
        return text

    replacement = (
        '{\n'
        '    /* PATCHED: skipped */\n'
        f'    std::printf("[port] SKIP {test_name}: {reason}\\n");\n'
        '    return;\n'
        '}'
    )
    return text[: open_idx] + replacement + text[close_idx + 1 :]


def stub_tests(text: str, test_names: Iterable[str], reason: str) -> str:
    for name in test_names:
        text = stub_test_body(text, name, reason)
    return text


def stub_all_remaining_tests(text: str, reason: str) -> str:
    """Stub every TEST_CASE in the file that isn't already stubbed."""
    seen: set[str] = set()
    for m in re.finditer(r'TEST_CASE\("([^"]+)"\)', text):
        seen.add(m.group(1))
    for name in sorted(seen):
        text = stub_test_body(text, name, reason)
    return text
