# Rusty-CPP CMake Example Project

This example demonstrates how to integrate rusty-cpp into a CMake-based C++ project.

## Setup

### Step 1: Add rusty-cpp to Your Project

**Recommended: Add as a git submodule** (allows easy updates with `git submodule update`):

```bash
cd your-project
git submodule add https://github.com/shuaimu/rusty-cpp.git third-party/rusty-cpp
git submodule update --init --recursive
```

Then set the path in your CMakeLists.txt:
```cmake
set(RUSTYCPP_DIR "${CMAKE_SOURCE_DIR}/third-party/rusty-cpp")
```

### Step 2: Prerequisites

The CMake module will check for these automatically:

- **Rust/Cargo**: Install from https://rustup.rs/
- **LLVM/Clang** (libclang): (higher versions should also work)
  - Ubuntu/Debian: `sudo apt-get install llvm-16-dev libclang-16-dev`
  - macOS: `brew install llvm`
- **Z3 Solver**:
  - Ubuntu/Debian: `sudo apt-get install libz3-dev`
  - macOS: `brew install z3`

### Step 3: Build

```bash
mkdir build && cd build
cmake ..
make
```

The build will automatically:
1. Build `rusty-cpp-checker` if not already built
2. Run safety checks on all source files
3. Fail if any violations are found
4. Compile the project

## CMakeLists.txt Integration

Minimal example:

```cmake
cmake_minimum_required(VERSION 3.16)
project(myproject CXX)

set(CMAKE_CXX_STANDARD 20)

# Point to rusty-cpp (as submodule or external path)
set(RUSTYCPP_DIR "${CMAKE_SOURCE_DIR}/third-party/rusty-cpp")

# Include the CMake module
include(${RUSTYCPP_DIR}/cmake/RustyCppSubmodule.cmake)

# Enable borrow checking
enable_borrow_checking()

# Your target
add_executable(myapp src/main.cpp src/utils.cpp)

# Add rusty:: types (Box, Arc, Vec, etc.)
target_include_directories(myapp PRIVATE ${RUSTYCPP_DIR}/include)

# Enable checking for this target
add_borrow_check_target(myapp)
```

## Configuration Options

```bash
# Disable borrow checking
cmake -DENABLE_BORROW_CHECKING=OFF ..

# Use debug build of rusty-cpp (slower but more debug info)
cmake -DRUSTYCPP_BUILD_TYPE=debug ..

# Make borrow check failures fatal (stop on first error)
cmake -DBORROW_CHECK_FATAL=ON ..
```

## Keeping rusty-cpp Updated

Since rusty-cpp is rapidly evolving, update your submodule frequently:

```bash
cd third-party/rusty-cpp
git pull origin main
cd ../..
git add third-party/rusty-cpp
git commit -m "Update rusty-cpp"
```

Or update all submodules at once:
```bash
git submodule update --remote --merge
```

## Available CMake Functions

| Function | Description |
|----------|-------------|
| `enable_borrow_checking()` | Enable checking, verify dependencies, create build target |
| `add_borrow_check_target(target)` | Add checks for all sources in a target |
| `add_borrow_check(file.cpp)` | Add check for a single file |

## Example Annotations

See `src/safe_example.cpp` for examples of:
- `@safe` function annotations
- `@unsafe` blocks for STL operations
- Borrow checking patterns
- Scope-based lifetime management
