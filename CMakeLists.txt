cmake_minimum_required(VERSION 3.28)  # C++20 modules need 3.28+
project(rusty_cpp_tests LANGUAGES CXX)

include(CTest)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

option(RUSTY_PLATFORM_POSIX "Use POSIX backend for rusty::platform primitives" OFF)
if(RUSTY_PLATFORM_POSIX)
    add_compile_definitions(RUSTY_PLATFORM_BACKEND_POSIX=1)
endif()

# ──────────────────────────────────────────────────────────────────────
# hashbrown_port — transpiled Rust hashbrown port, exposed as C++20 modules.
# Replaces the old hand-written include/rusty/hashmap.hpp / hashset.hpp.
# Sources live in transpiled/hashbrown_port/*.cppm and are vendored from
# the transpiler+patcher pipeline (docs/hashbrown_port/).
# ──────────────────────────────────────────────────────────────────────
file(GLOB HASHBROWN_PORT_CPPM transpiled/hashbrown_port/*.cppm)
add_library(hashbrown_port STATIC)
target_sources(hashbrown_port
    PUBLIC FILE_SET CXX_MODULES FILES ${HASHBROWN_PORT_CPPM}
)
target_include_directories(hashbrown_port PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
# Hashbrown_port uses std::println, std::span etc. — needs C++23.
# Rest of the repo stays at C++20 to avoid touching unrelated tests.
target_compile_features(hashbrown_port PUBLIC cxx_std_23)
# Match the bench build: O3+LTO so the module-boundary inlining works.
target_compile_options(hashbrown_port PRIVATE -O3 -DNDEBUG -march=native )

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
# Stub-only test-port modules for the remaining rustc collection tests.
# Each is a hand-written .cppm (generated by docs/_gen_test_stub.py from
# the Rust source's #[test] annotations) that registers every test as a
# skip. The pipeline directory (docs/<name>_tests_port/) is kept so the
# transpile + patch flow can be re-run once the prerequisites land.
# ──────────────────────────────────────────────────────────────────────
if(CMAKE_CXX_COMPILER_ID STREQUAL "Clang"
   OR CMAKE_CXX_COMPILER_ID STREQUAL "AppleClang")
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
        target_compile_options(${_port} PRIVATE -O3 -DNDEBUG -march=native)
    endforeach()
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

    # HashMap/HashSet now exposed via the transpiled hashbrown_port
    # C++20 module (transpiled/hashbrown_port/*.cppm). Tests import
    # the module directly; the old hand-written
    # include/rusty/{hashmap,hashset}.hpp is deleted.
    tests/hashbrown_port_map_test.cpp
    tests/hashbrown_port_set_test.cpp

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

    # vec_deque_tests_port — driver for tests/vec_deque.rs (101 tests, stub).
    tests/vec_deque_tests_port_module_test.cpp
    # vec_tests_port — driver for tests/vec.rs (148 tests, stub).
    tests/vec_tests_port_module_test.cpp
    # arc_tests_port — driver for tests/arc.rs (14 tests, stub).
    tests/arc_tests_port_module_test.cpp
    # rc_tests_port — driver for tests/rc.rs (61 tests, stub).
    tests/rc_tests_port_module_test.cpp
    # string_tests_port — driver for tests/string.rs (57 tests, stub).
    tests/string_tests_port_module_test.cpp
    # btree_set_hash_tests_port — driver for tests/btree_set_hash.rs (2 tests, stub).
    tests/btree_set_hash_tests_port_module_test.cpp

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
    if((test_name MATCHES "^(btree_port|vec_port|binary_heap_port|binary_heap_tests_port|linked_list_tests_port|eq_diff_len_tests_port|vec_deque_tests_port|vec_tests_port|arc_tests_port|rc_tests_port|string_tests_port|btree_set_hash_tests_port|cell_port|rc_port|arc_port|vec_deque_port|linked_list_port|string_port|borrow_port|ascii_port)_"
        OR test_name STREQUAL "async_module_test"
        OR NOT _vec_consume_idx EQUAL -1)
       AND NOT (CMAKE_CXX_COMPILER_ID STREQUAL "Clang"
                OR CMAKE_CXX_COMPILER_ID STREQUAL "AppleClang"))
        continue()
    endif()
    add_executable(${test_executable} ${test_source})
    target_include_directories(${test_executable} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/include)
    target_compile_options(${test_executable} PRIVATE -Wall -Wextra -Wpedantic)
    target_link_libraries(${test_executable} PRIVATE Threads::Threads)
    # Tests that import hashbrown_port.* link against the module library.
    if(test_name MATCHES "^hashbrown_port_")
        target_link_libraries(${test_executable} PRIVATE hashbrown_port)
        # Match the library's release flags so the module BMIs are compatible.
        target_compile_features(${test_executable} PRIVATE cxx_std_23)
        target_compile_options(${test_executable} PRIVATE -O3 -DNDEBUG -march=native )
        target_link_options(${test_executable} PRIVATE )
    endif()
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
            target_link_libraries(${test_executable} PRIVATE ${_stub_port})
            target_compile_features(${test_executable} PRIVATE cxx_std_23)
            target_compile_options(${test_executable} PRIVATE -O3 -DNDEBUG -march=native )
            target_link_options(${test_executable} PRIVATE )
        endif()
    endforeach()
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
    # Benches build like tests (same module/link wiring) but stay out of
    # ctest — they print timings rather than asserting, and we don't want
    # them to gate `ctest --output-on-failure`. Invoke directly:
    # `./build/<bench>.out`.
    if(NOT test_name MATCHES "_bench$")
        add_test(NAME ${test_name} COMMAND ${test_executable})
    endif()
    list(APPEND TEST_TARGETS ${test_executable})
endforeach()

add_custom_target(run-tests
    COMMAND ${CMAKE_CTEST_COMMAND} --output-on-failure
    DEPENDS ${TEST_TARGETS}
    WORKING_DIRECTORY ${CMAKE_BINARY_DIR}
)
