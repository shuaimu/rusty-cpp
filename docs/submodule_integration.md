# Using RustyCpp as a Git Submodule

This guide explains how to integrate rusty-cpp-checker into your C++ project as a git submodule, without requiring global installation.

## Quick Start

### 1. Add RustyCpp as a Submodule

In your project's root directory:

```bash
# Add the submodule
git submodule add https://github.com/shuaimu/rustycpp external/rustycpp

# Initialize and update the submodule
git submodule update --init --recursive
```

### 2. Update Your CMakeLists.txt

Add the following to your project's main `CMakeLists.txt`:

```cmake
cmake_minimum_required(VERSION 3.10)
project(YourProject CXX)

# Include the submodule CMake integration
include(${CMAKE_CURRENT_SOURCE_DIR}/external/rustycpp/cmake/RustyCppSubmodule.cmake)

# Configure and enable borrow checking
set(RUSTYCPP_BUILD_TYPE "release")  # or "debug"
set(ENABLE_BORROW_CHECKING ON)
enable_borrow_checking()

# Your existing project configuration...
add_executable(your_app src/main.cpp)

# Enable borrow checking for your targets
add_borrow_check_target(your_app)
```

### 3. Build Your Project

```bash
mkdir build
cd build
cmake ..
cmake --build .
```

The rusty-cpp-checker will be automatically built as part of your project's build process.

## Detailed Configuration

### Build Options

```cmake
# Choose between debug and release builds of the checker
set(RUSTYCPP_BUILD_TYPE "release")  # Slower to build, faster to run
# or
set(RUSTYCPP_BUILD_TYPE "debug")    # Faster to build, slower to run

# Make borrow check failures stop the build
set(BORROW_CHECK_FATAL ON)

# Control when checking happens
set(ENABLE_BORROW_CHECKING ON)  # or OFF to temporarily disable
```

### Selective Checking

You have three levels of control:

#### 1. Check Entire Targets
```cmake
add_executable(my_app src/main.cpp src/utils.cpp)
add_borrow_check_target(my_app)  # Checks all source files in my_app
```

#### 2. Check Specific Files
```cmake
add_borrow_check(src/critical_safety.cpp)  # Only check this file
add_borrow_check(src/memory_manager.cpp)   # And this one
```

#### 3. Mark Files as Safe
```cmake
mark_safe(src/legacy_code.cpp src/third_party.cpp)  # Skip these files
```

### Include Paths

The integration automatically detects include paths from your CMake targets. For additional paths:

```cmake
# These will be passed to the borrow checker
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/include)
include_directories(${THIRD_PARTY_INCLUDES})

# Or set them specifically for the checker
set(BORROW_CHECKER_INCLUDE_PATHS 
    "${CMAKE_CURRENT_SOURCE_DIR}/custom_includes"
    "${EXTERNAL_LIBRARY_PATH}/include"
)
```

### Using compile_commands.json

For the most accurate include path resolution:

```cmake
# Enable compile_commands.json generation
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
setup_borrow_checker_compile_commands()

# Then you can run checks with full compilation context
# cmake --build . --target borrow_check_with_compile_commands -- SOURCE_FILE=src/main.cpp
```

## Complete Example

Here's a full example of a project using rustycpp as a submodule:

```cmake
cmake_minimum_required(VERSION 3.10)
project(SafeCppProject CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Include rustycpp submodule
include(${CMAKE_CURRENT_SOURCE_DIR}/external/rustycpp/cmake/RustyCppSubmodule.cmake)

# Configure borrow checker
set(RUSTYCPP_BUILD_TYPE "release")
set(ENABLE_BORROW_CHECKING ON)
set(BORROW_CHECK_FATAL OFF)  # Warnings only during development

enable_borrow_checking()

# Enable compile_commands.json
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
setup_borrow_checker_compile_commands()

# Project includes
include_directories(include)

# Library with safety checking
add_library(safe_core STATIC
    src/memory/allocator.cpp
    src/memory/pool.cpp
    src/containers/safe_vector.cpp
)
add_borrow_check_target(safe_core)

# Main application
add_executable(app
    src/main.cpp
    src/application.cpp
    src/config.cpp
)
target_link_libraries(app safe_core)

# Only check critical files
add_borrow_check(src/main.cpp)
add_borrow_check(src/application.cpp)
# Skip config.cpp as it's just data parsing

# Tests with checking
if(BUILD_TESTS)
    add_executable(tests
        tests/test_main.cpp
        tests/test_allocator.cpp
        tests/test_containers.cpp
    )
    target_link_libraries(tests safe_core)
    add_borrow_check_target(tests)
endif()

# Custom target for manual checking
add_custom_target(safety_check
    COMMAND ${CMAKE_COMMAND} --build . --target borrow_check_all
    COMMENT "Running all safety checks"
)
```

## Project Structure

Your project structure should look like:

```
your-project/
├── CMakeLists.txt          # Your main CMake file
├── external/
│   └── rustycpp/           # Submodule
│       ├── src/            # Checker source (Rust)
│       ├── cmake/          # CMake integration files
│       └── Cargo.toml      # Rust project file
├── src/
│   └── *.cpp               # Your C++ source files
└── include/
    └── *.h                 # Your header files
```

## Continuous Integration

For CI/CD pipelines, ensure Rust is installed:

```yaml
# GitHub Actions example
- name: Install Rust
  uses: actions-rs/toolchain@v1
  with:
    toolchain: stable

- name: Configure CMake
  run: cmake -B build -DENABLE_BORROW_CHECKING=ON

- name: Build and Check
  run: cmake --build build --target all
```

## Troubleshooting

### Checker Not Building

Ensure you have Rust installed:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Slow First Build

The first build compiles the Rust checker and its dependencies. Subsequent builds are much faster. Use `RUSTYCPP_BUILD_TYPE=debug` during development for faster iteration.

### Include Path Issues

1. Check that `CMAKE_EXPORT_COMPILE_COMMANDS` is ON
2. Use absolute paths in `include_directories()`
3. Run with verbose output: `add_borrow_check(src/file.cpp VERBOSE)`

### Submodule Out of Date

Update the submodule to the latest version:
```bash
cd external/rustycpp
git pull origin main
cd ../..
git add external/rustycpp
git commit -m "Update rustycpp submodule"
```

## Advanced Usage

### Custom Build Directory

If your Rust target directory is elsewhere:

```cmake
set(RUSTYCPP_TARGET_DIR "${CMAKE_BINARY_DIR}/rust_build")
```

### Cross-compilation

For cross-compilation, you may need to set Rust target:

```cmake
set(CARGO_BUILD_FLAGS "--release --target x86_64-unknown-linux-gnu")
```

### Integration with IDE

Most IDEs that support CMake will automatically recognize the borrow checking targets. In VS Code or CLion, you can run specific check targets from the CMake panel.

## Benefits of Submodule Integration

1. **No Global Installation**: Everything is self-contained in your project
2. **Version Control**: Lock to specific versions of the checker
3. **Reproducible Builds**: Team members get the exact same checker version
4. **CI/CD Friendly**: No need to pre-install tools on build servers
5. **Gradual Adoption**: Enable checking file-by-file or target-by-target

## Migration from Global Installation

If you're currently using a globally installed rusty-cpp-checker:

1. Remove global installation references from CMakeLists.txt
2. Add the submodule as shown above
3. Replace `include(CppBorrowChecker.cmake)` with `include(path/to/RustyCppSubmodule.cmake)`
4. The rest of your CMake code remains the same

## Support

For issues or questions about submodule integration, please open an issue at:
https://github.com/shuaimu/rustycpp/issues