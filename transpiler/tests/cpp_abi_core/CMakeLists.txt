cmake_minimum_required(VERSION 3.28)
project(rusty_cpp_abi_core LANGUAGES CXX)

if(NOT DEFINED RUSTY_CPP_SOURCE_DIR OR NOT DEFINED CPP_ABI_GENERATED_MODULE)
    message(FATAL_ERROR "RUSTY_CPP_SOURCE_DIR and CPP_ABI_GENERATED_MODULE are required")
endif()

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

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

enable_testing()
add_test(NAME cpp_abi_core_runtime COMMAND cpp_abi_core_runtime)
