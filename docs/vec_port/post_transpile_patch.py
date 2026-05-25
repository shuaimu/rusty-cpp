#!/usr/bin/env python3
"""Post-transpile patches for the vec_port C++20 module port.

Each patch addresses a specific cluster of errors documented in
docs/rusty-std-book.md Chapter 4. As transpiler fixes land that
make a patch redundant, mark its entry with `# RETIRED:` and leave
the function in place for one re-transpile cycle as a guard against
regression.

Usage:
    python3 post_transpile_patch.py /tmp/vec_port/cpp_out/

Idempotent: rerunning detects already-applied patches and skips.
"""

import re
import sys
from pathlib import Path


def patch_set_len_on_drop_copy_assign(cpp_out: Path) -> int:
    """Cluster V-D: SetLenOnDrop has `size_t& len` (reference field) so
    its copy assignment operator can't be `= default` (implicitly
    deleted). Rust types don't have implicit copy-assign; emit
    `= delete` instead.

    Also: copy ctor `= default` on a struct with reference field is
    fine (binds the reference identically), but copy-assign isn't —
    references can't rebind.
    """
    path = cpp_out / "vec_port.vec.set_len_on_drop.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    old = "    SetLenOnDrop& operator=(const SetLenOnDrop&) = default;"
    new = "    SetLenOnDrop& operator=(const SetLenOnDrop&) = delete;"
    if old in text:
        text = text.replace(old, new)
        path.write_text(text)
        return 1
    if new in text:
        return 0  # already applied
    return 0


def patch_is_zero_free_fn_const(cpp_out: Path) -> int:
    """Cluster V-F (related to V-D): is_zero.cppm emits free functions
    with `const` qualifier (`bool is_zero() const` outside a class).
    Strip the trailing `const` for free-function emissions.
    """
    path = cpp_out / "vec_port.vec.is_zero.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    # Match `bool fn_name(...) const {` at module/file scope only —
    # the `const` qualifier is invalid there.
    # Pattern is conservative: only strip when the line starts at
    # column 0 (no indent, so it's free-fn scope).
    text = re.sub(
        r"^(bool [A-Za-z_][A-Za-z_0-9]*\([^)]*\))\s+const(\s*\{)",
        r"\1\2",
        text,
        flags=re.MULTILINE,
    )
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_std_collections_to_rusty(cpp_out: Path) -> int:
    """Cluster V-B: `std::collections::TryReserveError` doesn't exist
    in C++ std. Map to `rusty::collections::TryReserveError` (defined
    in include/rusty/collections.hpp). Also strip the namespace-using
    Rust glob `using namespace std::collections::TryReserveErrorKind;`
    which can't apply to enum-class members in C++ (use scoped names
    instead).
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        text = text.replace(
            "std::collections::TryReserveError",
            "rusty::collections::TryReserveError",
        )
        text = text.replace(
            "std::collections::TryReserveErrorKind",
            "rusty::collections::TryReserveErrorKind",
        )
        # The `using namespace ...::TryReserveErrorKind;` would import
        # CapacityOverflow + AllocError as unqualified. After the
        # rusty:: switch above, that becomes
        # `using namespace rusty::collections::TryReserveErrorKind;`
        # which doesn't work for an enum class. Replace with the
        # explicit prefix at call sites in raw_vec source via prep.sh;
        # here, just delete the using line as a no-op.
        text = text.replace(
            "// Rust-only: using namespace rusty::collections::TryReserveErrorKind;\n",
            "",
        )
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_std_ptr_to_rusty(cpp_out: Path) -> int:
    """Cluster V-C: `std::ptr::Unique<T>` / `std::ptr::Alignment` don't
    exist. Map to `rusty::ptr::Unique<T>` / `rusty::ptr::Alignment`
    (added to include/rusty/ptr.hpp for this port).
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        text = text.replace("std::ptr::Unique", "rusty::ptr::Unique")
        text = text.replace("std::ptr::Alignment", "rusty::ptr::Alignment")
        text = text.replace("std::ptr::NonNull", "rusty::ptr::NonNull")
        # also `std::ptr::slice_from_raw_parts_mut` is a free function
        text = text.replace(
            "std::ptr::slice_from_raw_parts_mut",
            "rusty::ptr::from_raw_parts_mut",
        )
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_trim_cmakelists(cpp_out: Path) -> int:
    """Reduce CMakeLists.txt to core modules only — drop auxiliary
    modules (cow, extract_if, in_place_*, peek_mut, splice, spec_*,
    partial_eq) that have their own deeper cluster issues (V-A...V-E)
    and aren't needed for a Phase C smoke test.
    """
    path = cpp_out / "CMakeLists.txt"
    if not path.exists():
        return 0
    text = path.read_text()
    if "# REDUCED-SCOPE BUILD" in text:
        return 0  # already trimmed
    reduced = """# Auto-generated by rusty-cpp-transpiler, then trimmed by
# docs/vec_port/post_transpile_patch.py.
# REDUCED-SCOPE BUILD — core 7 modules only. Auxiliary modules
# (cow, extract_if, in_place_*, peek_mut, splice, spec_*, partial_eq)
# are not built until their cluster-specific issues are addressed
# (see rusty-std-book Chapter 4).

cmake_minimum_required(VERSION 3.28)
project(vec_port VERSION 0.0.1 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library(vec_port
    vec_port.cppm
    vec_port.raw_vec.cppm
    vec_port.vec.is_zero.cppm
    vec_port.vec.set_len_on_drop.cppm
    vec_port.vec.into_iter.cppm
    vec_port.vec.drain.cppm
    vec_port.vec.cppm
)

target_sources(vec_port PUBLIC FILE_SET CXX_MODULES FILES
    vec_port.cppm
    vec_port.raw_vec.cppm
    vec_port.vec.is_zero.cppm
    vec_port.vec.set_len_on_drop.cppm
    vec_port.vec.into_iter.cppm
    vec_port.vec.drain.cppm
    vec_port.vec.cppm
)
"""
    path.write_text(reduced)
    return 1


def main(cpp_out: Path):
    patches = [
        ("set_len_on_drop copy-assign", patch_set_len_on_drop_copy_assign),
        ("is_zero free-fn const qualifier", patch_is_zero_free_fn_const),
        ("std::collections → rusty::collections", patch_std_collections_to_rusty),
        ("std::ptr → rusty::ptr", patch_std_ptr_to_rusty),
        ("trim CMakeLists to core 7", patch_trim_cmakelists),
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
