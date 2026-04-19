# CMake Integration for Rusty C++ Checker

This directory contains CMake modules for integrating the Rusty C++ Checker into your CMake-based projects. We provide two integration methods:

1. **Global Installation** (`CppBorrowChecker.cmake`) - Traditional approach with system-wide installation
2. **Submodule Integration** (`RustyCppSubmodule.cmake`) - Self-contained approach without global installation

It also includes a transpiler helper module:

3. **Transpiler Integration** (`RustyCppTranspiler.cmake`) - Build-graph Rust->C++ transpile targets

## Integration Methods

### Method 1: Global Installation

Use this when you want to install the checker system-wide and use it across multiple projects.

#### Installation

```bash
# From the project root
./install.sh
```

This will:
- Build the borrow checker binary
- Install it to `/usr/local/bin` (or `$INSTALL_PREFIX/bin`)
- Install the CMake module to `/usr/local/share/cmake/Modules`

#### Usage

In your `CMakeLists.txt`:

```cmake
# Include the module
include(CppBorrowChecker)

# Enable globally for all targets
enable_borrow_checking()

# Add your targets
add_executable(my_app main.cpp)

# The borrow checker will run automatically during build
```

### Method 2: Git Submodule Integration (Recommended)

Use this for self-contained projects where the checker is part of your repository. No global installation required!

#### Setup

```bash
# In your project root
git submodule add https://github.com/shuaimu/rustycpp external/rustycpp
git submodule update --init --recursive
```

#### Usage

In your `CMakeLists.txt`:

```cmake
# Include the submodule integration
include(${CMAKE_CURRENT_SOURCE_DIR}/external/rustycpp/cmake/RustyCppSubmodule.cmake)

# Configure and enable
set(RUSTYCPP_BUILD_TYPE "release")  # or "debug"
enable_borrow_checking()

# Add your targets
add_executable(my_app main.cpp)
add_borrow_check_target(my_app)

# The checker will be built automatically as part of your project
```

## Choosing Between Methods

| Aspect | Global Installation | Submodule Integration |
|--------|--------------------|-----------------------|
| **Installation** | Once per system | None (automatic) |
| **Version Control** | System-wide version | Per-project version |
| **CI/CD** | Requires pre-installation | Self-contained |
| **Team Setup** | Each member installs | Automatic via git |
| **Build Time** | Faster (pre-built) | Slower first build |
| **Reproducibility** | Depends on system | Fully reproducible |
| **Best For** | Personal development | Team projects |

## Common Configuration Options

Both methods support the same configuration options:

```cmake
# Enable/disable checking (default: OFF)
set(ENABLE_BORROW_CHECKING ON)

# Make check failures stop the build (default: OFF)
set(BORROW_CHECK_FATAL ON)

# Additional include paths for the checker
set(BORROW_CHECKER_INCLUDE_PATHS 
    "/usr/local/include/boost"
    "/opt/custom/include"
)
```

## Transpiler Target Integration

Use `RustyCppTranspiler.cmake` when you want CMake targets that generate `.cppm` output from Rust sources:

```cmake
include(${RUSTYCPP_DIR}/cmake/RustyCppTranspiler.cmake)

rustycpp_add_transpile_target(
    my_bridge_transpile
    SOURCE ${CMAKE_CURRENT_SOURCE_DIR}/src/bridge.rs
    OUTPUT ${CMAKE_CURRENT_BINARY_DIR}/generated/my.bridge.cppm
    MODULE_NAME my.bridge
    CPP_MODULE_INDEX ${CMAKE_CURRENT_SOURCE_DIR}/src/cpp_module_index.toml
)
```

## Checking Strategies

### 1. Check Everything

```cmake
enable_borrow_checking()
# All targets will be checked automatically
```

### 2. Check Specific Targets

```cmake
set(ENABLE_BORROW_CHECKING ON)

add_executable(my_app main.cpp utils.cpp)
add_borrow_check_target(my_app)  # Only check this target
```

### 3. Check Specific Files

```cmake
set(ENABLE_BORROW_CHECKING ON)

# Check only critical files
add_borrow_check(src/critical.cpp)
add_borrow_check(src/memory_sensitive.cpp)
# src/legacy.cpp is not checked
```

## Gradual Adoption Strategy

The borrow checker is designed for gradual adoption in existing codebases:

### Step 1: Start with Critical Files

```cmake
# Check only the most critical files first
add_borrow_check(src/memory_pool.cpp)
add_borrow_check(src/resource_manager.cpp)
```

### Step 2: Add Safety Annotations

In your C++ code:
```cpp
// @safe
namespace critical {
    // This namespace will be checked
    void process_data();
}

// Legacy code remains unchecked by default
void legacy_function() {
    // Not checked unless explicitly marked @safe
}
```

### Step 3: Expand Coverage

```cmake
# Gradually add more files
add_borrow_check_target(core_library)
# Then later...
add_borrow_check_target(network_module)
```

### Step 4: Enable Globally

```cmake
# When ready, enable for everything
enable_borrow_checking()
set(BORROW_CHECK_FATAL ON)  # Make it enforced
```

## Advanced Usage

### With compile_commands.json

Both methods support using compile_commands.json for better include path handling:

```cmake
# Enable compile_commands.json generation
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
setup_borrow_checker_compile_commands()
```

### Custom Check Target

```cmake
# Create a target that checks all files
add_custom_target(check_all
    COMMAND ${CPP_BORROW_CHECKER}
            ${CMAKE_CURRENT_SOURCE_DIR}/src/*.cpp
            -I${CMAKE_CURRENT_SOURCE_DIR}/include
    COMMENT "Running borrow checker on all files"
)
```

### Integration with CTest

```cmake
enable_testing()

# Add borrow checking as a test
add_test(NAME borrow_check
    COMMAND ${CPP_BORROW_CHECKER}
            ${CMAKE_CURRENT_SOURCE_DIR}/src/main.cpp
)
```

### Conditional Checking

```cmake
# Only enable in Debug builds
if(CMAKE_BUILD_TYPE STREQUAL "Debug")
    enable_borrow_checking()
endif()

# Or as an option
option(ENABLE_SAFETY_CHECKS "Enable borrow checking" OFF)
if(ENABLE_SAFETY_CHECKS)
    enable_borrow_checking()
endif()
```

## Example Project Structures

### With Global Installation

```
my_project/
├── CMakeLists.txt          # Includes CppBorrowChecker
├── include/
│   └── my_lib.h
├── src/
│   ├── main.cpp           # @safe functions
│   └── utils.cpp
└── build/
```

### With Submodule

```
my_project/
├── CMakeLists.txt          # Includes RustyCppSubmodule
├── external/
│   └── rustycpp/           # Git submodule
│       ├── src/
│       ├── cmake/
│       └── Cargo.toml
├── include/
│   └── my_lib.h
├── src/
│   ├── main.cpp
│   └── utils.cpp
└── build/
```

## CI/CD Integration

### GitHub Actions with Submodule

```yaml
- name: Checkout with submodules
  uses: actions/checkout@v2
  with:
    submodules: recursive

- name: Install Rust
  uses: actions-rs/toolchain@v1
  with:
    toolchain: stable

- name: Configure and Build
  run: |
    cmake -B build -DENABLE_BORROW_CHECKING=ON
    cmake --build build
```

### GitHub Actions with Global Installation

```yaml
- name: Install Borrow Checker
  run: |
    git clone https://github.com/shuaimu/rustycpp
    cd rustycpp
    ./install.sh

- name: Configure and Build
  run: |
    cmake -B build -DENABLE_BORROW_CHECKING=ON
    cmake --build build
```

## Troubleshooting

### Checker Not Found (Global Installation)

```cmake
# Explicitly set the path
set(CPP_BORROW_CHECKER "/path/to/rusty-cpp-checker")
include(CppBorrowChecker)
```

### Slow First Build (Submodule)

The first build compiles the Rust checker. Use `debug` mode for faster builds during development:

```cmake
set(RUSTYCPP_BUILD_TYPE "debug")
```

### Include Path Issues

Both methods support explicit include paths:

```cmake
set(BORROW_CHECKER_INCLUDE_PATHS
    ${CMAKE_CURRENT_SOURCE_DIR}/include
    ${CMAKE_CURRENT_SOURCE_DIR}/third_party
    /usr/include/c++/11
)
```

### Submodule Out of Date

```bash
cd external/rustycpp
git pull origin main
cd ../..
git add external/rustycpp
git commit -m "Update rustycpp submodule"
```

## Environment Variables

The checker respects standard C++ include path environment variables:
- `CPLUS_INCLUDE_PATH` - C++ include directories
- `C_INCLUDE_PATH` - C include directories
- `CPATH` - Both C and C++ includes

## Performance Tips

### Parallel Checking

```cmake
# Run checks in parallel
set(CMAKE_JOB_POOLS check_pool=4)
set_property(GLOBAL PROPERTY JOB_POOLS check_pool=4)
```

### Caching (with ccache)

```cmake
find_program(CCACHE_PROGRAM ccache)
if(CCACHE_PROGRAM)
    set(CMAKE_CXX_COMPILER_LAUNCHER ${CCACHE_PROGRAM})
endif()
```

## Module Files

- **`CppBorrowChecker.cmake`** - Global installation integration
- **`RustyCppSubmodule.cmake`** - Submodule integration (no installation required)

## Further Documentation

- [Submodule Integration Guide](../docs/SUBMODULE_INTEGRATION.md) - Detailed submodule setup
- [Main Project README](../README.md) - Project overview and features
- [Examples](../examples/) - Sample projects using both integration methods

## License

Same as the Rusty C++ Checker project (MIT).
