# RustyCppTranspiler.cmake - CMake integration for rusty-cpp-transpiler
#
# This module provides helper functions to transpile Rust sources into C++
# module/interface files as part of a normal CMake build.
#
# Usage:
#   include(${RUSTYCPP_DIR}/cmake/RustyCppTranspiler.cmake)
#
#   rustycpp_add_transpile_target(
#       my_transpile_target
#       SOURCE ${CMAKE_CURRENT_SOURCE_DIR}/src/lib.rs
#       OUTPUT ${CMAKE_CURRENT_BINARY_DIR}/generated/my_crate.cppm
#       MODULE_NAME my_crate
#       CPP_MODULE_INDEX ${CMAKE_CURRENT_SOURCE_DIR}/src/cpp_module_index.toml
#   )

include(CMakeParseArguments)

# Respect caller-provided RUSTYCPP_DIR, otherwise derive from this module path.
if(NOT DEFINED RUSTYCPP_DIR)
    get_filename_component(RUSTYCPP_DIR "${CMAKE_CURRENT_LIST_DIR}/.." ABSOLUTE)
endif()

# Keep transpiler build type aligned with checker build type by default.
if(NOT DEFINED RUSTYCPP_BUILD_TYPE)
    set(RUSTYCPP_BUILD_TYPE "release")
endif()

if(DEFINED ENV{CARGO_TARGET_DIR})
    set(RUSTYCPP_TRANSPILER_TARGET_BASE "$ENV{CARGO_TARGET_DIR}")
else()
    set(RUSTYCPP_TRANSPILER_TARGET_BASE "${RUSTYCPP_DIR}/target")
endif()

set(RUSTYCPP_TRANSPILER_CARGO_BUILD_FLAGS)
if(RUSTYCPP_BUILD_TYPE STREQUAL "release")
    set(RUSTYCPP_TRANSPILER_TARGET_DIR "${RUSTYCPP_TRANSPILER_TARGET_BASE}/release")
    list(APPEND RUSTYCPP_TRANSPILER_CARGO_BUILD_FLAGS --release)
else()
    set(RUSTYCPP_TRANSPILER_TARGET_DIR "${RUSTYCPP_TRANSPILER_TARGET_BASE}/debug")
endif()

set(RUSTYCPP_TRANSPILER "${RUSTYCPP_TRANSPILER_TARGET_DIR}/rusty-cpp-transpiler")
if(WIN32)
    set(RUSTYCPP_TRANSPILER "${RUSTYCPP_TRANSPILER}.exe")
endif()

function(create_rustycpp_transpiler_build_target)
    if(TARGET build_rusty_cpp_transpiler)
        return()
    endif()

    find_program(CARGO_EXECUTABLE cargo)
    if(NOT CARGO_EXECUTABLE)
        message(FATAL_ERROR "cargo not found in PATH; required to build rusty-cpp-transpiler")
    endif()

    add_custom_target(build_rusty_cpp_transpiler
        COMMAND
            ${CARGO_EXECUTABLE}
            build
            -p
            rusty-cpp-transpiler
            ${RUSTYCPP_TRANSPILER_CARGO_BUILD_FLAGS}
        WORKING_DIRECTORY "${RUSTYCPP_DIR}"
        COMMENT "Building rusty-cpp-transpiler..."
        VERBATIM
    )
endfunction()

# rustycpp_add_transpile_target(<target>
#   SOURCE <rust_source.rs>
#   OUTPUT <generated.cppm>
#   MODULE_NAME <cpp_module_name>
#   [CPP_MODULE_INDEX <index1> <index2> ...]
#   [EXTRA_ARGS <...>]
#   [DEPENDS <...>]
#   [WORKING_DIRECTORY <dir>]
#   [OUT_VAR <var_name>]
# )
function(rustycpp_add_transpile_target TARGET_NAME)
    set(options)
    set(one_value_args SOURCE OUTPUT MODULE_NAME WORKING_DIRECTORY OUT_VAR)
    set(multi_value_args CPP_MODULE_INDEX EXTRA_ARGS DEPENDS)
    cmake_parse_arguments(RCT "${options}" "${one_value_args}" "${multi_value_args}" ${ARGN})

    if(NOT RCT_SOURCE)
        message(FATAL_ERROR "rustycpp_add_transpile_target(${TARGET_NAME}): SOURCE is required")
    endif()
    if(NOT RCT_OUTPUT)
        message(FATAL_ERROR "rustycpp_add_transpile_target(${TARGET_NAME}): OUTPUT is required")
    endif()
    if(NOT RCT_MODULE_NAME)
        message(FATAL_ERROR "rustycpp_add_transpile_target(${TARGET_NAME}): MODULE_NAME is required")
    endif()

    create_rustycpp_transpiler_build_target()

    set(_transpile_workdir "${RUSTYCPP_DIR}")
    if(RCT_WORKING_DIRECTORY)
        set(_transpile_workdir "${RCT_WORKING_DIRECTORY}")
    endif()

    get_filename_component(_transpile_output_dir "${RCT_OUTPUT}" DIRECTORY)
    set(_transpile_cmd
        "${RUSTYCPP_TRANSPILER}"
        "${RCT_SOURCE}"
        --output
        "${RCT_OUTPUT}"
        --module-name
        "${RCT_MODULE_NAME}"
    )

    foreach(_index_file IN LISTS RCT_CPP_MODULE_INDEX)
        list(APPEND _transpile_cmd --cpp-module-index "${_index_file}")
    endforeach()
    if(RCT_EXTRA_ARGS)
        list(APPEND _transpile_cmd ${RCT_EXTRA_ARGS})
    endif()

    add_custom_command(
        OUTPUT "${RCT_OUTPUT}"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${_transpile_output_dir}"
        COMMAND ${_transpile_cmd}
        DEPENDS
            build_rusty_cpp_transpiler
            "${RCT_SOURCE}"
            ${RCT_CPP_MODULE_INDEX}
            ${RCT_DEPENDS}
        WORKING_DIRECTORY "${_transpile_workdir}"
        COMMENT "Transpiling ${RCT_SOURCE} -> ${RCT_OUTPUT} (module: ${RCT_MODULE_NAME})"
        VERBATIM
    )

    add_custom_target(${TARGET_NAME}
        DEPENDS "${RCT_OUTPUT}"
    )

    set_source_files_properties("${RCT_OUTPUT}" PROPERTIES GENERATED TRUE)

    if(RCT_OUT_VAR)
        set(${RCT_OUT_VAR} "${RCT_OUTPUT}" PARENT_SCOPE)
    endif()
endfunction()
