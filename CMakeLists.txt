cmake_minimum_required(VERSION 3.16)
project(rusty_cpp_tests LANGUAGES CXX)

include(CTest)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(TEST_SOURCES
    # Basic Rusty types - ALL WORKING
    tests/rusty_arc_test.cpp
    tests/rusty_box_test.cpp
    tests/rusty_cell_test.cpp
    tests/rusty_option_test.cpp
    tests/rusty_rc_test.cpp
    tests/rusty_refcell_test.cpp
    tests/rusty_result_test.cpp
    tests/rusty_vec_test.cpp
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

    # HashMap/HashSet tests - RE-ENABLED (APIs complete)
    tests/rusty_hashmap_test.cpp
    tests/rusty_hashset_test.cpp

    # BTreeSet tests - RE-ENABLED (fixed range and iterator)
    tests/rusty_btreeset_test.cpp

    # TODO: Re-enable when String API is complete
    # tests/rusty_string_test.cpp
    # tests/test_all_types.cpp

    # TODO: Re-enable when performance/analysis features are complete
    # tests/btreemap_analysis.cpp
    # tests/final_btree_test.cpp
    # tests/final_performance_test.cpp
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
    add_test(NAME ${test_name} COMMAND ${test_executable})
    list(APPEND TEST_TARGETS ${test_executable})
endforeach()

add_custom_target(run-tests
    COMMAND ${CMAKE_CTEST_COMMAND} --output-on-failure
    DEPENDS ${TEST_TARGETS}
    WORKING_DIRECTORY ${CMAKE_BINARY_DIR}
)
