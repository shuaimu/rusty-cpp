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
target_compile_options(hashbrown_port PRIVATE -O3 -DNDEBUG -march=native -flto=thin)

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
    target_compile_options(btree_port PRIVATE -O3 -DNDEBUG -march=native -flto=thin)
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
    # btree_port_* tests depend on the clang-only btree_port module
    # library; skip them entirely under gcc.
    if(test_name MATCHES "^btree_port_"
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
        target_compile_options(${test_executable} PRIVATE -O3 -DNDEBUG -march=native -flto=thin)
        target_link_options(${test_executable} PRIVATE -flto=thin)
    endif()
    # Tests that import btree_port.* link against the (clang-only) module
    # library. Same release-flag matching as hashbrown_port.
    if(test_name MATCHES "^btree_port_")
        target_link_libraries(${test_executable} PRIVATE btree_port)
        target_compile_features(${test_executable} PRIVATE cxx_std_23)
        target_compile_options(${test_executable} PRIVATE -O3 -DNDEBUG -march=native -flto=thin)
        target_link_options(${test_executable} PRIVATE -flto=thin)
    endif()
    add_test(NAME ${test_name} COMMAND ${test_executable})
    list(APPEND TEST_TARGETS ${test_executable})
endforeach()

add_custom_target(run-tests
    COMMAND ${CMAKE_CTEST_COMMAND} --output-on-failure
    DEPENDS ${TEST_TARGETS}
    WORKING_DIRECTORY ${CMAKE_BINARY_DIR}
)
