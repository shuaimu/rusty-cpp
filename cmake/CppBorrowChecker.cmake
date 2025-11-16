# CppBorrowChecker.cmake - CMake integration for Rusty C++ Checker
#
# This module provides functions to integrate the Rusty C++ Checker into your CMake build.
#
# Usage:
#   include(CppBorrowChecker.cmake)
#   
#   # Enable for all targets
#   enable_borrow_checking()
#   
#   # Or enable for specific targets
#   add_borrow_check_target(my_target)
#   
#   # Or run on specific files
#   add_borrow_check(my_file.cpp)

# Find the borrow checker executable
find_program(CPP_BORROW_CHECKER
    NAMES rusty-cpp-checker cpp-borrow-checker
    PATHS 
        ${CMAKE_CURRENT_LIST_DIR}/../target/release
        ${CMAKE_CURRENT_LIST_DIR}/../target/debug
        $ENV{HOME}/.cargo/bin
        /usr/local/bin
        /usr/bin
    DOC "Path to the Rusty C++ Checker executable"
)

if(NOT CPP_BORROW_CHECKER)
    message(WARNING "Rusty C++ Checker not found. Please build and install it first.")
    message(STATUS "To install: cd ${CMAKE_CURRENT_LIST_DIR}/.. && cargo build --release")
endif()

# Option to enable/disable borrow checking
option(ENABLE_BORROW_CHECKING "Enable C++ Borrow Checking" OFF)
option(BORROW_CHECK_FATAL "Make borrow check failures fatal" OFF)

# Set default include paths for the checker
set(BORROW_CHECKER_INCLUDE_PATHS "" CACHE STRING "Additional include paths for borrow checker")

# Function to add borrow checking for a single file
function(add_borrow_check SOURCE_FILE)
    if(NOT CPP_BORROW_CHECKER)
        return()
    endif()
    
    if(NOT ENABLE_BORROW_CHECKING)
        return()
    endif()
    
    get_filename_component(FILE_NAME ${SOURCE_FILE} NAME_WE)
    set(CHECK_NAME "borrow_check_${FILE_NAME}")
    
    # Get include directories from the current directory properties
    get_directory_property(INCLUDE_DIRS INCLUDE_DIRECTORIES)
    
    # Build include flags
    set(INCLUDE_FLAGS)
    foreach(DIR ${INCLUDE_DIRS})
        list(APPEND INCLUDE_FLAGS "-I${DIR}")
    endforeach()
    
    # Add user-specified includes
    foreach(DIR ${BORROW_CHECKER_INCLUDE_PATHS})
        list(APPEND INCLUDE_FLAGS "-I${DIR}")
    endforeach()
    
    # NOTE: We use add_custom_target with COMMAND instead of add_custom_command
    # so that borrow checking always runs on every build (see note in add_borrow_check_target)
    add_custom_target(${CHECK_NAME} ALL
        COMMAND ${CPP_BORROW_CHECKER} ${CMAKE_CURRENT_SOURCE_DIR}/${SOURCE_FILE} ${INCLUDE_FLAGS}
        COMMENT "Borrow checking ${SOURCE_FILE}"
        VERBATIM
    )
    
    # Make it fatal if requested
    if(BORROW_CHECK_FATAL)
        set_property(TARGET ${CHECK_NAME} PROPERTY JOB_POOL_COMPILE console)
    endif()
endfunction()

# Function to add borrow checking for a target
function(add_borrow_check_target TARGET_NAME)
    if(NOT CPP_BORROW_CHECKER)
        return()
    endif()
    
    if(NOT ENABLE_BORROW_CHECKING)
        return()
    endif()
    
    # Get source files from target
    get_target_property(SOURCES ${TARGET_NAME} SOURCES)
    
    # Get include directories from target
    get_target_property(TARGET_INCLUDES ${TARGET_NAME} INCLUDE_DIRECTORIES)
    
    # Build include flags
    set(INCLUDE_FLAGS)
    if(TARGET_INCLUDES)
        foreach(DIR ${TARGET_INCLUDES})
            list(APPEND INCLUDE_FLAGS "-I${DIR}")
        endforeach()
    endif()
    
    # Add checks for each source file
    foreach(SOURCE ${SOURCES})
        # Only check .cpp files
        if(SOURCE MATCHES "\\.(cpp|cc|cxx)$")
            get_filename_component(FILE_NAME ${SOURCE} NAME_WE)
            set(CHECK_NAME "borrow_check_${TARGET_NAME}_${FILE_NAME}")
            
            # Get absolute path for the source file
            get_filename_component(SOURCE_ABS ${SOURCE} ABSOLUTE)
            
            # NOTE: We use add_custom_target with COMMAND instead of add_custom_command
            # so that borrow checking always runs on every build. This is necessary because:
            # 1. Changes to header files (.hpp) need to trigger re-checking
            # 2. CMake's automatic dependency tracking only works for compilation, not custom commands
            # 3. Borrow checking is fast enough that running it every time is acceptable
            # TODO: Implement proper dependency tracking using DEPFILE in the future
            add_custom_target(${CHECK_NAME}
                COMMAND ${CPP_BORROW_CHECKER} ${SOURCE_ABS} ${INCLUDE_FLAGS}
                COMMENT "Borrow checking ${SOURCE} (from ${TARGET_NAME})"
                VERBATIM
            )
            
            add_dependencies(${TARGET_NAME} ${CHECK_NAME})
        endif()
    endforeach()
endfunction()

# Function to enable borrow checking globally
function(enable_borrow_checking)
    if(NOT CPP_BORROW_CHECKER)
        message(WARNING "Cannot enable borrow checking: rusty-cpp-checker not found")
        return()
    endif()
    
    set(ENABLE_BORROW_CHECKING ON PARENT_SCOPE)
    message(STATUS "C++ Borrow Checking enabled globally")
    message(STATUS "Using checker: ${CPP_BORROW_CHECKER}")
endfunction()

# Create a custom target for checking all files
if(CPP_BORROW_CHECKER AND ENABLE_BORROW_CHECKING)
    add_custom_target(borrow_check_all
        COMMENT "Running borrow checker on all C++ files"
    )
endif()

# Function to add compile_commands.json support
function(setup_borrow_checker_compile_commands)
    set(CMAKE_EXPORT_COMPILE_COMMANDS ON PARENT_SCOPE)
    
    # Create a custom target that runs borrow checker with compile_commands.json
    if(CPP_BORROW_CHECKER)
        add_custom_target(borrow_check_with_compile_commands
            COMMAND ${CPP_BORROW_CHECKER} 
                    --compile-commands ${CMAKE_BINARY_DIR}/compile_commands.json
                    \${SOURCE_FILE}
            WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
            COMMENT "Run: cmake --build . --target borrow_check_with_compile_commands -- SOURCE_FILE=<path/to/file.cpp>"
            VERBATIM
        )
    endif()
endfunction()

# Macro to mark files as safe
macro(mark_safe)
    foreach(FILE ${ARGN})
        set_source_files_properties(${FILE} PROPERTIES 
            COMPILE_DEFINITIONS "BORROW_CHECKER_SAFE"
        )
    endforeach()
endmacro()

# Function to install the borrow checker
function(install_borrow_checker)
    if(CPP_BORROW_CHECKER)
        install(PROGRAMS ${CPP_BORROW_CHECKER}
                DESTINATION ${CMAKE_INSTALL_BINDIR}
                COMPONENT borrow_checker)
    endif()
endfunction()