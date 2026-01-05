cmake_minimum_required(VERSION 3.16)
project(rusty_cpp_example CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Generate compile_commands.json for rusty-cpp
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Find rusty-cpp executable
# Users should set RUSTY_CPP_PATH or have it in PATH
find_program(RUSTY_CPP_EXECUTABLE
    NAMES rusty-cpp-checker rusty-cpp
    HINTS
        ${RUSTY_CPP_PATH}
        $ENV{RUSTY_CPP_PATH}
        ${CMAKE_SOURCE_DIR}/../target/release
        ${CMAKE_SOURCE_DIR}/../target/debug
    DOC "Path to rusty-cpp-checker executable"
)

if(NOT RUSTY_CPP_EXECUTABLE)
    message(WARNING "rusty-cpp-checker not found. Safety checks will be skipped. "
                    "Set RUSTY_CPP_PATH or build rusty-cpp with 'cargo build --release'")
endif()

# Option to enable/disable rusty-cpp checks
option(ENABLE_RUSTY_CPP "Enable rusty-cpp safety checks" ON)

# Source files
set(SOURCES
    src/main.cpp
    src/safe_example.cpp
)

# Main executable
add_executable(example ${SOURCES})

# Function to add rusty-cpp check for a source file
function(add_rusty_cpp_check target source_file)
    if(RUSTY_CPP_EXECUTABLE AND ENABLE_RUSTY_CPP)
        get_filename_component(source_name ${source_file} NAME)
        set(check_target "rusty_check_${source_name}")

        add_custom_command(
            OUTPUT ${CMAKE_BINARY_DIR}/.rusty_checked_${source_name}
            COMMAND ${RUSTY_CPP_EXECUTABLE}
                    ${CMAKE_CURRENT_SOURCE_DIR}/${source_file}
                    --compile-commands ${CMAKE_BINARY_DIR}/compile_commands.json
            COMMAND ${CMAKE_COMMAND} -E touch ${CMAKE_BINARY_DIR}/.rusty_checked_${source_name}
            DEPENDS ${CMAKE_CURRENT_SOURCE_DIR}/${source_file}
            COMMENT "Running rusty-cpp on ${source_file}"
            VERBATIM
        )

        add_custom_target(${check_target}
            DEPENDS ${CMAKE_BINARY_DIR}/.rusty_checked_${source_name}
        )

        add_dependencies(${target} ${check_target})
    endif()
endfunction()

# Add rusty-cpp checks for all source files
foreach(source ${SOURCES})
    add_rusty_cpp_check(example ${source})
endforeach()

# Convenience target to run all rusty-cpp checks
if(RUSTY_CPP_EXECUTABLE AND ENABLE_RUSTY_CPP)
    add_custom_target(rusty_check_all
        COMMENT "Running rusty-cpp on all source files"
    )

    foreach(source ${SOURCES})
        get_filename_component(source_name ${source} NAME)
        add_dependencies(rusty_check_all "rusty_check_${source_name}")
    endforeach()
endif()

# Print configuration summary
message(STATUS "")
message(STATUS "=== Rusty-CPP Configuration ===")
message(STATUS "  Enabled: ${ENABLE_RUSTY_CPP}")
if(RUSTY_CPP_EXECUTABLE)
    message(STATUS "  Executable: ${RUSTY_CPP_EXECUTABLE}")
else()
    message(STATUS "  Executable: NOT FOUND")
endif()
message(STATUS "")
