# RustyCppSubmodule.cmake - Submodule integration for Rusty C++ Checker
#
# This module enables using rusty-cpp-checker as a git submodule without global installation.
# It builds the checker as part of your project and uses it directly from the build directory.
#
# Usage in your project's CMakeLists.txt:
#   add_subdirectory(external/rustycpp)  # Or wherever you placed the submodule
#   include(${CMAKE_CURRENT_SOURCE_DIR}/external/rustycpp/cmake/RustyCppSubmodule.cmake)
#   
#   # Then use the same functions as before:
#   enable_borrow_checking()
#   add_borrow_check_target(my_target)
#   add_borrow_check(my_file.cpp)

# Detect the rusty-cpp directory (should be the parent of this cmake file)
get_filename_component(RUSTYCPP_DIR "${CMAKE_CURRENT_LIST_DIR}/.." ABSOLUTE)

# Option to control the Rust build type
set(RUSTYCPP_BUILD_TYPE "release" CACHE STRING "Build type for rusty-cpp-checker (debug or release)")
set_property(CACHE RUSTYCPP_BUILD_TYPE PROPERTY STRINGS "debug" "release")

# Option to enable/disable borrow checking
option(ENABLE_BORROW_CHECKING "Enable C++ Borrow Checking" OFF)
option(BORROW_CHECK_FATAL "Make borrow check failures fatal" OFF)
option(RUSTYCPP_SKIP_DEPENDENCY_CHECK "Skip dependency checking for RustyCpp" OFF)

# Function to check if a command exists
function(check_command_exists CMD VAR)
    execute_process(
        COMMAND which ${CMD}
        OUTPUT_VARIABLE CMD_PATH
        ERROR_QUIET
        OUTPUT_STRIP_TRAILING_WHITESPACE
    )
    if(CMD_PATH)
        set(${VAR} TRUE PARENT_SCOPE)
        message(STATUS "Found ${CMD}: ${CMD_PATH}")
    else()
        set(${VAR} FALSE PARENT_SCOPE)
    endif()
endfunction()

# Function to detect C++ standard library include paths
function(detect_cpp_include_paths)
    message(STATUS "Detecting C++ standard library include paths...")
    
    # Try to get the compiler's include paths
    if(CMAKE_CXX_COMPILER)
        # Run the compiler with -E -x c++ -v to get include paths
        execute_process(
            COMMAND ${CMAKE_CXX_COMPILER} -E -x c++ -v /dev/null
            OUTPUT_VARIABLE COMPILER_OUTPUT
            ERROR_VARIABLE COMPILER_ERROR
            OUTPUT_STRIP_TRAILING_WHITESPACE
            ERROR_STRIP_TRAILING_WHITESPACE
        )
        
        # Parse the output to find include paths
        string(REGEX MATCH "#include <\\.\\.\\.> search starts here:(.*)End of search list" INCLUDE_SECTION "${COMPILER_ERROR}")
        if(CMAKE_MATCH_1)
            string(REGEX REPLACE "\n" ";" INCLUDE_LINES "${CMAKE_MATCH_1}")
            set(CPP_INCLUDE_PATHS "")
            foreach(LINE ${INCLUDE_LINES})
                string(STRIP "${LINE}" STRIPPED_LINE)
                if(STRIPPED_LINE AND NOT STRIPPED_LINE MATCHES "^#")
                    list(APPEND CPP_INCLUDE_PATHS "${STRIPPED_LINE}")
                endif()
            endforeach()
            
            if(CPP_INCLUDE_PATHS)
                # Join paths with colons for environment variable
                string(REPLACE ";" ":" CPP_INCLUDE_PATH_STR "${CPP_INCLUDE_PATHS}")
                set(ENV{CPLUS_INCLUDE_PATH} "${CPP_INCLUDE_PATH_STR}")
                set(RUSTYCPP_CXX_INCLUDE_PATHS ${CPP_INCLUDE_PATHS} PARENT_SCOPE)
                message(STATUS "Detected C++ include paths: ${CPP_INCLUDE_PATH_STR}")
            endif()
        endif()
    endif()
    
    # Fallback: try to detect common paths based on compiler version
    if(NOT DEFINED RUSTYCPP_CXX_INCLUDE_PATHS)
        # Get GCC/G++ version if available
        execute_process(
            COMMAND ${CMAKE_CXX_COMPILER} -dumpversion
            OUTPUT_VARIABLE GCC_VERSION
            OUTPUT_STRIP_TRAILING_WHITESPACE
            ERROR_QUIET
        )
        
        if(GCC_VERSION)
            # Extract major version
            string(REGEX MATCH "^([0-9]+)" GCC_MAJOR_VERSION "${GCC_VERSION}")
            
            # Common paths for GCC
            set(FALLBACK_PATHS
                "/usr/include/c++/${GCC_MAJOR_VERSION}"
                "/usr/include/x86_64-linux-gnu/c++/${GCC_MAJOR_VERSION}"
                "/usr/include/c++/${GCC_MAJOR_VERSION}/backward"
                "/usr/include/x86_64-linux-gnu"
                "/usr/include"
            )
            
            # Check which paths exist
            set(EXISTING_PATHS "")
            foreach(PATH ${FALLBACK_PATHS})
                if(EXISTS ${PATH})
                    list(APPEND EXISTING_PATHS ${PATH})
                endif()
            endforeach()
            
            if(EXISTING_PATHS)
                string(REPLACE ";" ":" CPP_INCLUDE_PATH_STR "${EXISTING_PATHS}")
                set(ENV{CPLUS_INCLUDE_PATH} "${CPP_INCLUDE_PATH_STR}")
                set(RUSTYCPP_CXX_INCLUDE_PATHS ${EXISTING_PATHS} PARENT_SCOPE)
                message(STATUS "Using fallback C++ include paths: ${CPP_INCLUDE_PATH_STR}")
            endif()
        endif()
    endif()
    
    # Set CPATH for C headers
    set(C_PATHS "/usr/include/x86_64-linux-gnu:/usr/include")
    set(ENV{CPATH} "${C_PATHS}")
endfunction()

# Function to check for required dependencies
function(check_rustycpp_dependencies)
    message(STATUS "Checking RustyCpp dependencies...")
    
    if(RUSTYCPP_SKIP_DEPENDENCY_CHECK)
        message(STATUS "Skipping RustyCpp dependency checks")
        return()
    endif()

    # Detect C++ include paths first
    detect_cpp_include_paths()

    set(MISSING_DEPS FALSE)
    
    # Check for Rust/Cargo
    check_command_exists(cargo HAS_CARGO)
    if(NOT HAS_CARGO)
        message(WARNING "cargo not found. Please install Rust from https://rustup.rs/")
        set(MISSING_DEPS TRUE)
    endif()
    
    # Check for LLVM/Clang development libraries
    # Try to find libclang
    find_path(LIBCLANG_INCLUDE_DIR clang-c/Index.h
        PATHS
            /usr/include
            /usr/local/include
            /usr/lib/llvm-14/include
            /usr/lib/llvm-15/include
            /usr/lib/llvm-16/include
            /usr/lib/llvm/14/include
            /usr/lib/llvm/15/include
            /usr/lib/llvm/16/include
            /opt/homebrew/opt/llvm/include
        PATH_SUFFIXES
            llvm
            llvm-14
            llvm-15
            llvm-16
    )
    
    find_library(LIBCLANG_LIBRARY
        NAMES clang libclang
        PATHS
            /usr/lib
            /usr/local/lib
            /usr/lib/llvm-14/lib
            /usr/lib/llvm-15/lib
            /usr/lib/llvm-16/lib
            /usr/lib/llvm/14/lib
            /usr/lib/llvm/15/lib
            /usr/lib/llvm/16/lib
            /usr/lib/x86_64-linux-gnu
            /opt/homebrew/opt/llvm/lib
    )
    
    if(NOT LIBCLANG_INCLUDE_DIR OR NOT LIBCLANG_LIBRARY)
        message(WARNING "libclang development files not found.")
        message(WARNING "Please install LLVM/Clang development packages:")
        message(WARNING "  Ubuntu/Debian: sudo apt-get install llvm-14-dev libclang-14-dev")
        message(WARNING "  Fedora/RHEL: sudo dnf install llvm-devel clang-devel")
        message(WARNING "  macOS: brew install llvm")
        set(MISSING_DEPS TRUE)
    else()
        message(STATUS "Found libclang: ${LIBCLANG_LIBRARY}")
        message(STATUS "Found libclang headers: ${LIBCLANG_INCLUDE_DIR}")
        
        # Set environment variables for the build
        get_filename_component(LIBCLANG_DIR ${LIBCLANG_LIBRARY} DIRECTORY)
        set(ENV{LIBCLANG_PATH} "${LIBCLANG_DIR}")
        
        # Try to find llvm-config
        find_program(LLVM_CONFIG_EXECUTABLE
            NAMES llvm-config llvm-config-14 llvm-config-15 llvm-config-16
            PATHS
                /usr/bin
                /usr/local/bin
                /usr/lib/llvm-14/bin
                /usr/lib/llvm-15/bin
                /usr/lib/llvm-16/bin
                /usr/lib/llvm/14/bin
                /usr/lib/llvm/15/bin
                /usr/lib/llvm/16/bin
                /opt/homebrew/opt/llvm/bin
        )
        
        if(LLVM_CONFIG_EXECUTABLE)
            set(ENV{LLVM_CONFIG_PATH} "${LLVM_CONFIG_EXECUTABLE}")
            message(STATUS "Found llvm-config: ${LLVM_CONFIG_EXECUTABLE}")
        endif()
    endif()
    
    # Check for Z3 (required for constraint solving)
    find_path(Z3_INCLUDE_DIR z3.h
        PATHS
            /usr/include
            /usr/local/include
            /opt/homebrew/include
    )
    
    find_library(Z3_LIBRARY
        NAMES z3
        PATHS
            /usr/lib
            /usr/local/lib
            /usr/lib/x86_64-linux-gnu
            /opt/homebrew/lib
    )
    
    if(NOT Z3_INCLUDE_DIR OR NOT Z3_LIBRARY)
        message(WARNING "Z3 solver not found.")
        message(WARNING "Please install Z3 development packages:")
        message(WARNING "  Ubuntu/Debian: sudo apt-get install libz3-dev")
        message(WARNING "  Fedora/RHEL: sudo dnf install z3-devel")
        message(WARNING "  macOS: brew install z3")
        set(MISSING_DEPS TRUE)
    else()
        message(STATUS "Found Z3: ${Z3_LIBRARY}")
        message(STATUS "Found Z3 headers: ${Z3_INCLUDE_DIR}")
        set(ENV{Z3_SYS_Z3_HEADER} "${Z3_INCLUDE_DIR}/z3.h")
    endif()
    
    if(MISSING_DEPS)
        message(FATAL_ERROR "Missing required dependencies for RustyCpp. "
                "Please install them or set RUSTYCPP_SKIP_DEPENDENCY_CHECK=ON to skip this check.")
    endif()
endfunction()

# Determine the target directory and binary name based on build type
if(RUSTYCPP_BUILD_TYPE STREQUAL "release")
    set(RUSTYCPP_TARGET_DIR "${RUSTYCPP_DIR}/target/release")
    set(CARGO_BUILD_FLAGS "--release")
else()
    set(RUSTYCPP_TARGET_DIR "${RUSTYCPP_DIR}/target/debug")
    set(CARGO_BUILD_FLAGS "")
endif()

# Set the checker binary path
set(CPP_BORROW_CHECKER "${RUSTYCPP_TARGET_DIR}/rusty-cpp-checker")
if(WIN32)
    set(CPP_BORROW_CHECKER "${CPP_BORROW_CHECKER}.exe")
endif()

# Function to build the rusty-cpp-checker with proper environment
function(create_rustycpp_build_target)
    # Get the environment variables that were set during dependency check
    set(BUILD_ENV)
    
    # Check if we found libclang and set the environment
    if(DEFINED ENV{LIBCLANG_PATH})
        list(APPEND BUILD_ENV "LIBCLANG_PATH=$ENV{LIBCLANG_PATH}")
    endif()
    
    if(DEFINED ENV{LLVM_CONFIG_PATH})
        list(APPEND BUILD_ENV "LLVM_CONFIG_PATH=$ENV{LLVM_CONFIG_PATH}")
    endif()
    
    if(DEFINED ENV{Z3_SYS_Z3_HEADER})
        list(APPEND BUILD_ENV "Z3_SYS_Z3_HEADER=$ENV{Z3_SYS_Z3_HEADER}")
    endif()
    
    # Add C++ include paths if detected
    if(DEFINED ENV{CPLUS_INCLUDE_PATH})
        list(APPEND BUILD_ENV "CPLUS_INCLUDE_PATH=$ENV{CPLUS_INCLUDE_PATH}")
    endif()
    
    if(DEFINED ENV{CPATH})
        list(APPEND BUILD_ENV "CPATH=$ENV{CPATH}")
    endif()
    
    # Create a custom target to build the rusty-cpp-checker
    if(BUILD_ENV)
        add_custom_target(build_rusty_cpp_checker
            COMMAND ${CMAKE_COMMAND} -E env ${BUILD_ENV} cargo build ${CARGO_BUILD_FLAGS}
            WORKING_DIRECTORY ${RUSTYCPP_DIR}
            COMMENT "Building rusty-cpp-checker (${RUSTYCPP_BUILD_TYPE} mode) with environment: ${BUILD_ENV}"
            VERBATIM
        )
    else()
        add_custom_target(build_rusty_cpp_checker
            COMMAND cargo build ${CARGO_BUILD_FLAGS}
            WORKING_DIRECTORY ${RUSTYCPP_DIR}
            COMMENT "Building rusty-cpp-checker (${RUSTYCPP_BUILD_TYPE} mode)"
            VERBATIM
        )
    endif()
endfunction()

# Create the output directory if it doesn't exist
file(MAKE_DIRECTORY ${RUSTYCPP_TARGET_DIR})

# Mark the checker binary as a generated file
set_source_files_properties(${CPP_BORROW_CHECKER} PROPERTIES GENERATED TRUE)

# Function to ensure the checker is built before use
function(ensure_checker_built TARGET_NAME)
    if(TARGET build_rusty_cpp_checker)
        add_dependencies(${TARGET_NAME} build_rusty_cpp_checker)
    else()
        message(WARNING "RustyCpp build target not created. Checker may not be available.")
    endif()
endfunction()

# Function to add borrow checking for a single file
function(add_borrow_check SOURCE_FILE)
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
    
    # Add user-specified includes if they exist
    if(DEFINED BORROW_CHECKER_INCLUDE_PATHS)
        foreach(DIR ${BORROW_CHECKER_INCLUDE_PATHS})
            list(APPEND INCLUDE_FLAGS "-I${DIR}")
        endforeach()
    endif()
    
    # Get compile definitions from directory properties
    get_directory_property(COMPILE_DEFS COMPILE_DEFINITIONS)
    set(DEFINE_FLAGS)
    if(COMPILE_DEFS)
        foreach(DEF ${COMPILE_DEFS})
            list(APPEND DEFINE_FLAGS "-D${DEF}")
        endforeach()
    endif()
    
    # Add CONFIG_H if it's defined as a CMake variable
    if(DEFINED CONFIG_H)
        list(APPEND DEFINE_FLAGS "-DCONFIG_H=\"${CONFIG_H}\"")
    endif()
    
    # Get absolute path for the source file
    get_filename_component(SOURCE_ABS ${SOURCE_FILE} ABSOLUTE)
    
    # Set up environment for the checker
    set(CHECKER_ENV)
    if(DEFINED ENV{CPLUS_INCLUDE_PATH})
        list(APPEND CHECKER_ENV "CPLUS_INCLUDE_PATH=$ENV{CPLUS_INCLUDE_PATH}")
    endif()
    if(DEFINED ENV{CPATH})
        list(APPEND CHECKER_ENV "CPATH=$ENV{CPATH}")
    endif()
    
    # Create custom command for borrow checking
    if(CHECKER_ENV)
        add_custom_command(
            OUTPUT ${CMAKE_CURRENT_BINARY_DIR}/${CHECK_NAME}.stamp
            COMMAND ${CMAKE_COMMAND} -E env ${CHECKER_ENV} ${CPP_BORROW_CHECKER} ${SOURCE_ABS} ${INCLUDE_FLAGS} ${DEFINE_FLAGS}
            COMMAND ${CMAKE_COMMAND} -E touch ${CMAKE_CURRENT_BINARY_DIR}/${CHECK_NAME}.stamp
            DEPENDS ${SOURCE_ABS} ${CPP_BORROW_CHECKER}
            COMMENT "Borrow checking ${SOURCE_FILE}"
            VERBATIM
        )
    else()
        add_custom_command(
            OUTPUT ${CMAKE_CURRENT_BINARY_DIR}/${CHECK_NAME}.stamp
            COMMAND ${CPP_BORROW_CHECKER} ${SOURCE_ABS} ${INCLUDE_FLAGS} ${DEFINE_FLAGS}
            COMMAND ${CMAKE_COMMAND} -E touch ${CMAKE_CURRENT_BINARY_DIR}/${CHECK_NAME}.stamp
            DEPENDS ${SOURCE_ABS} ${CPP_BORROW_CHECKER}
            COMMENT "Borrow checking ${SOURCE_FILE}"
            VERBATIM
        )
    endif()
    
    # Add to custom target
    add_custom_target(${CHECK_NAME} ALL
        DEPENDS ${CMAKE_CURRENT_BINARY_DIR}/${CHECK_NAME}.stamp
    )
    
    # Ensure the checker is built first
    ensure_checker_built(${CHECK_NAME})
    
    # Make it fatal if requested
    if(BORROW_CHECK_FATAL)
        set_property(TARGET ${CHECK_NAME} PROPERTY JOB_POOL_COMPILE console)
    endif()
endfunction()

# Function to add borrow checking for a target
function(add_borrow_check_target TARGET_NAME)
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
    
    # Get compile definitions from target
    get_target_property(TARGET_COMPILE_DEFS ${TARGET_NAME} COMPILE_DEFINITIONS)
    set(DEFINE_FLAGS)
    if(TARGET_COMPILE_DEFS)
        foreach(DEF ${TARGET_COMPILE_DEFS})
            list(APPEND DEFINE_FLAGS "-D${DEF}")
        endforeach()
    endif()
    
    # Also get interface compile definitions
    get_target_property(TARGET_INTERFACE_DEFS ${TARGET_NAME} INTERFACE_COMPILE_DEFINITIONS)
    if(TARGET_INTERFACE_DEFS)
        foreach(DEF ${TARGET_INTERFACE_DEFS})
            list(APPEND DEFINE_FLAGS "-D${DEF}")
        endforeach()
    endif()
    
    # Extract -D flags from compile options (for flags added via target_compile_options)
    get_target_property(TARGET_COMPILE_OPTIONS ${TARGET_NAME} COMPILE_OPTIONS)
    if(TARGET_COMPILE_OPTIONS)
        foreach(OPT ${TARGET_COMPILE_OPTIONS})
            if(OPT MATCHES "^-D")
                list(APPEND DEFINE_FLAGS "${OPT}")
            endif()
        endforeach()
    endif()
    
    # Also check interface compile options
    get_target_property(TARGET_INTERFACE_OPTIONS ${TARGET_NAME} INTERFACE_COMPILE_OPTIONS)
    if(TARGET_INTERFACE_OPTIONS)
        foreach(OPT ${TARGET_INTERFACE_OPTIONS})
            if(OPT MATCHES "^-D")
                list(APPEND DEFINE_FLAGS "${OPT}")
            endif()
        endforeach()
    endif()
    
    # Add CONFIG_H if it's defined as a CMake variable (fallback)
    if(DEFINED CONFIG_H AND NOT DEFINE_FLAGS MATCHES "CONFIG_H")
        list(APPEND DEFINE_FLAGS "-DCONFIG_H=\"${CONFIG_H}\"")
    endif()
    
    # Create a custom target for all checks of this target
    set(ALL_CHECKS_TARGET "borrow_check_all_${TARGET_NAME}")
    add_custom_target(${ALL_CHECKS_TARGET})
    ensure_checker_built(${ALL_CHECKS_TARGET})
    
    # Add checks for each source file
    foreach(SOURCE ${SOURCES})
        # Only check .cpp files
        if(SOURCE MATCHES "\\.(cpp|cc|cxx)$")
            get_filename_component(FILE_NAME ${SOURCE} NAME_WE)
            set(CHECK_NAME "borrow_check_${TARGET_NAME}_${FILE_NAME}")
            
            # Get absolute path for the source file
            get_filename_component(SOURCE_ABS ${SOURCE} ABSOLUTE)
            
            # Set up environment for the checker
            set(CHECKER_ENV)
            if(DEFINED ENV{CPLUS_INCLUDE_PATH})
                list(APPEND CHECKER_ENV "CPLUS_INCLUDE_PATH=$ENV{CPLUS_INCLUDE_PATH}")
            endif()
            if(DEFINED ENV{CPATH})
                list(APPEND CHECKER_ENV "CPATH=$ENV{CPATH}")
            endif()
            
            if(CHECKER_ENV)
                add_custom_command(
                    OUTPUT ${CMAKE_CURRENT_BINARY_DIR}/${CHECK_NAME}.stamp
                    COMMAND ${CMAKE_COMMAND} -E env ${CHECKER_ENV} ${CPP_BORROW_CHECKER} ${SOURCE_ABS} ${INCLUDE_FLAGS} ${DEFINE_FLAGS}
                    COMMAND ${CMAKE_COMMAND} -E touch ${CMAKE_CURRENT_BINARY_DIR}/${CHECK_NAME}.stamp
                    DEPENDS ${SOURCE_ABS} ${CPP_BORROW_CHECKER}
                    COMMENT "Borrow checking ${SOURCE} (from ${TARGET_NAME})"
                    VERBATIM
                )
            else()
                add_custom_command(
                    OUTPUT ${CMAKE_CURRENT_BINARY_DIR}/${CHECK_NAME}.stamp
                    COMMAND ${CPP_BORROW_CHECKER} ${SOURCE_ABS} ${INCLUDE_FLAGS} ${DEFINE_FLAGS}
                    COMMAND ${CMAKE_COMMAND} -E touch ${CMAKE_CURRENT_BINARY_DIR}/${CHECK_NAME}.stamp
                    DEPENDS ${SOURCE_ABS} ${CPP_BORROW_CHECKER}
                    COMMENT "Borrow checking ${SOURCE} (from ${TARGET_NAME})"
                    VERBATIM
                )
            endif()
            
            # Add custom target for this check
            add_custom_target(${CHECK_NAME}
                DEPENDS ${CMAKE_CURRENT_BINARY_DIR}/${CHECK_NAME}.stamp
            )
            
            # Add to the all checks target
            add_dependencies(${ALL_CHECKS_TARGET} ${CHECK_NAME})
        endif()
    endforeach()
    
    # Make the main target depend on all checks
    add_dependencies(${TARGET_NAME} ${ALL_CHECKS_TARGET})
endfunction()

# Function to enable borrow checking globally
function(enable_borrow_checking)
    set(ENABLE_BORROW_CHECKING ON PARENT_SCOPE)
    message(STATUS "C++ Borrow Checking enabled")
    message(STATUS "Checker will be built at: ${CPP_BORROW_CHECKER}")
    message(STATUS "Build type: ${RUSTYCPP_BUILD_TYPE}")
    
    # Check dependencies now that borrow checking is enabled
    check_rustycpp_dependencies()
    
    # Create the build target after dependency check
    create_rustycpp_build_target()
endfunction()

# Create a custom target for checking all files
if(ENABLE_BORROW_CHECKING)
    add_custom_target(borrow_check_all
        COMMENT "Running borrow checker on all C++ files"
    )
    ensure_checker_built(borrow_check_all)
endif()

# Function to add compile_commands.json support
function(setup_borrow_checker_compile_commands)
    set(CMAKE_EXPORT_COMPILE_COMMANDS ON PARENT_SCOPE)
    
    # Create a custom target that runs borrow checker with compile_commands.json
    add_custom_target(borrow_check_with_compile_commands
        COMMAND ${CPP_BORROW_CHECKER} 
                --compile-commands ${CMAKE_BINARY_DIR}/compile_commands.json
                \${SOURCE_FILE}
        WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
        DEPENDS ${CPP_BORROW_CHECKER}
        COMMENT "Run: cmake --build . --target borrow_check_with_compile_commands -- SOURCE_FILE=<path/to/file.cpp>"
        VERBATIM
    )
    ensure_checker_built(borrow_check_with_compile_commands)
endfunction()

# Macro to mark files as safe
macro(mark_safe)
    foreach(FILE ${ARGN})
        set_source_files_properties(${FILE} PROPERTIES 
            COMPILE_DEFINITIONS "BORROW_CHECKER_SAFE"
        )
    endforeach()
endmacro()

# Print initial status
message(STATUS "Rusty C++ Checker submodule found at: ${RUSTYCPP_DIR}")
message(STATUS "Checker will be built on demand at: ${CPP_BORROW_CHECKER}")