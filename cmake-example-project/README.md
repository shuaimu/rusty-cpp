# Rusty-CPP CMake Example Project

This example demonstrates how to integrate rusty-cpp into a CMake-based C++ project.

## Prerequisites

1. Build rusty-cpp:
   ```bash
   cd /path/to/rusty-cpp
   cargo build --release
   ```

2. CMake 3.16 or later
3. A C++20 compatible compiler

## Building

```bash
# Create build directory
mkdir build && cd build

# Configure (rusty-cpp will be auto-detected from ../target/release/)
cmake ..

# Build (this will run rusty-cpp checks automatically)
make
```

## How It Works

The CMake configuration:

1. **Generates `compile_commands.json`** - Required by rusty-cpp for include paths
2. **Finds rusty-cpp executable** - Searches in common locations
3. **Adds custom build targets** - Runs rusty-cpp before compilation
4. **Fails the build on violations** - Safety errors stop the build

## Configuration Options

```bash
# Disable rusty-cpp checks
cmake -DENABLE_RUSTY_CPP=OFF ..

# Specify custom rusty-cpp path
cmake -DRUSTY_CPP_PATH=/path/to/rusty-cpp/target/release ..
```

## Manual Checking

Run rusty-cpp on all files without building:

```bash
make rusty_check_all
```

## Adding New Source Files

1. Add the file to the `SOURCES` list in `CMakeLists.txt`
2. The rusty-cpp check is automatically added

## Example Annotations

See `src/safe_example.cpp` for examples of:
- `@safe` function annotations
- `@unsafe` blocks for STL operations
- Borrow checking patterns
- Move detection
