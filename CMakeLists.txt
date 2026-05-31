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
# Phase B/C bridge modules for rc_port, arc_port, vec_deque_port,
# linked_list_port, string_port. Each port's full transpiled .cppm
# is blocked on transpiler-side issues (catalogued in their STATUS.md
# files). The bridge `*_port_stub.cppm` re-exports the hand-written
# equivalent (rusty::Rc, rusty::Arc, rusty::VecDeque, std::list,
# rusty::String) under the port's namespace — giving us a working
# library + smoke test today. When transpiler fixes land, swap the
# stub for the full file.
# ──────────────────────────────────────────────────────────────────────
if(CMAKE_CXX_COMPILER_ID STREQUAL "Clang"
   OR CMAKE_CXX_COMPILER_ID STREQUAL "AppleClang")
    foreach(_port rc_port arc_port vec_deque_port string_port)
        add_library(${_port} STATIC)
        target_sources(${_port}
            PUBLIC FILE_SET CXX_MODULES FILES
            ${CMAKE_CURRENT_SOURCE_DIR}/transpiled/${_port}/${_port}_stub.cppm
        )
        target_include_directories(${_port} PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
        target_compile_features(${_port} PUBLIC cxx_std_23)
        target_compile_options(${_port} PRIVATE -O3 -DNDEBUG -march=native )
    endforeach()
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
# vec_deque_port — transpiled rustc VecDeque (ring buffer).
# Multi-file (10 .cppm files). CMake target intentionally not wired —
# the hand-written rusty::VecDeque<T> takes one template arg but
# transpiled code emits VecDeque<T, A>; same shape as rc_port's
# blocker. Patcher seeded in docs/vec_deque_port/post_transpile_patch.py;
# see docs/vec_deque_port/STATUS.md for the full punch list.
#
# if(CMAKE_CXX_COMPILER_ID STREQUAL "Clang"
#    OR CMAKE_CXX_COMPILER_ID STREQUAL "AppleClang")
#     file(GLOB VEC_DEQUE_PORT_CPPM transpiled/vec_deque_port/*.cppm)
#     add_library(vec_deque_port STATIC)
#     target_sources(vec_deque_port
#         PUBLIC FILE_SET CXX_MODULES FILES ${VEC_DEQUE_PORT_CPPM}
#     )
#     target_include_directories(vec_deque_port PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
#     target_compile_features(vec_deque_port PUBLIC cxx_std_23)
#     target_compile_options(vec_deque_port PRIVATE -O3 -DNDEBUG -march=native )
#     target_link_libraries(vec_deque_port PUBLIC vec_port)
# endif()

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
    tests/rusty_rc_test.cpp
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

    # binary_heap_port Phase C: push() drive test, iterates on the 6
    # instantiation clusters documented in docs/binary_heap_port/STATUS.md.
    tests/binary_heap_port_push_test.cpp

    # binary_heap_port Phase C: pop() exercise — sift-down path
    # (hits clusters C4/C5/C6).
    tests/binary_heap_port_pop_test.cpp

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
    if((test_name MATCHES "^(btree_port|vec_port|binary_heap_port|cell_port|rc_port|arc_port|vec_deque_port|linked_list_port|string_port)_"
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
    foreach(_bridge rc_port arc_port vec_deque_port linked_list_port string_port)
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
    add_test(NAME ${test_name} COMMAND ${test_executable})
    list(APPEND TEST_TARGETS ${test_executable})
endforeach()

add_custom_target(run-tests
    COMMAND ${CMAKE_CTEST_COMMAND} --output-on-failure
    DEPENDS ${TEST_TARGETS}
    WORKING_DIRECTORY ${CMAKE_BINARY_DIR}
)
