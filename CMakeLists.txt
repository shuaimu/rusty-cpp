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

    # BTreeMap tests - RE-ENABLED (all methods implemented)
    tests/rusty_btreemap_test.cpp

    # HashMap/HashSet now exposed via the transpiled hashbrown_port
    # C++20 module (transpiled/hashbrown_port/*.cppm). Tests import
    # the module directly; the old hand-written
    # include/rusty/{hashmap,hashset}.hpp is deleted.
    tests/hashbrown_port_map_test.cpp
    tests/hashbrown_port_set_test.cpp

    # BTreeSet tests - RE-ENABLED (fixed range and iterator)
    tests/rusty_btreeset_test.cpp

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
    add_test(NAME ${test_name} COMMAND ${test_executable})
    list(APPEND TEST_TARGETS ${test_executable})
endforeach()

add_custom_target(run-tests
    COMMAND ${CMAKE_CTEST_COMMAND} --output-on-failure
    DEPENDS ${TEST_TARGETS}
    WORKING_DIRECTORY ${CMAKE_BINARY_DIR}
)
