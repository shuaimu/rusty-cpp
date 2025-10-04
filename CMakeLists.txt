cmake_minimum_required(VERSION 3.16)
project(rusty_cpp_tests LANGUAGES CXX)

include(CTest)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(TEST_SOURCES
    tests/rusty_arc_test.cpp
    tests/rusty_box_test.cpp
    tests/rusty_cell_test.cpp
    tests/rusty_option_test.cpp
    tests/rusty_rc_test.cpp
    tests/rusty_refcell_test.cpp
    tests/rusty_result_test.cpp
    tests/rusty_vec_test.cpp
)

find_package(Threads REQUIRED)

set(TEST_TARGETS)
foreach(test_source IN LISTS TEST_SOURCES)
    get_filename_component(test_name "${test_source}" NAME_WE)
    add_executable(${test_name} ${test_source})
    target_include_directories(${test_name} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/include)
    target_compile_options(${test_name} PRIVATE -Wall -Wextra -Wpedantic)
    target_link_libraries(${test_name} PRIVATE Threads::Threads)
    add_test(NAME ${test_name} COMMAND ${test_name})
    list(APPEND TEST_TARGETS ${test_name})
endforeach()

add_custom_target(run-tests
    COMMAND ${CMAKE_CTEST_COMMAND} --output-on-failure
    DEPENDS ${TEST_TARGETS}
    WORKING_DIRECTORY ${CMAKE_BINARY_DIR}
)
