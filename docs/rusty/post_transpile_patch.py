#!/usr/bin/env python3
"""Post-transpile patcher for the std->`rusty` port (docs/rusty/build.sh).

Two files: out/hashbrown/hashbrown.cppm (the recursively-transpiled dep) and
out/rusty.cppm (the std slice). Rules were characterized by the std-spike
widening probe (2026-07-12) and target three roots:

hashbrown — the crate-own `mod alloc` vs extern-alloc-crate qualification
cluster (the known "stdalloc" class): the crate's OWN `alloc` submodule
shadows the extern alloc crate for Layout/alloc-fn refs.

rusty — (a) the transpiler emits NO `import hashbrown;` for a local path dep
and leaves the dep's glob re-exports unbridged (transpiler gap, bridged in
text here); (b) paths.rs maps std::hash::RandomState ->
::hashbrown::DefaultHashBuilder, wrong for the crate that DEFINES
RandomState; (c) std::hash::SipHasher -> rusty::hash::SipHasher (runtime,
include/rusty/hash.hpp); (d) TryReserveErrorKind::AllocError payload ctor ->
the runtime's (Kind, size, align) ctor.
"""
import re
import sys
from pathlib import Path


def patch_hashbrown(path: Path) -> None:
    t = path.read_text()
    t = t.replace("::alloc::alloc::Layout", "::rusty::alloc::Layout")
    t = re.sub(r"([^:])alloc::alloc::Layout", r"\1::rusty::alloc::Layout", t)
    t = t.replace(
        "export using ::alloc::do_alloc;",
        "export using ::raw::alloc::inner::do_alloc;",
    )
    t = t.replace(
        "NonNull<uint8_t>::new_(alloc(std::move(layout)))",
        "NonNull<uint8_t>::new_(rusty::alloc::alloc(std::move(layout)))",
    )
    # make_hasher/equivalent_key are declared with UNDEDUCIBLE tuple params
    # (Q,V / K,V appear only in the std::function return type); every call
    # site is bare. Re-declare as auto-returning single-param templates —
    # the returned lambdas are already generic, and RawTable takes the
    # callables generically.
    t = t.replace(
        "export template<typename Q, typename V, typename S>\n"
        "    std::function<uint64_t(const std::tuple<Q, V>&)> make_hasher(const S& hash_builder);",
        "export template<typename S>\n"
        "    auto make_hasher(const S& hash_builder);",
    )
    t = t.replace(
        "export template<typename Q, typename V, typename S>\n"
        "    std::function<uint64_t(const std::tuple<Q, V>&)> make_hasher(const S& hash_builder) {",
        "export template<typename S>\n"
        "    auto make_hasher(const S& hash_builder) {",
    )
    t = t.replace("make_hash<Q, S>(hash_builder,", "make_hash(hash_builder,")
    t = t.replace(
        "export template<typename Q, typename K, typename V>\n"
        "    std::function<bool(const std::tuple<K, V>&)> equivalent_key(const Q& k);",
        "export template<typename Q>\n"
        "    auto equivalent_key(const Q& k);",
    )
    t = t.replace(
        "export template<typename Q, typename K, typename V>\n"
        "    std::function<bool(const std::tuple<K, V>&)> equivalent_key(const Q& k) {",
        "export template<typename Q>\n"
        "    auto equivalent_key(const Q& k) {",
    )
    # `mem::replace(&mut place, v)` — the &mut place arrived as a POINTER
    # (addr_of_temp); the runtime replace takes T&.
    t = t.replace(
        "rusty::mem::replace(rusty::addr_of_temp(",
        "rusty::mem::replace(*rusty::addr_of_temp(",
    )
    # RawTable::new_/with_capacity hardcode hashbrown's own inner::Global as
    # the allocator VALUE while the generic param A may be rusty::alloc::Global
    # (the std port instantiates it so). The A param is the truth — use it.
    t = t.replace(
        "return RawTable<T, A>(rusty::clone(rusty::clone(RawTableInner::NEW)), raw::alloc::inner::Global{}, rusty::PhantomData<T>{});",
        "return RawTable<T, A>(rusty::clone(rusty::clone(RawTableInner::NEW)), A{}, rusty::PhantomData<T>{});",
    )
    t = t.replace(
        "return RawTable<T, A>::with_capacity_in(std::move(capacity), raw::alloc::inner::Global{});",
        "return RawTable<T, A>::with_capacity_in(std::move(capacity), A{});",
    )
    # do_alloc bridges Allocator::allocate's Result<NonNull<u8>, AllocError>
    # into hashbrown's Result<NonNullSlice<u8>, ()> — the emitted body skips
    # both conversions. Spell the bridge explicitly.
    lines = t.splitlines(keepends=True)
    out, i = [], 0
    sig = "rusty::Result<rusty::ptr::NonNullSlice<uint8_t>, rusty::Unit> do_alloc(const A& alloc, ::rusty::alloc::Layout layout) {"
    while i < len(lines):
        if sig in lines[i]:
            indent = re.match(r"\s*", lines[i]).group(0)
            out.append(lines[i])
            out.append(f"{indent}    auto __r = alloc.allocate(layout);\n")
            out.append(
                f"{indent}    if (__r.is_err()) {{ return rusty::Result<rusty::ptr::NonNullSlice<uint8_t>, rusty::Unit>::Err(rusty::Unit{{}}); }}\n"
            )
            out.append(
                # Rust's allocate returns NonNull<[u8]> WITH the allocated
                # length; hashbrown checks block.len() against the layout, so
                # the slice length must be real (a 0-len slice sends
                # new_uninitialized down the oversized-block path).
                f"{indent}    return rusty::Result<rusty::ptr::NonNullSlice<uint8_t>, rusty::Unit>::Ok(rusty::ptr::NonNullSlice<uint8_t>(__r.unwrap().as_ptr(), layout.size));\n"
            )
            out.append(f"{indent}}}\n")
            depth = 0
            seen = False
            j = i
            while j < len(lines):
                for ch in lines[j]:
                    if ch == "{":
                        depth += 1
                        seen = True
                    elif ch == "}":
                        depth -= 1
                j += 1
                if seen and depth == 0:
                    break
            i = j
            continue
        out.append(lines[i])
        i += 1
    t = "".join(out)
    path.write_text(t)


def patch_rusty(path: Path) -> None:
    t = path.read_text()
    # (b) the crate defines its own RandomState — undo the builtin mapping.
    t = t.replace("::hashbrown::DefaultHashBuilder", "::hash::random::RandomState")
    # TryReserveError -> the rusty runtime type (same as the alloc patcher).
    t = t.replace("std::collections::TryReserveError", "rusty::collections::TryReserveError")
    # (c) SipHasher runtime class.
    t = t.replace("std::hash::SipHasher", "rusty::hash::SipHasher")
    # DefaultHasher::write forwards `self.0.write(msg)`; the name-keyed UFCS
    # dispatch routed it to the io::Write helper (Hasher::write vs io::Write
    # collision). Call the member directly.
    t = t.replace(
        "rusty::io::write(this->_0, msg);",
        "this->_0.write(msg);",
    )
    # HashMap::new_/HashSet::new_ route through rusty::default_value, whose
    # protocol doesn't cover these types. Rust's Default here IS
    # with_hasher(RandomState::default()) — spell that directly.
    t = t.replace(
        "return rusty::default_value<HashMap<K, V, ::hash::random::RandomState>>();",
        "return HashMap<K, V, ::hash::random::RandomState>::with_hasher("
        "::hash::random::RandomState::new_());",
    )
    t = t.replace(
        "return rusty::default_value<HashSet<T, ::hash::random::RandomState>>();",
        "return HashSet<T, ::hash::random::RandomState>::with_capacity_and_hasher("
        "0, ::hash::random::RandomState::new_());",
    )
    # (d) AllocError carries a Layout payload in Rust; the runtime ctor takes
    # (Kind, size, align).
    t = t.replace(
        "rusty::from_into<rusty::collections::TryReserveError>("
        "rusty::collections::TryReserveErrorKind::AllocError(layout, std::make_tuple()))",
        "rusty::collections::TryReserveError("
        "rusty::collections::TryReserveError::Kind::AllocError, layout.size, layout.align)",
    )
    # (a) import the recursively-transpiled dep + bridge its glob re-exports
    # (they are emitted as un-exported using-directives, invisible to
    # importers — transpiler gap).
    t = t.replace(
        "export module rusty;\n",
        """export module rusty;
import hashbrown;
namespace hashbrown {
    namespace hash_map {
        using namespace ::map;
        using namespace ::rustc_entry;
    }
    namespace hash_set { using namespace ::set; }
    using ::TryReserveError;
    using ::TryReserveError_CapacityOverflow;
    using ::TryReserveError_AllocError;
}
""",
        1,
    )
    path.write_text(t)


def main(out_dir: Path) -> None:
    hb = out_dir / "hashbrown" / "hashbrown.cppm"
    ru = out_dir / "rusty.cppm"
    patch_hashbrown(hb)
    patch_rusty(ru)
    print("docs/rusty patcher: applied")


if __name__ == "__main__":
    main(Path(sys.argv[1]))
