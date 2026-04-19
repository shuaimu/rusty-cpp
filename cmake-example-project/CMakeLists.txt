cmake_minimum_required(VERSION 3.16)
project(rusty_cpp_example CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
include(CTest)

# Generate compile_commands.json
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# =============================================================================
# Rusty-CPP Integration
# =============================================================================
# Set RUSTYCPP_DIR to point to the rusty-cpp directory
# This can be:
#   1. A git submodule: set(RUSTYCPP_DIR "${CMAKE_SOURCE_DIR}/third-party/rusty-cpp")
#   2. An external path: set(RUSTYCPP_DIR "/path/to/rusty-cpp")
#   3. Relative to this example: (default below)

if(NOT DEFINED RUSTYCPP_DIR)
    # Default: assume this example is inside the rusty-cpp repo
    set(RUSTYCPP_DIR "${CMAKE_SOURCE_DIR}/..")
endif()

message(STATUS "RUSTYCPP_DIR: ${RUSTYCPP_DIR}")

# Include the RustyCpp CMake module
# This provides: enable_borrow_checking(), add_borrow_check_target(), etc.
include(${RUSTYCPP_DIR}/cmake/RustyCppSubmodule.cmake)

# Set build type for rusty-cpp (release is recommended for speed)
set(RUSTYCPP_BUILD_TYPE "release" CACHE STRING "Build type for rusty-cpp")

# Enable borrow checking - this will:
#   1. Check for required dependencies (cargo, libclang, z3)
#   2. Create a build target for rusty-cpp-checker
enable_borrow_checking()

# =============================================================================
# Project Sources
# =============================================================================

set(SOURCES
    src/main.cpp
    src/safe_example.cpp
)

# Main executable
add_executable(example ${SOURCES})

# Add rusty-cpp include path for rusty:: types (Box, Arc, Vec, etc.)
target_include_directories(example PRIVATE ${RUSTYCPP_DIR}/include)

# Enable borrow checking for this target
# This will run rusty-cpp-checker on all source files before compilation
add_borrow_check_target(example)

# =============================================================================
# C++ <-> Rust Interop Demo Test (Member Calls in Both Directions)
# =============================================================================

option(
    ENABLE_CPP_RUST_MEMBER_INTEROP_TEST
    "Enable transpiler + C++20 module interop test in this example project"
    ON
)

find_program(BASH_EXECUTABLE bash)
if(ENABLE_CPP_RUST_MEMBER_INTEROP_TEST AND BASH_EXECUTABLE)
    set(
        CPP_RUST_MEMBER_INTEROP_SCRIPT
        "${CMAKE_CURRENT_SOURCE_DIR}/tests/run_cpp_rust_member_interop.sh"
    )
    set(
        CPP_RUST_MEMBER_INTEROP_WORK_DIR
        "${CMAKE_BINARY_DIR}/cpp_rust_member_interop"
    )

    add_test(
        NAME cpp_rust_member_interop
        COMMAND
            ${BASH_EXECUTABLE}
            ${CPP_RUST_MEMBER_INTEROP_SCRIPT}
            --rustycpp-dir
            ${RUSTYCPP_DIR}
            --work-dir
            ${CPP_RUST_MEMBER_INTEROP_WORK_DIR}
    )

    add_custom_target(cpp_rust_member_interop_test
        COMMAND
            ${BASH_EXECUTABLE}
            ${CPP_RUST_MEMBER_INTEROP_SCRIPT}
            --rustycpp-dir
            ${RUSTYCPP_DIR}
            --work-dir
            ${CPP_RUST_MEMBER_INTEROP_WORK_DIR}
        COMMENT "Running C++ <-> Rust member interop smoke test"
        USES_TERMINAL
    )
else()
    message(
        STATUS
        "Skipping cpp_rust_member_interop test wiring (ENABLE_CPP_RUST_MEMBER_INTEROP_TEST=${ENABLE_CPP_RUST_MEMBER_INTEROP_TEST}, bash=${BASH_EXECUTABLE})"
    )
endif()

# =============================================================================
# Build Summary
# =============================================================================

message(STATUS "")
message(STATUS "=== Build Configuration ===")
message(STATUS "  C++ Standard: ${CMAKE_CXX_STANDARD}")
message(STATUS "  Borrow Checking: ${ENABLE_BORROW_CHECKING}")
message(STATUS "  Interop Test: ${ENABLE_CPP_RUST_MEMBER_INTEROP_TEST}")
message(STATUS "  RustyCpp Dir: ${RUSTYCPP_DIR}")
message(STATUS "")
