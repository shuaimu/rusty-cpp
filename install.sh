#!/bin/bash

# Rusty-CPP Installation Script
# This script builds and installs Rusty-CPP for use with CMake and other build systems

set -e  # Exit on error

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
INSTALL_PREFIX="${INSTALL_PREFIX:-/usr/local}"
BUILD_TYPE="${BUILD_TYPE:-release}"

echo "Rusty-CPP Installation"
echo "=============================="
echo "Install prefix: $INSTALL_PREFIX"
echo "Build type: $BUILD_TYPE"
echo

# Check for required tools
check_requirement() {
    if ! command -v "$1" &> /dev/null; then
        echo "Error: $1 is required but not installed."
        echo "Please install $1 and try again."
        exit 1
    fi
}

check_requirement cargo
check_requirement rustc

# Platform detection
PLATFORM=$(uname -s)
case "$PLATFORM" in
    Darwin*)
        echo "Detected macOS"
        # Check for Homebrew packages
        if ! brew list llvm &> /dev/null; then
            echo "Warning: LLVM not found. Install with: brew install llvm"
        fi
        if ! brew list z3 &> /dev/null; then
            echo "Warning: Z3 not found. Install with: brew install z3"
        fi
        
        # Set environment variables for macOS
        export Z3_SYS_Z3_HEADER="/opt/homebrew/include/z3.h"
        if [ ! -f "$Z3_SYS_Z3_HEADER" ]; then
            # Try alternative location
            export Z3_SYS_Z3_HEADER="/usr/local/include/z3.h"
        fi
        ;;
    Linux*)
        echo "Detected Linux"
        # Check for required packages
        if ! ldconfig -p | grep -q libclang; then
            echo "Warning: libclang not found. Install with:"
            echo "  Ubuntu/Debian: sudo apt-get install libclang-dev"
            echo "  Fedora: sudo dnf install clang-devel"
        fi
        if ! ldconfig -p | grep -q libz3; then
            echo "Warning: Z3 not found. Install with:"
            echo "  Ubuntu/Debian: sudo apt-get install libz3-dev"
            echo "  Fedora: sudo dnf install z3-devel"
        fi
        
        # Set environment variables for Linux
        export Z3_SYS_Z3_HEADER="/usr/include/z3.h"
        ;;
    *)
        echo "Warning: Unknown platform $PLATFORM"
        ;;
esac

# Build the project
echo
echo "Building Rusty-CPP..."
cd "$SCRIPT_DIR"

if [ "$BUILD_TYPE" = "release" ]; then
    cargo build --release
    BINARY_PATH="target/release/rusty-cpp-checker"
else
    cargo build
    BINARY_PATH="target/debug/rusty-cpp-checker"
fi

if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Build failed. Binary not found at $BINARY_PATH"
    exit 1
fi

echo "Build successful!"

# Install the binary
echo
echo "Installing binary to $INSTALL_PREFIX/bin..."
if [ -w "$INSTALL_PREFIX/bin" ]; then
    cp "$BINARY_PATH" "$INSTALL_PREFIX/bin/"
else
    echo "Need sudo access to install to $INSTALL_PREFIX/bin"
    sudo cp "$BINARY_PATH" "$INSTALL_PREFIX/bin/"
fi

# Install CMake module
CMAKE_MODULE_DIR="$INSTALL_PREFIX/share/cmake/Modules"
echo "Installing CMake module to $CMAKE_MODULE_DIR..."

if [ -w "$CMAKE_MODULE_DIR" ] || [ -w "$(dirname "$CMAKE_MODULE_DIR")" ]; then
    mkdir -p "$CMAKE_MODULE_DIR"
    cp "$SCRIPT_DIR/cmake/CppBorrowChecker.cmake" "$CMAKE_MODULE_DIR/"
else
    echo "Need sudo access to install CMake module"
    sudo mkdir -p "$CMAKE_MODULE_DIR"
    sudo cp "$SCRIPT_DIR/cmake/CppBorrowChecker.cmake" "$CMAKE_MODULE_DIR/"
fi

# Verify installation
echo
echo "Verifying installation..."
if command -v rusty-cpp-checker &> /dev/null; then
    echo "✓ Binary installed successfully"
    echo "  Version: $(rusty-cpp-checker --version 2>&1 || echo 'version info not available')"
else
    echo "✗ Binary not found in PATH"
    echo "  Make sure $INSTALL_PREFIX/bin is in your PATH"
fi

if [ -f "$CMAKE_MODULE_DIR/CppBorrowChecker.cmake" ]; then
    echo "✓ CMake module installed successfully"
else
    echo "✗ CMake module not found"
fi

# Print usage instructions
echo
echo "Installation complete!"
echo
echo "To use with CMake:"
echo "  1. Add to your CMakeLists.txt:"
echo "     include(CppBorrowChecker)"
echo "     enable_borrow_checking()"
echo
echo "  2. Or for specific targets:"
echo "     add_borrow_check_target(your_target)"
echo
echo "To use standalone:"
echo "  rusty-cpp-checker your_file.cpp"
echo
echo "To uninstall:"
echo "  rm $INSTALL_PREFIX/bin/rusty-cpp-checker"
echo "  rm $CMAKE_MODULE_DIR/CppBorrowChecker.cmake"
echo
echo "For more information, see: $SCRIPT_DIR/README.md"