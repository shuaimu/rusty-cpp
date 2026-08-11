cmake_minimum_required(VERSION 4.4)

# This gate intentionally exercises a real libc++ `import std;` consumer.
# CMake 4.4 keeps import-std materialization behind this versioned opt-in.
set(CMAKE_EXPERIMENTAL_CXX_IMPORT_STD
    "f35a9ac6-8463-4d38-8eec-5d6008153e7d")
set(CMAKE_CXX_FLAGS_INIT "${CMAKE_CXX_FLAGS_INIT} -stdlib=libc++")
set(CMAKE_EXE_LINKER_FLAGS_INIT
    "${CMAKE_EXE_LINKER_FLAGS_INIT} -stdlib=libc++")
project(rusty_cpp_abi_core LANGUAGES CXX)

if(NOT DEFINED RUSTY_CPP_SOURCE_DIR OR
   NOT DEFINED CPP_ABI_GENERATED_MODULE OR
   NOT DEFINED CPP_ABI_INLINE_MODULE)
    message(FATAL_ERROR
        "RUSTY_CPP_SOURCE_DIR, CPP_ABI_GENERATED_MODULE, and CPP_ABI_INLINE_MODULE are required")
endif()

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

if(NOT CMAKE_CXX_COMPILER_ID MATCHES "Clang" OR
   CMAKE_CXX_COMPILER_VERSION VERSION_LESS 22)
    message(FATAL_ERROR "cpp_abi integration gate requires Clang 22 or newer")
endif()

add_compile_options($<$<COMPILE_LANGUAGE:CXX>:-stdlib=libc++>)
add_link_options(-stdlib=libc++ -lc++abi)

# Linuxbrew's libc++ is not necessarily on the dynamic loader's default path.
get_filename_component(CPP_ABI_CLANG_BINDIR "${CMAKE_CXX_COMPILER}" DIRECTORY)
get_filename_component(CPP_ABI_CLANG_ROOT "${CPP_ABI_CLANG_BINDIR}" DIRECTORY)
if(EXISTS "${CPP_ABI_CLANG_ROOT}/lib/libc++.so")
    add_link_options(
        "-L${CPP_ABI_CLANG_ROOT}/lib"
        "-Wl,-rpath,${CPP_ABI_CLANG_ROOT}/lib")
endif()

add_subdirectory("${RUSTY_CPP_SOURCE_DIR}" rusty-cpp-runtime EXCLUDE_FROM_ALL)

add_library(cpp_abi_core_module STATIC)
get_filename_component(CPP_ABI_GENERATED_DIR "${CPP_ABI_GENERATED_MODULE}" DIRECTORY)
target_sources(cpp_abi_core_module
    PUBLIC FILE_SET CXX_MODULES
    BASE_DIRS "${CPP_ABI_GENERATED_DIR}"
    FILES "${CPP_ABI_GENERATED_MODULE}")
target_compile_options(cpp_abi_core_module PRIVATE -O0)
target_link_libraries(cpp_abi_core_module PUBLIC rusty)

add_executable(cpp_abi_core_runtime
    "${RUSTY_CPP_SOURCE_DIR}/transpiler/tests/fixtures/cpp_abi_core_main.cpp")
target_link_libraries(cpp_abi_core_runtime PRIVATE cpp_abi_core_module)

add_library(cpp_abi_inline_module STATIC)
get_filename_component(CPP_ABI_INLINE_DIR "${CPP_ABI_INLINE_MODULE}" DIRECTORY)
target_sources(cpp_abi_inline_module
    PUBLIC FILE_SET CXX_MODULES
    BASE_DIRS "${CPP_ABI_INLINE_DIR}"
    FILES "${CPP_ABI_INLINE_MODULE}")
target_compile_options(cpp_abi_inline_module PRIVATE -O0)
target_link_libraries(cpp_abi_inline_module PUBLIC rusty)
set_property(TARGET cpp_abi_inline_module PROPERTY CXX_MODULE_STD ON)

add_executable(cpp_abi_inline_runtime
    "${RUSTY_CPP_SOURCE_DIR}/transpiler/tests/fixtures/cpp_abi_inline_main.cpp")
target_link_libraries(cpp_abi_inline_runtime PRIVATE cpp_abi_inline_module)
set_property(TARGET cpp_abi_inline_runtime PROPERTY CXX_MODULE_STD ON)

enable_testing()
add_test(NAME cpp_abi_core_runtime COMMAND cpp_abi_core_runtime)
add_test(NAME cpp_abi_inline_runtime COMMAND cpp_abi_inline_runtime)
