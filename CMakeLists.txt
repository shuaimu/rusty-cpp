cmake_minimum_required(VERSION 3.28)  # C++20 modules need 3.28+

# Prefer clang/LLVM as the default toolchain. The transpiled C++20 module
# pipeline relies on clang-specific module support (BMI scanning, the
# `import std;` PCM, etc.); gcc paths are not exercised. Pick clang++ if
# the user hasn't pinned a compiler before `project()` locks one in.
if(NOT DEFINED CMAKE_CXX_COMPILER AND NOT DEFINED ENV{CXX})
    find_program(CLANGXX_EXECUTABLE
        NAMES clang++-19 clang++-18 clang++-17 clang++-16 clang++
        DOC "Clang C++ compiler")
    if(CLANGXX_EXECUTABLE)
        set(CMAKE_CXX_COMPILER "${CLANGXX_EXECUTABLE}" CACHE FILEPATH
            "C++ compiler (auto-selected clang++)")
    endif()
endif()

project(rusty_cpp_tests LANGUAGES CXX)

include(CTest)

# Global standard: strict C++23 (no GNU extensions). Most transpiled-port
# modules already required cxx_std_23 per-target via target_compile_features;
# bump the global default so plain tests pick up the same standard without
# falling back to C++20.
set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)  # -std=c++23, not -std=gnu++23

# Surface the chosen toolchain so misconfigurations are obvious.
message(STATUS "rusty-cpp toolchain: ${CMAKE_CXX_COMPILER_ID} (${CMAKE_CXX_COMPILER})")
if(NOT (CMAKE_CXX_COMPILER_ID STREQUAL "Clang"
        OR CMAKE_CXX_COMPILER_ID STREQUAL "AppleClang"))
    message(WARNING
        "rusty-cpp is developed and tested against clang/LLVM. "
        "The current compiler is ${CMAKE_CXX_COMPILER_ID}; transpiled "
        "C++20 module targets and `import std;` paths will be skipped.")
endif()

option(RUSTY_PLATFORM_POSIX "Use POSIX backend for rusty::platform primitives" OFF)
if(RUSTY_PLATFORM_POSIX)
    add_compile_definitions(RUSTY_PLATFORM_BACKEND_POSIX=1)
endif()

# ──────────────────────────────────────────────────────────────────────
# hashbrown_port — TARGET DELETED (#177, 2026-08-02). rusty::HashMap/HashSet
# come from std_port; the last consumer (vec_tests_port's ZstTracker/HashMap
# references) was migrated to std_port and its 151 tests pass. The vendored
# sources remain at transpiled/hashbrown_port/ (no target builds them) purely
# as the artifact docs/hashbrown_port/MEMORY_SAFETY_STATUS.md describes.
# Do not resurrect: its entry API does not compile and RawTable's defaulted
# copy ctor bitwise-copies an owning type (HashMap::clone UAF, never fixed).
# ──────────────────────────────────────────────────────────────────────

# ──────────────────────────────────────────────────────────────────────
# vec_port — transpiled rustc-stdlib Vec port, exposed as C++20 modules.
# Sources live in transpiled/vec_port/*.cppm and are vendored from the
# transpiler+patcher pipeline (docs/vec_port/).
#
# This is the FLAT-EXPORT vendoring (no auto-namespace wrap) — exports
# `Vec<T, A>` at global scope, mirroring the original transpile output
# the 4-way bench in docs/vec_port/ was validated against. The
# auto-namespace retranspile is a possible follow-up; see
# rusty-std-book §4 and docs/vec_port/STATUS.md.
#
# Reduced-scope build: 6 modules (vec_port + raw_vec + set_len_on_drop
# + into_iter + drain + vec). Auxiliary modules (cow, extract_if,
# in_place_*, peek_mut, splice, spec_*, partial_eq, is_zero) need their
# cluster-specific issues addressed before they can ship.
# ──────────────────────────────────────────────────────────────────────
if(CMAKE_CXX_COMPILER_ID STREQUAL "Clang"
   OR CMAKE_CXX_COMPILER_ID STREQUAL "AppleClang")
    # vec_port.vec.drain dropped from the build during the VecLegacy retirement —
    # the drain submodule cyclically depends on ::Vec while ::Vec's drain()
    # method requires Drain to be visible. Original transpilation bridged this
    # via header-attached rusty::VecLegacy which is going away. drain() can be
    # re-vendored later as a header-only helper or by restructuring the modules.
    set(VEC_PORT_CPPM
        ${CMAKE_CURRENT_SOURCE_DIR}/transpiled/vec_port/vec_port.cppm
        ${CMAKE_CURRENT_SOURCE_DIR}/transpiled/vec_port/vec_port.raw_vec.cppm
        ${CMAKE_CURRENT_SOURCE_DIR}/transpiled/vec_port/vec_port.vec.set_len_on_drop.cppm
        ${CMAKE_CURRENT_SOURCE_DIR}/transpiled/vec_port/vec_port.vec.into_iter.cppm
        ${CMAKE_CURRENT_SOURCE_DIR}/transpiled/vec_port/vec_port.vec.cppm
    )
    add_library(vec_port STATIC)
    target_sources(vec_port
        PUBLIC FILE_SET CXX_MODULES FILES ${VEC_PORT_CPPM}
    )
    target_include_directories(vec_port PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
    target_compile_features(vec_port PUBLIC cxx_std_23)
    target_compile_options(vec_port PRIVATE -O3 -DNDEBUG -march=native )
endif()

# ──────────────────────────────────────────────────────────────────────
# btree_port — transpiled rustc-stdlib BTreeMap port, exposed as C++20
# modules. Sources live in transpiled/btree_port/*.cppm and are vendored
# from the transpiler+patcher pipeline (docs/btreemap_port/).
#
# Per docs/btreemap_port/STATUS.md, btree_internal compiles cleanly under
# both g++ and clang++; map.cppm needs clang++ (gcc 14 ICEs on the
# destructor analysis of rusty::RcControlBlockBase). The library is
# clang-only for that reason.
#
# This is a STAGED vendoring: the library builds and links, but
# rusty::BTreeMap continues to be the std::map-backed facade in
# include/rusty/btreemap.hpp. Adopting the transpiled BTreeMap as the
# rusty::BTreeMap backing (analogous to the hashbrown_port→rusty::HashMap
# move) requires a separate facade-rewrite step.
# ──────────────────────────────────────────────────────────────────────
if(CMAKE_CXX_COMPILER_ID STREQUAL "Clang"
   OR CMAKE_CXX_COMPILER_ID STREQUAL "AppleClang")
    # The patcher pipeline produces 4 vendored .cppm files under the
    # `namespace btree_port::btree::X { … }` wrap (consistent
    # `--auto-namespace` retranspile of all three modules, STATUS Step 88).
    # All four ship in the build target. `map.entry` is merged into
    # `map.cppm` by patcher Step 52 — no separate map.entry.cppm.
    set(BTREE_PORT_CPPM
        ${CMAKE_CURRENT_SOURCE_DIR}/transpiled/btree_port/btree_port.btree.btree_internal.cppm
        ${CMAKE_CURRENT_SOURCE_DIR}/transpiled/btree_port/btree_port.btree.map.cppm
        ${CMAKE_CURRENT_SOURCE_DIR}/transpiled/btree_port/btree_port.btree.set.entry.cppm
        ${CMAKE_CURRENT_SOURCE_DIR}/transpiled/btree_port/btree_port.btree.set.cppm
    )
    add_library(btree_port STATIC)
    target_sources(btree_port
        PUBLIC FILE_SET CXX_MODULES FILES ${BTREE_PORT_CPPM}
    )
    target_include_directories(btree_port PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
    target_compile_features(btree_port PUBLIC cxx_std_23)
    target_compile_options(btree_port PRIVATE -O3 -DNDEBUG -march=native )
endif()

# ──────────────────────────────────────────────────────────────────────
# binary_heap_port — transpiled rustc BinaryHeap (priority queue),
# exposed as a C++20 module. Single .cppm vendored from the
# transpiler+patcher pipeline (docs/binary_heap_port/).
# ──────────────────────────────────────────────────────────────────────
if(CMAKE_CXX_COMPILER_ID STREQUAL "Clang"
   OR CMAKE_CXX_COMPILER_ID STREQUAL "AppleClang")
    file(GLOB BINARY_HEAP_PORT_CPPM transpiled/binary_heap_port/*.cppm)
    add_library(binary_heap_port STATIC)
    target_sources(binary_heap_port
        PUBLIC FILE_SET CXX_MODULES FILES ${BINARY_HEAP_PORT_CPPM}
    )
    target_include_directories(binary_heap_port PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
    target_compile_features(binary_heap_port PUBLIC cxx_std_23)
    target_compile_options(binary_heap_port PRIVATE -O3 -DNDEBUG -march=native )
    target_link_libraries(binary_heap_port PUBLIC vec_port)
endif()

# ──────────────────────────────────────────────────────────────────────
# binary_heap_tests_port — transpiled rustc binary_heap.rs test file
# (library/alloctests/tests/collections/binary_heap.rs, 34 #[test]
# functions). Imports binary_heap_port; tests are auto-registered via
# rusty/test_runner.hpp at static-init time. Driver in
# tests/binary_heap_tests_port_module_test.cpp.
# ──────────────────────────────────────────────────────────────────────
if(CMAKE_CXX_COMPILER_ID STREQUAL "Clang"
   OR CMAKE_CXX_COMPILER_ID STREQUAL "AppleClang")
    file(GLOB BINARY_HEAP_TESTS_PORT_CPPM transpiled/binary_heap_tests_port/*.cppm)
    add_library(binary_heap_tests_port STATIC)
    target_sources(binary_heap_tests_port
        PUBLIC FILE_SET CXX_MODULES FILES ${BINARY_HEAP_TESTS_PORT_CPPM}
    )
    target_include_directories(binary_heap_tests_port PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
    target_compile_features(binary_heap_tests_port PUBLIC cxx_std_23)
    # Match binary_heap_port's release flags so the BMI is target-compatible.
    target_compile_options(binary_heap_tests_port PRIVATE -O3 -DNDEBUG -march=native)
    target_link_libraries(binary_heap_tests_port PUBLIC binary_heap_port vec_port)
endif()

# ──────────────────────────────────────────────────────────────────────
# linked_list_tests_port — transpiled rustc library/alloctests/tests/
# linked_list.rs (1 #[test], stubbed as skip — needs std::hash builder).
# ──────────────────────────────────────────────────────────────────────
if(CMAKE_CXX_COMPILER_ID STREQUAL "Clang"
   OR CMAKE_CXX_COMPILER_ID STREQUAL "AppleClang")
    file(GLOB LINKED_LIST_TESTS_PORT_CPPM transpiled/linked_list_tests_port/*.cppm)
    add_library(linked_list_tests_port STATIC)
    target_sources(linked_list_tests_port
        PUBLIC FILE_SET CXX_MODULES FILES ${LINKED_LIST_TESTS_PORT_CPPM}
    )
    target_include_directories(linked_list_tests_port PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
    target_compile_features(linked_list_tests_port PUBLIC cxx_std_23)
    target_compile_options(linked_list_tests_port PRIVATE -O3 -DNDEBUG -march=native)
    target_link_libraries(linked_list_tests_port PUBLIC linked_list_port)
endif()

# ──────────────────────────────────────────────────────────────────────
# eq_diff_len_tests_port — transpiled rustc tests/collections/eq_diff_len.rs
# (7 #[test], all stubbed — needs custom-type `==` plumbing + should_panic).
# ──────────────────────────────────────────────────────────────────────
if(CMAKE_CXX_COMPILER_ID STREQUAL "Clang"
   OR CMAKE_CXX_COMPILER_ID STREQUAL "AppleClang")
    file(GLOB EQ_DIFF_LEN_TESTS_PORT_CPPM transpiled/eq_diff_len_tests_port/*.cppm)
    add_library(eq_diff_len_tests_port STATIC)
    target_sources(eq_diff_len_tests_port
        PUBLIC FILE_SET CXX_MODULES FILES ${EQ_DIFF_LEN_TESTS_PORT_CPPM}
    )
    target_include_directories(eq_diff_len_tests_port PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
    target_compile_features(eq_diff_len_tests_port PUBLIC cxx_std_23)
    target_compile_options(eq_diff_len_tests_port PRIVATE -O3 -DNDEBUG -march=native)
endif()

# ──────────────────────────────────────────────────────────────────────
# alloc — the WHOLE of Rust's alloc crate as one transpiled module:
# raw_vec, boxed, string, borrow, rc, sync (Arc), collections
# (vec_deque, binary_heap, linked_list, btree_map/btree_set) and vec.
# Vendored (see transpiled/alloc/REGENERATE.md) so this build never needs
# a rustc source tree or cargo-expand. The vec/vec_deque suites below bind
# it, and are its regression gate.
# ──────────────────────────────────────────────────────────────────────
if(CMAKE_CXX_COMPILER_ID STREQUAL "Clang"
   OR CMAKE_CXX_COMPILER_ID STREQUAL "AppleClang")
    add_library(alloc STATIC)
    target_sources(alloc
        PUBLIC FILE_SET CXX_MODULES FILES
        ${CMAKE_CURRENT_SOURCE_DIR}/transpiled/alloc/alloc.cppm
    )
    target_include_directories(alloc PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
    target_compile_features(alloc PUBLIC cxx_std_23)
    # ⚠️ -O0 IS LOAD-BEARING. Producing this module's BMI at any optimization
    # level >= 1 yields a BMI that SEGFAULTS clang 22.1.8 on import, inside
    # ASTDeclReader::VisitFunctionDecl -> RedeclarableTemplateDecl::
    # getCommonPtr() (while loading a decl-update record). Bisected across
    # -O0/-O1/-O2/-O3: -O0 imports fine, all others crash. The poison also
    # PROPAGATES — a suite BMI built against a poisoned alloc BMI is itself
    # poisoned. docs/alloc/build.sh gets this right by accident (it passes no
    # -O when precompiling), which is why the out-of-tree flow always worked.
    # CMake emits the BMI and the object in one command, so the object is
    # unoptimized too; that only costs test runtime.
    target_compile_options(alloc PRIVATE
        -O0 -march=native -UNDEBUG -DRUSTY_PORTABLE_INTRINSICS=1)
    target_link_libraries(alloc PUBLIC rusty)
endif()

# ──────────────────────────────────────────────────────────────────────
# std_port — the transpiled Rust **std** collections::hash slice
# (HashMap/HashSet over std's RandomState) and the recursively transpiled
# hashbrown 0.16.1 it sits on. This is the layering the 2026-07-27 directive
# asks for: translate from std, reach hashbrown only as std's dependency.
# Vendored; see transpiled/std_port/REGENERATE.md.
#
# Two modules, so two targets: `hashbrown` is a separate C++20 module that
# std_port imports. Each is wrapped in its own namespace
# (`std_port::…`, `hashbrown::…`) by --crate-namespace-wrap, so neither
# pollutes the global namespace when the umbrella re-exports them.
# ──────────────────────────────────────────────────────────────────────
if(CMAKE_CXX_COMPILER_ID STREQUAL "Clang"
   OR CMAKE_CXX_COMPILER_ID STREQUAL "AppleClang")
    add_library(std_port_hashbrown STATIC)
    target_sources(std_port_hashbrown
        PUBLIC FILE_SET CXX_MODULES FILES
        ${CMAKE_CURRENT_SOURCE_DIR}/transpiled/std_port/hashbrown/hashbrown.cppm
    )
    target_include_directories(std_port_hashbrown PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
    target_compile_features(std_port_hashbrown PUBLIC cxx_std_23)
    # NOTE (#183): these flags MUST match every other port target and the
    # `rusty` umbrella exactly. A flag delta here makes CMake synthesise a
    # SECOND BMI for this module (CMakeFiles/std_port@synth_N.dir/*.bmi), so
    # `rusty.pcm` ends up built against one std_port BMI while consumers
    # resolve `std_port` to a different one. Clang then merges two full copies
    # of the same module and either hangs in ASTReader or SIGSEGVs in codegen.
    # (The old -DRUSTY_PORTABLE_INTRINSICS=1 was a no-op here: it is only read
    # by rusty/sync/mpsc_lockfree.hpp, which std_port never includes.)
    target_compile_options(std_port_hashbrown PRIVATE -O3 -DNDEBUG -march=native)
    # Deliberately NOT linked against the `rusty` target: these modules only
    # #include <rusty/*.hpp>, they never `import rusty`. Linking it would make
    # a target cycle once the umbrella links std_port to re-export HashMap.

    add_library(std_port STATIC)
    target_sources(std_port
        PUBLIC FILE_SET CXX_MODULES FILES
        ${CMAKE_CURRENT_SOURCE_DIR}/transpiled/std_port/std_port.cppm
    )
    target_include_directories(std_port PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
    target_compile_features(std_port PUBLIC cxx_std_23)
    # Same flags as std_port_hashbrown and every other port target — see the
    # #183 note above; a delta here forks the BMI and breaks `import rusty;`.
    target_compile_options(std_port PRIVATE -O3 -DNDEBUG -march=native)
    target_link_libraries(std_port PUBLIC std_port_hashbrown)
endif()

# ──────────────────────────────────────────────────────────────────────
# Test-port modules for the rustc collection tests. Some are real
# translations; the rest are hand-written SKIP stubs (generated by
# docs/_gen_test_stub.py from the Rust source's #[test] annotations) whose
# pipeline directory (docs/<name>_tests_port/) is kept so the transpile +
# patch flow can be re-run once the prerequisites land.
# ──────────────────────────────────────────────────────────────────────
if(CMAKE_CXX_COMPILER_ID STREQUAL "Clang"
   OR CMAKE_CXX_COMPILER_ID STREQUAL "AppleClang")
    # Real translations, bound to the consolidated `alloc` module.
    set(_alloc_backed_suites vec_tests_port vec_deque_tests_port)
    foreach(_port
        vec_deque_tests_port
        vec_tests_port
        arc_tests_port
        rc_tests_port
        string_tests_port
        btree_set_hash_tests_port)
        file(GLOB _CPPM transpiled/${_port}/*.cppm)
        add_library(${_port} STATIC)
        target_sources(${_port}
            PUBLIC FILE_SET CXX_MODULES FILES ${_CPPM}
        )
        target_include_directories(${_port} PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
        target_compile_features(${_port} PUBLIC cxx_std_23)
        if(_port IN_LIST _alloc_backed_suites)
            # Real translations of rustc's vec.rs (151 tests) / vec_deque.rs
            # (104), run against alloc's Vec/VecDeque. -O1 keeps these two
            # ~340 KB modules' build time sane. Their assertions lower to
            # rusty::panic::do_panic, not C assert, so NDEBUG cannot vacuum
            # them out — but keep -UNDEBUG anyway for any assert() the
            # transpiled bodies still carry.
            target_compile_options(${_port} PRIVATE
                -O1 -march=native -UNDEBUG -DRUSTY_PORTABLE_INTRINSICS=1)
            target_link_libraries(${_port} PUBLIC rusty alloc vec_port)
        elseif(_port STREQUAL "btree_set_hash_tests_port" OR _port STREQUAL "arc_tests_port"
               OR _port STREQUAL "string_tests_port" OR _port STREQUAL "rc_tests_port")
            # Real translations (asserts live IN the module) — no NDEBUG,
            # or the whole suite passes vacuously.
            target_compile_options(${_port} PRIVATE -O3 -march=native -UNDEBUG)
            target_link_libraries(${_port} PUBLIC rusty)
        else()
            target_compile_options(${_port} PRIVATE -O3 -DNDEBUG -march=native)
        endif()
    endforeach()
    # btree_tests_port (the SKIP-stub module) was retired 2026-07: the
    # btree suite is fully translated in tests/btree_tests_port_unstubbed.cpp.
endif()

# ──────────────────────────────────────────────────────────────────────
# string_port — the last bridge module. The full transpiled body
# (string_port.cppm, 5983 LOC) builds against core::str::Pattern /
# core::str::Searcher / alloc::borrow::Cow / core::ascii::Char which
# aren't yet vendored; until those land, we build the 27-line
# `string_port_stub.cppm` that re-exports the hand-written
# `rusty::String` under `string_port::String`. The other "bridge"
# ports (rc, arc, vec_deque, linked_list, cell) all moved past the
# bridge — they build from their full transpiled .cppm directly.
# ──────────────────────────────────────────────────────────────────────
if(CMAKE_CXX_COMPILER_ID STREQUAL "Clang"
   OR CMAKE_CXX_COMPILER_ID STREQUAL "AppleClang")
    foreach(_port string_port)
        add_library(${_port} STATIC)
        target_sources(${_port}
            PUBLIC FILE_SET CXX_MODULES FILES
            ${CMAKE_CURRENT_SOURCE_DIR}/transpiled/${_port}/${_port}_stub.cppm
        )
        target_include_directories(${_port} PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
        target_compile_features(${_port} PUBLIC cxx_std_23)
        target_compile_options(${_port} PRIVATE -O3 -DNDEBUG -march=native )
    endforeach()

    # rc_port: full transpiled body from library/alloc/src/rc.rs
    # (4565 LOC Rust → ~5100 LOC C++). Pipeline:
    #   docs/rc_port/{prep.sh, post_transpile_patch.py} → transpiled/rc_port/rc_port.cppm
    add_library(rc_port STATIC)
    target_sources(rc_port
        PUBLIC FILE_SET CXX_MODULES FILES
        ${CMAKE_CURRENT_SOURCE_DIR}/transpiled/rc_port/rc_port.cppm
    )
    target_include_directories(rc_port PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
    target_compile_features(rc_port PUBLIC cxx_std_23)
    target_compile_options(rc_port PRIVATE -O3 -DNDEBUG -march=native )
    target_link_libraries(rc_port PUBLIC vec_port)

    # arc_port: full transpiled body from library/alloc/src/sync.rs.
    # Pipeline mirrors rc_port — see docs/arc_port/{prep.sh,
    # post_transpile_patch.py}.
    add_library(arc_port STATIC)
    target_sources(arc_port
        PUBLIC FILE_SET CXX_MODULES FILES
        ${CMAKE_CURRENT_SOURCE_DIR}/transpiled/arc_port/arc_port.cppm
    )
    target_include_directories(arc_port PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
    target_compile_features(arc_port PUBLIC cxx_std_23)
    target_compile_options(arc_port PRIVATE -O3 -DNDEBUG -march=native )
    target_link_libraries(arc_port PUBLIC vec_port)

    # linked_list_port: full transpiled body (the transpiler fix landed
    # for the Cluster A 'Box<auto>::new_in' regression — see
    # transpiler/src/codegen.rs test_linked_list_box_new_in_with_typed_arg_*).
    add_library(linked_list_port STATIC)
    target_sources(linked_list_port
        PUBLIC FILE_SET CXX_MODULES FILES
        ${CMAKE_CURRENT_SOURCE_DIR}/transpiled/linked_list_port/linked_list_port.cppm
    )
    target_include_directories(linked_list_port PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
    target_compile_features(linked_list_port PUBLIC cxx_std_23)
    target_compile_options(linked_list_port PRIVATE -O3 -DNDEBUG -march=native )
    target_link_libraries(linked_list_port PUBLIC vec_port)

    # vec_deque_port: full transpiled body (collapse-Rust-source
    # approach — see docs/vec_deque_port/collapse.py).
    add_library(vec_deque_port STATIC)
    target_sources(vec_deque_port
        PUBLIC FILE_SET CXX_MODULES FILES
        ${CMAKE_CURRENT_SOURCE_DIR}/transpiled/vec_deque_port/vec_deque_port.cppm
    )
    target_include_directories(vec_deque_port PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
    target_compile_features(vec_deque_port PUBLIC cxx_std_23)
    target_compile_options(vec_deque_port PRIVATE -O3 -DNDEBUG -march=native )
    target_link_libraries(vec_deque_port PUBLIC vec_port)

    # borrow_port: transpiled alloc::borrow (Cow<'a, B> + ToOwned).
    # Phase 3a/3b transpiler trait machinery unblocked the assoc-type
    # emission; remaining patches in docs/borrow_port/post_transpile_patch.py
    # handle visit_byte_buf stub, ::rusty_ext qualification, and
    # orphan-impl `#if 0` block (all sibling-port patterns).
    add_library(borrow_port STATIC)
    target_sources(borrow_port
        PUBLIC FILE_SET CXX_MODULES FILES
        ${CMAKE_CURRENT_SOURCE_DIR}/transpiled/borrow_port/borrow_port.cppm
    )
    target_include_directories(borrow_port PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
    target_compile_features(borrow_port PUBLIC cxx_std_23)
    target_compile_options(borrow_port PRIVATE -O3 -DNDEBUG -march=native )
    target_link_libraries(borrow_port PUBLIC vec_port)

    # ascii_port: transpiled core::ascii (AsciiChar enum + impl block).
    # Phase A1 transpile clean (4151 LOC); Phase A2 patches in
    # docs/ascii_port/post_transpile_patch.py — TBD as compile errors
    # surface.
    add_library(ascii_port STATIC)
    target_sources(ascii_port
        PUBLIC FILE_SET CXX_MODULES FILES
        ${CMAKE_CURRENT_SOURCE_DIR}/transpiled/ascii_port/ascii_port.cppm
    )
    target_include_directories(ascii_port PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
    target_compile_features(ascii_port PUBLIC cxx_std_23)
    target_compile_options(ascii_port PRIVATE -O3 -DNDEBUG -march=native )

    # core_slice_port: transpiled library/core/src/slice/* (collapsed via
    # docs/core_slice_port/prep.sh + collapse.py). Phase A1 transpile
    # clean (8773 LOC C++); Phase A2 patches TBD as compile errors surface.
    if(RUSTY_CPP_BUILD_CORE_SLICE_PORT)
        add_library(core_slice_port STATIC)
        target_sources(core_slice_port
            PUBLIC FILE_SET CXX_MODULES FILES
            ${CMAKE_CURRENT_SOURCE_DIR}/transpiled/core_slice_port/core_slice_port.cppm
        )
        target_include_directories(core_slice_port PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
        target_compile_features(core_slice_port PUBLIC cxx_std_23)
        target_compile_options(core_slice_port PRIVATE -O3 -DNDEBUG -march=native )
    endif()
endif()

# ──────────────────────────────────────────────────────────────────────
# cell_port — transpiled rustc Cell / RefCell port (library/core/src/cell.rs).
# Single .cppm vendored from the transpiler+patcher pipeline
# (docs/cell_port/). Phase A2 patches: namespace `using ::ns::` →
# `using rusty::ns::` rewrites, plus Rust trait stubs added to
# include/rusty/{ops,marker,fmt}.hpp to unblock the `using
# rusty::ops::Deref;` etc. imports.
# ──────────────────────────────────────────────────────────────────────
if(CMAKE_CXX_COMPILER_ID STREQUAL "Clang"
   OR CMAKE_CXX_COMPILER_ID STREQUAL "AppleClang")
    file(GLOB CELL_PORT_CPPM transpiled/cell_port/*.cppm)
    add_library(cell_port STATIC)
    target_sources(cell_port
        PUBLIC FILE_SET CXX_MODULES FILES ${CELL_PORT_CPPM}
    )
    target_include_directories(cell_port PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
    target_compile_features(cell_port PUBLIC cxx_std_23)
    target_compile_options(cell_port PRIVATE -O3 -DNDEBUG -march=native )
endif()

# ──────────────────────────────────────────────────────────────────────
# vec_deque_port — full transpiled rustc VecDeque (ring buffer) attempt
# reverted to bridge-stub for now. The body needs C++20 module-ownership
# work: submodules reference vec_deque_port::VecDeque<T,A> but importing
# the main module from submodules creates a cycle, and forward-declaring
# in submodules fails C++20's "declaration attached to other modules"
# rule. Either the type needs to live at global namespace (like vec_port's
# ::Vec) or all code needs to merge into a single module unit.
#
# Patcher in docs/vec_deque_port/post_transpile_patch.py holds the
# ~15 rewrites we've accumulated — keep it ready for the next attempt.
# Infrastructure changes already landed:
#   - include/rusty/vecdeque.hpp emptied (stub)
#   - include/rusty/rusty.cppm: `using VecDeque = vec_deque_port::VecDeque<…>;`
#   - include/rusty/mem.hpp: `mem::needs_drop<T>()` template
#   - transpiled/vec_port/vec_port.vec.into_iter.cppm: stale
#     `using rusty::VecDeque;` commented out
# These don't break anything and prep for the eventual promotion.
# ──────────────────────────────────────────────────────────────────────

# ──────────────────────────────────────────────────────────────────────
# linked_list_port — transpiled rustc LinkedList (doubly-linked w/cursor).
# Single .cppm vendored. Net-new functionality — no hand-written
# rusty::LinkedList existed. CMake target intentionally not wired yet —
# the .cppm builds past Phase A2 patches but hits 13 "auto not allowed
# in template argument" emit-bug errors (same shape as BTreeMap port's
# Cluster A, which required a transpiler-side fix in commit 7311d18).
# To wire: uncomment when those clusters land via the transpiler patch
# or hand-port catalogue. See docs/linked_list_port/STATUS.md.
#
# if(CMAKE_CXX_COMPILER_ID STREQUAL "Clang"
#    OR CMAKE_CXX_COMPILER_ID STREQUAL "AppleClang")
#     file(GLOB LINKED_LIST_PORT_CPPM transpiled/linked_list_port/*.cppm)
#     add_library(linked_list_port STATIC)
#     target_sources(linked_list_port
#         PUBLIC FILE_SET CXX_MODULES FILES ${LINKED_LIST_PORT_CPPM}
#     )
#     target_include_directories(linked_list_port PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
#     target_compile_features(linked_list_port PUBLIC cxx_std_23)
#     target_compile_options(linked_list_port PRIVATE -O3 -DNDEBUG -march=native )
#     target_link_libraries(linked_list_port PUBLIC vec_port)
# endif()

# ──────────────────────────────────────────────────────────────────────
# rusty_async — first hand-written rusty header converted to a C++20
# module (was include/rusty/async.hpp, now include/rusty/async.cppm).
# Migration goal: replace the legacy VecLegacy-backed Executor task store
# with vec_port::Vec. Module units can `import` modules; headers can't —
# hence the conversion.
#
# Clang-only because it transitively imports vec_port.vec (which is
# clang-only — gcc 14 ICEs on parts of the transpiled output).
# ──────────────────────────────────────────────────────────────────────
if(CMAKE_CXX_COMPILER_ID STREQUAL "Clang"
   OR CMAKE_CXX_COMPILER_ID STREQUAL "AppleClang")
    add_library(rusty_async STATIC)
    target_sources(rusty_async
        PUBLIC FILE_SET CXX_MODULES FILES
        ${CMAKE_CURRENT_SOURCE_DIR}/include/rusty/async.cppm
    )
    target_include_directories(rusty_async PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
    target_compile_features(rusty_async PUBLIC cxx_std_23)
    target_compile_options(rusty_async PRIVATE -O3 -DNDEBUG -march=native )
    target_link_libraries(rusty_async PUBLIC vec_port)
endif()

# ──────────────────────────────────────────────────────────────────────
# rusty — umbrella C++20 module (include/rusty/rusty.cppm).
# Re-exports the migrated transpiled ports so consumers can write
# `import rusty;` and pick up `rusty::Vec`, `rusty::Rc`,
# `rusty::BTreeMap`, etc. without naming each port module directly.
# Mirrors how Rust's prelude auto-imports `std::*` types.
# ──────────────────────────────────────────────────────────────────────
if(CMAKE_CXX_COMPILER_ID STREQUAL "Clang"
   OR CMAKE_CXX_COMPILER_ID STREQUAL "AppleClang")
    add_library(rusty STATIC)
    target_sources(rusty
        PUBLIC FILE_SET CXX_MODULES FILES
        ${CMAKE_CURRENT_SOURCE_DIR}/include/rusty/rusty.cppm
    )
    target_include_directories(rusty PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
    target_compile_features(rusty PUBLIC cxx_std_23)
    target_compile_options(rusty PRIVATE -O3 -DNDEBUG -march=native )
    target_link_libraries(rusty PUBLIC
        rusty_async
        std_port
        vec_port
        btree_port
        rc_port
        binary_heap_port
        vec_deque_port
        linked_list_port
        cell_port
        string_port
        arc_port
    )
endif()

set(TEST_SOURCES
    # Basic Rusty types - ALL WORKING
    tests/rusty_arc_test.cpp
    tests/rusty_arc_mt_test.cpp
    tests/rusty_box_test.cpp
    tests/rusty_owned_fd_test.cpp
    tests/rusty_net_tcp_test.cpp
    tests/rusty_cell_test.cpp
    tests/rusty_option_test.cpp
    # rusty_rc_test.cpp deleted — legacy rusty::Rc retired; rc_port::Rc
    # exercised via tests/rc_port_module_test.cpp instead.
    tests/rusty_refcell_test.cpp
    tests/rusty_result_test.cpp
    tests/rusty_mem_test.cpp
    tests/rusty_vec_test.cpp
    tests/rusty_function_test.cpp
    tests/rusty_array_test.cpp
    tests/rusty_error_test.cpp
    tests/rusty_arch_test.cpp
    # Integer/range intrinsics (NonZero bit-ops, range::step_by) — hashbrown.
    tests/rusty_intrinsics_test.cpp
    # Self-contained (no-std) rusty::fmt runtime: coverage + Rust-parity.
    tests/rusty_fmt_test.cpp
    tests/rusty_fmt_parity_test.cpp
    # rusty iterator adapters vs Rust's own (oracle): tests/iter_parity/.
    tests/rusty_iter_parity_test.cpp
    # for_in must borrow lvalue containers (consuming into_iter is
    # rvalue-only; lvalue consumption gutted live containers).
    tests/rusty_for_in_lvalue_borrow_test.cpp
    # Range contains must not narrow the queried item (literal-bound
    # ranges vs wider items — silent wrong answers).
    tests/rusty_range_contains_width_test.cpp
    tests/test_traits.cpp
    tests/test_mutex.cpp
    tests/test_thread.cpp
    tests/test_channel.cpp
    tests/rusty_condvar_test.cpp

    # Universal auto-deref dispatcher (rusty::deref_call).
    tests/dispatch_test.cpp

    # Annotation system tests - WORKING (fixed missing includes)
    tests/test_external_annotations.cpp
    tests/test_simplified_external.cpp
    tests/test_stl_lifetimes.cpp
    tests/test_unified_annotations.cpp

    # rusty::BTreeMap dropped in favor of the transpiled
    # btree_port.btree.map module. Legacy test removed.
    # tests/rusty_btreemap_test.cpp  (deleted with the legacy header)
    # tests/btree_port_facade_test.cpp  (tested the deleted std::map facade)

    # tests/hashbrown_port_{map,set}_test.cpp — deleted with the
    # hashbrown_port target (#177); std_port's suites cover the surface.
    # Was orphaned: present in tests/ but absent from TEST_SOURCES, so the
    # regression guard for bug #161 (HashSet set-algebra members) had never
    # actually run. Its count/range-for idiom is also what exposed the
    # api_coverage probes measuring the wrong thing.
    tests/hashset_set_algebra_test.cpp
    # rusty::HashMap through the `import rusty;` UMBRELLA. Until this landed the
    # line above was the only umbrella-importing test and it covers HashSet
    # only, so the rusty::HashMap alias — the one #183 (forked BMI) and #185
    # (trigger pointed at the retired hashbrown port) both broke — had no
    # coverage at all. The six std_port_*.cpp suites cannot substitute: they
    # `import std_port;` directly and bypass the alias.
    tests/rusty_umbrella_hashmap_test.cpp
    # Same set-algebra surface, but reached via `import std_port;` directly
    # instead of the `rusty` umbrella. Post-retarget the umbrella TU above
    # costs >10 min to compile (was ~3.7s pre-retarget), so the FUNCTIONAL
    # coverage — range-for over all four lazy views — lives here where the TU
    # compiles in ~2s. Keep both: this one tests behaviour, that one tests
    # that the rusty::HashSet alias is still reachable through the umbrella.
    tests/std_port_set_algebra_test.cpp
    # Drop-accounting / ownership tests for std_port's HashMap, translated from
    # rustc's map/tests.rs. The shipped rusty::HashMap path had no runtime test
    # at all before this (see task #184).
    tests/std_port_map_drop_test.cpp
    # Behavioural (value-semantics) coverage for std_port's HashMap — the
    # companion to the drop/ownership file above. Both #184.
    tests/std_port_map_api_test.cpp
    # HashSet behavioural coverage — the surface the set-algebra file above
    # does not touch (membership, take/replace/get, extend, clear). #184.
    tests/std_port_set_api_test.cpp
    # Entry-API and drain coverage — the highest-risk untested surfaces
    # (Entry holds a reference into the table; drain mutates while iterating). #184.
    tests/std_port_map_entry_test.cpp
    # Mutation-through-iterator (values_mut/iter_mut) — the class that was
    # already proven broken in retain/ExtractIf by #179. #184.
    tests/std_port_map_mut_iter_test.cpp

    # btree_port — transpiled rustc-stdlib BTreeMap / BTreeSet exposed
    # as C++20 modules. Smoke tests import btree_port.btree.{map,set}
    # and exercise the public APIs. The vendored library is clang-only
    # (see the btree_port library target above) so the tests are too;
    # the loop in the test target section appends them conditionally.
    tests/btree_port_module_test.cpp
    tests/btree_port_set_module_test.cpp

    # vec_port — transpiled rustc-stdlib Vec exposed as C++20 modules.
    # Smoke test imports vec_port.vec and exercises new_/push/pop/len.
    tests/vec_port_module_test.cpp

    # rusty.async — first hand-written rusty header converted to a C++20
    # module (replaces VecLegacy<Task<void>> with vec_port::Vec under the
    # Executor). Smoke test imports rusty.async, spawns 3 Task<void>s,
    # and verifies they all run.
    tests/async_module_test.cpp

    # binary_heap_port — transpiled rustc BinaryHeap (max-heap priority
    # queue). Smoke test imports binary_heap_port, pushes scrambled ints,
    # asserts they come out in descending order.
    tests/binary_heap_port_module_test.cpp

    # binary_heap_tests_port — driver for the transpiled rustc
    # binary_heap.rs test file (34 test cases). Auto-registers each
    # TEST_CASE at static-init via rusty/test_runner.hpp.
    tests/binary_heap_tests_port_module_test.cpp

    # linked_list_tests_port — driver for the transpiled rustc
    # linked_list.rs test file (1 test, stubbed as skip).
    tests/linked_list_tests_port_module_test.cpp

    # eq_diff_len_tests_port — driver for collections/eq_diff_len.rs.
    tests/eq_diff_len_tests_port_module_test.cpp

    # vec_deque_tests_port — driver for tests/vec_deque.rs: REAL translation,
    # 104 tests (90 live + 14 cited skips) against alloc's VecDeque.
    tests/vec_deque_tests_port_module_test.cpp
    # vec_tests_port — driver for tests/vec.rs: REAL translation,
    # 151 tests (126 live + 25 cited skips) against alloc's Vec.
    tests/vec_tests_port_module_test.cpp
    # arc_tests_port — driver for tests/arc.rs (14 tests, stub).
    tests/arc_tests_port_module_test.cpp
    # rc_tests_port — driver for tests/rc.rs (61 tests, REAL
    # translation: 30 live + 31 cited skips).
    tests/rc_tests_port_module_test.cpp
    # string_tests_port — driver for tests/string.rs (60 tests, REAL
    # translation: 52 live + 8 cited skips).
    tests/string_tests_port_module_test.cpp
    # btree_set_hash_tests_port — driver for tests/btree_set_hash.rs (2 tests, stub).
    tests/btree_set_hash_tests_port_module_test.cpp
    # btree_tests_port_module_test — driver for btree/{map,set}/tests.rs.
    # Fully translated bodies (439 tests, live asserts) live in
    # tests/btree_tests_port_unstubbed.cpp; the SKIP-stub module retired 2026-07.
    tests/btree_tests_port_module_test.cpp

    # binary_heap_port Phase C: push() drive test, iterates on the 6
    # instantiation clusters documented in docs/binary_heap_port/STATUS.md.
    tests/binary_heap_port_push_test.cpp

    # binary_heap_port Phase C: pop() exercise — sift-down path
    # (hits clusters C4/C5/C6).
    tests/binary_heap_port_pop_test.cpp

    # binary_heap_port comprehensive: peek + full drain ordering +
    # clear + push-after-pop preserves heap.
    tests/binary_heap_port_comprehensive_test.cpp

    # binary_heap_port iter + bulk: iter, into_iter_sorted, as_slice,
    # into_sorted_vec, append. Hits the iterator and rebuild paths
    # the other tests don't touch (Cluster C6 in STATUS.md).
    tests/binary_heap_port_iter_test.cpp

    # binary_heap_port advanced: consume / bulk-build / mutation surface —
    # with_capacity_in, from(Vec), into_sorted_vec, into_vec, drain,
    # drain_sorted, append, retain. Hits the remaining API surface.
    tests/binary_heap_port_advanced_test.cpp

    # binary_heap_port full-API: every remaining public method —
    # new_, default_, with_capacity, from(array), from_iter, from_raw_vec,
    # peek_mut, pop_if, extend, extend_one, into_iter (unsorted),
    # reserve*, try_reserve*, shrink_to_fit, shrink_to, clone, clone_from,
    # allocator.
    tests/binary_heap_port_full_api_test.cpp

    # binary_heap_port micro-bench: transpiled BinaryHeap vs
    # std::priority_queue across PUSH / POP / MIX. Built as a test so
    # CMake handles module-aware linkage; prints to stdout — invoke
    # directly (`./build/bench.out`), not via ctest.
    docs/binary_heap_port/binary_heap_port_bench.cpp

    # cell_port — transpiled rustc Cell / RefCell (library/core/src/cell.rs).
    # Phase B/C smoke test: import cell_port, build BorrowError /
    # BorrowMutError + format them via operator<<.
    tests/cell_port_module_test.cpp

    # rc_port — Phase B/C bridge (stub module re-exports hand-written
    # rusty::Rc<T>). Smoke test constructs an Rc<int>(42) and clones.
    tests/rc_port_module_test.cpp
    tests/arc_port_module_test.cpp
    tests/vec_deque_port_module_test.cpp
    tests/linked_list_port_module_test.cpp
    tests/string_port_module_test.cpp

    # borrow_port — Phase B smoke test: import borrow_port, construct
    # Cow<int>::Borrowed/Owned + verify is_borrowed/is_owned. Phase 3a/3b
    # trait machinery (commits 9fe6506 → fdefbff) unblocked the full body.
    tests/borrow_port_module_test.cpp

    # ascii_port — Phase B smoke test: import ascii_port, exercise
    # AsciiChar enum + manifest constants + to_u8/to_char. Other API
    # methods (to_uppercase/is_alphabetic/etc.) are patcher-stubbed
    # pending rusty::ascii::* helper additions.
    tests/ascii_port_module_test.cpp

    # TODO: Re-enable when String API is complete
    # tests/rusty_string_test.cpp
    # tests/test_all_types.cpp

    # TODO: Re-enable when performance/analysis features are complete
    # tests/final_btree_test.cpp
)

# Opt-in coverage-gap pin for the four latent bugs in btree_port's
# iter()/remove()/clone() paths (variant ._0 emit, NodeRef temp
# binding, const-correctness drift, copy-ctor on move-only T). The
# test EXPECTS TO FAIL TO COMPILE on btree_port as it stands today;
# it is wired through this flag so CI's default builds stay green
# while the bug coverage gap is pinned in tree.
# Enable with `cmake -DRUSTY_CPP_BUILD_BTREE_PORT_ITER_REMOVE_TEST=ON`.
if(RUSTY_CPP_BUILD_BTREE_PORT_ITER_REMOVE_TEST)
    list(APPEND TEST_SOURCES tests/btree_port_iter_remove_movonly_test.cpp)
endif()

find_package(Threads REQUIRED)

set(TEST_TARGETS)
foreach(test_source IN LISTS TEST_SOURCES)
    get_filename_component(test_name "${test_source}" NAME_WE)
    set(test_executable "${test_name}.out")
    # btree_port_* / vec_port_* / async_module_* tests depend on the
    # clang-only module libraries; skip them entirely under gcc.
    # Tests that now `import vec_port.vec` for rusty::Vec access (after
    # the VecLegacy retirement) join the clang-only set:
    set(_vec_consuming_tests
        test_mutex test_thread rusty_vec_test rusty_array_test)
    list(FIND _vec_consuming_tests "${test_name}" _vec_consume_idx)
    # Tests that `import rc_port` for the Send/Sync trait specializations
    # on rusty::Rc (which lives in rc_port post-legacy-retirement).
    set(_rc_consuming_tests test_traits)
    list(FIND _rc_consuming_tests "${test_name}" _rc_consume_idx)
    if((test_name MATCHES "^(btree_port|vec_port|binary_heap_port|binary_heap_tests_port|linked_list_tests_port|eq_diff_len_tests_port|vec_deque_tests_port|vec_tests_port|arc_tests_port|rc_tests_port|string_tests_port|btree_set_hash_tests_port|btree_tests_port|cell_port|rc_port|arc_port|vec_deque_port|linked_list_port|string_port|borrow_port|ascii_port|std_port)_"
        OR test_name STREQUAL "async_module_test"
        OR NOT _vec_consume_idx EQUAL -1
        OR NOT _rc_consume_idx EQUAL -1)
       AND NOT (CMAKE_CXX_COMPILER_ID STREQUAL "Clang"
                OR CMAKE_CXX_COMPILER_ID STREQUAL "AppleClang"))
        continue()
    endif()
    add_executable(${test_executable} ${test_source})
    # btree_tests_port driver also pulls in the hand-translated test
    # bodies that live in a sibling .cpp (separate TU to work around
    # an in-module-purview BTreeMap instantiation bug).
    if(test_name STREQUAL "btree_tests_port_module_test")
        target_sources(${test_executable} PRIVATE
            ${CMAKE_CURRENT_SOURCE_DIR}/tests/btree_tests_port_unstubbed.cpp)
        target_link_libraries(${test_executable} PRIVATE btree_port)
        target_compile_features(${test_executable} PRIVATE cxx_std_23)
        # assert() liveness for the un-stubbed bodies is handled in the
        # _stub_port foreach below — the option must be added exactly
        # once, at the LAST target_compile_options touching this target
        # (CMake dedupes repeated flags, so an early -UNDEBUG cannot be
        # re-added after a later -DNDEBUG).
        target_compile_options(${test_executable} PRIVATE -O3 -march=native)
    endif()
    target_include_directories(${test_executable} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/include)
    target_compile_options(${test_executable} PRIVATE -Wall -Wextra -Wpedantic)
    target_link_libraries(${test_executable} PRIVATE Threads::Threads)
    # Tests that `import rusty;` need the umbrella module library. The umbrella
    # re-exports std_port, so rusty::HashMap/HashSet resolve through it.
    if(test_name STREQUAL "hashset_set_algebra_test"
       OR test_name STREQUAL "rusty_umbrella_hashmap_test")
        target_link_libraries(${test_executable} PRIVATE rusty)
        target_compile_features(${test_executable} PRIVATE cxx_std_23)
        target_compile_options(${test_executable} PRIVATE -O3 -DNDEBUG -march=native)
    endif()
    # Tests that `import std_port;` directly (no umbrella) link the std port.
    # std_port PUBLIC-links std_port_hashbrown, so that comes along.
    if(test_name MATCHES "^std_port_")
        target_link_libraries(${test_executable} PRIVATE std_port)
        target_compile_features(${test_executable} PRIVATE cxx_std_23)
        target_compile_options(${test_executable} PRIVATE -O3 -DNDEBUG -march=native)
    endif()
    # Tests that import hashbrown_port.* link against the module library.
    # Tests that import btree_port.* link against the (clang-only) module
    # library. Same release-flag matching as hashbrown_port.
    if(test_name MATCHES "^btree_port_")
        target_link_libraries(${test_executable} PRIVATE btree_port)
        target_compile_features(${test_executable} PRIVATE cxx_std_23)
        target_compile_options(${test_executable} PRIVATE -O3 -DNDEBUG -march=native )
        target_link_options(${test_executable} PRIVATE )
    endif()
    # Same wiring for vec_port_* tests.
    if(test_name MATCHES "^vec_port_")
        target_link_libraries(${test_executable} PRIVATE vec_port)
        target_compile_features(${test_executable} PRIVATE cxx_std_23)
        target_compile_options(${test_executable} PRIVATE -O3 -DNDEBUG -march=native )
        target_link_options(${test_executable} PRIVATE )
    endif()
    # binary_heap_port_* tests link binary_heap_port (which pulls vec_port).
    if(test_name MATCHES "^binary_heap_port_")
        target_link_libraries(${test_executable} PRIVATE binary_heap_port)
        target_compile_features(${test_executable} PRIVATE cxx_std_23)
        target_compile_options(${test_executable} PRIVATE -O3 -DNDEBUG -march=native )
        target_link_options(${test_executable} PRIVATE )
    endif()
    # binary_heap_tests_port driver imports the transpiled tests module.
    if(test_name MATCHES "^binary_heap_tests_port_")
        target_link_libraries(${test_executable} PRIVATE binary_heap_tests_port)
        target_compile_features(${test_executable} PRIVATE cxx_std_23)
        target_compile_options(${test_executable} PRIVATE -O3 -DNDEBUG -march=native )
        target_link_options(${test_executable} PRIVATE )
    endif()
    # linked_list_tests_port driver — same wiring as binary_heap_tests_port.
    if(test_name MATCHES "^linked_list_tests_port_")
        target_link_libraries(${test_executable} PRIVATE linked_list_tests_port)
        target_compile_features(${test_executable} PRIVATE cxx_std_23)
        target_compile_options(${test_executable} PRIVATE -O3 -DNDEBUG -march=native )
        target_link_options(${test_executable} PRIVATE )
    endif()
    if(test_name MATCHES "^eq_diff_len_tests_port_")
        target_link_libraries(${test_executable} PRIVATE eq_diff_len_tests_port)
        target_compile_features(${test_executable} PRIVATE cxx_std_23)
        target_compile_options(${test_executable} PRIVATE -O3 -DNDEBUG -march=native )
        target_link_options(${test_executable} PRIVATE )
    endif()
    # Stub-only test ports — one link per port.
    foreach(_stub_port
        vec_deque_tests_port
        vec_tests_port
        arc_tests_port
        rc_tests_port
        string_tests_port
        btree_set_hash_tests_port)
        if(test_name MATCHES "^${_stub_port}_")
            if(_stub_port IN_LIST _alloc_backed_suites)
                # WHOLE_ARCHIVE is REQUIRED: the driver deliberately does not
                # import the suite module (that crashes clang 22 — see the
                # driver .cpp), so nothing references these objects and a plain
                # static-library link would drop them, silently registering
                # ZERO tests. Force them in.
                target_link_libraries(${test_executable} PRIVATE
                    "$<LINK_LIBRARY:WHOLE_ARCHIVE,${_stub_port}>")
            else()
                target_link_libraries(${test_executable} PRIVATE ${_stub_port})
            endif()
            target_compile_features(${test_executable} PRIVATE cxx_std_23)
            if(_stub_port IN_LIST _alloc_backed_suites)
                # Must MATCH the suite library's flags or clang rejects the
                # BMI. NDEBUG must not be defined here: CMake dedupes
                # repeated flags, so a -DNDEBUG added now could not be
                # undone later (see the btree note above).
                target_compile_options(${test_executable} PRIVATE
                    -O1 -march=native -UNDEBUG -DRUSTY_PORTABLE_INTRINSICS=1)
            else()
                target_compile_options(${test_executable} PRIVATE -O3 -DNDEBUG -march=native )
            endif()
            target_link_options(${test_executable} PRIVATE )
        endif()
    endforeach()
    # btree_tests_port_module_test no longer links a stub module (retired
    # 2026-07); its translated bodies check results via assert(), so make
    # the no-NDEBUG state explicit at this LAST option site.
    if(test_name STREQUAL "btree_tests_port_module_test")
        target_compile_options(${test_executable} PRIVATE -UNDEBUG)
    endif()
    # cell_port_* tests link cell_port (single-file C++20 module).
    if(test_name MATCHES "^cell_port_")
        target_link_libraries(${test_executable} PRIVATE cell_port)
        target_compile_features(${test_executable} PRIVATE cxx_std_23)
        target_compile_options(${test_executable} PRIVATE -O3 -DNDEBUG -march=native )
        target_link_options(${test_executable} PRIVATE )
    endif()
    # rc_port_* / arc_port_* / vec_deque_port_* / linked_list_port_* /
    # string_port_* tests link their respective Phase B/C bridge stub
    # modules.
    foreach(_bridge rc_port arc_port vec_deque_port linked_list_port string_port borrow_port ascii_port)
        if(test_name MATCHES "^${_bridge}_")
            target_link_libraries(${test_executable} PRIVATE ${_bridge})
            target_compile_features(${test_executable} PRIVATE cxx_std_23)
            target_compile_options(${test_executable} PRIVATE -O3 -DNDEBUG -march=native )
            target_link_options(${test_executable} PRIVATE )
        endif()
    endforeach()
    # rusty.async module test: links rusty_async (which transitively pulls vec_port).
    if(test_name STREQUAL "async_module_test")
        target_link_libraries(${test_executable} PRIVATE rusty_async)
        target_compile_features(${test_executable} PRIVATE cxx_std_23)
        target_compile_options(${test_executable} PRIVATE -O3 -DNDEBUG -march=native )
        target_link_options(${test_executable} PRIVATE )
    endif()
    # Tests that `import vec_port.vec` for rusty::Vec access after the
    # VecLegacy retirement: link against vec_port and use cxx_std_23.
    if(NOT _vec_consume_idx EQUAL -1)
        target_link_libraries(${test_executable} PRIVATE vec_port)
        target_compile_features(${test_executable} PRIVATE cxx_std_23)
        target_compile_options(${test_executable} PRIVATE -O3 -DNDEBUG -march=native )
        target_link_options(${test_executable} PRIVATE )
    endif()
    # Tests that `import rc_port` for rusty::Rc trait specializations.
    if(NOT _rc_consume_idx EQUAL -1)
        target_link_libraries(${test_executable} PRIVATE rc_port)
        target_compile_features(${test_executable} PRIVATE cxx_std_23)
        target_compile_options(${test_executable} PRIVATE -O3 -DNDEBUG -march=native )
        target_link_options(${test_executable} PRIVATE )
    endif()
    # Benches build like tests (same module/link wiring) but stay out of
    # ctest — they print timings rather than asserting, and we don't want
    # them to gate `ctest --output-on-failure`. Invoke directly:
    # `./build/<bench>.out`.
    if(test_name STREQUAL "rusty_fmt_parity_test")
        # The parity test reads the Rust-generated golden fixture; ctest runs
        # from the build dir, so pass the source-tree path explicitly.
        add_test(NAME ${test_name} COMMAND ${test_executable}
            ${CMAKE_CURRENT_SOURCE_DIR}/tests/fmt_parity/golden.txt)
    elseif(test_name STREQUAL "rusty_iter_parity_test")
        add_test(NAME ${test_name} COMMAND ${test_executable}
            ${CMAKE_CURRENT_SOURCE_DIR}/tests/iter_parity/golden.txt)
    elseif(NOT test_name MATCHES "_bench$")
        add_test(NAME ${test_name} COMMAND ${test_executable})
    endif()
    list(APPEND TEST_TARGETS ${test_executable})
endforeach()

add_custom_target(run-tests
    COMMAND ${CMAKE_CTEST_COMMAND} --output-on-failure
    DEPENDS ${TEST_TARGETS}
    WORKING_DIRECTORY ${CMAKE_BINARY_DIR}
)
