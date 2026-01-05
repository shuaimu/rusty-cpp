#!/bin/bash
#
# rusty-cpp installer
#
# This script installs system dependencies and builds rusty-cpp from source.
#
# Supported platforms:
#   - macOS (via Homebrew)
#   - Debian/Ubuntu (apt)
#   - Fedora/CentOS/RHEL (dnf/yum)
#   - Arch Linux (pacman)
#
# Usage:
#   curl -sSL https://raw.githubusercontent.com/shuaimu/rusty-cpp/main/install.sh | bash
#
# Or clone and run locally:
#   git clone https://github.com/shuaimu/rusty-cpp
#   cd rusty-cpp
#   ./install.sh
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_banner() {
    echo -e "${BLUE}"
    echo "╔═══════════════════════════════════════════════════════════════╗"
    echo "║                    rusty-cpp installer                        ║"
    echo "║     Rust's borrow checker rules applied to C++ code           ║"
    echo "╚═══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# Detect OS and distribution
detect_os() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        OS="macos"
        return
    fi

    if [[ -f /etc/os-release ]]; then
        . /etc/os-release
        case "$ID" in
            ubuntu|debian|linuxmint|pop)
                OS="debian"
                ;;
            fedora)
                OS="fedora"
                ;;
            centos|rhel|rocky|almalinux)
                OS="centos"
                ;;
            arch|manjaro|endeavouros)
                OS="arch"
                ;;
            *)
                # Check for derivatives
                if [[ "$ID_LIKE" == *"debian"* ]]; then
                    OS="debian"
                elif [[ "$ID_LIKE" == *"fedora"* ]] || [[ "$ID_LIKE" == *"rhel"* ]]; then
                    OS="fedora"
                elif [[ "$ID_LIKE" == *"arch"* ]]; then
                    OS="arch"
                else
                    OS="unknown"
                fi
                ;;
        esac
    else
        OS="unknown"
    fi
}

# Check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check if running as root (for package installation)
check_sudo() {
    if [[ $EUID -eq 0 ]]; then
        SUDO=""
    elif command_exists sudo; then
        SUDO="sudo"
    else
        error "This script requires sudo privileges to install packages"
    fi
}

# Install dependencies on macOS
install_macos() {
    info "Detected macOS"

    if ! command_exists brew; then
        error "Homebrew is required. Install it from https://brew.sh"
    fi

    info "Installing LLVM and Z3 via Homebrew..."
    brew install llvm z3

    # Set up environment for clang and z3
    LLVM_PREFIX=$(brew --prefix llvm)
    Z3_PREFIX=$(brew --prefix z3)
    export LLVM_CONFIG_PATH="$LLVM_PREFIX/bin/llvm-config"
    export LIBCLANG_PATH="$LLVM_PREFIX/lib"
    export Z3_SYS_Z3_HEADER="$Z3_PREFIX/include/z3.h"

    success "Dependencies installed"

    echo ""
    warn "Add these to your shell profile (~/.zshrc or ~/.bashrc):"
    echo "  export LLVM_CONFIG_PATH=\"$LLVM_PREFIX/bin/llvm-config\""
    echo "  export LIBCLANG_PATH=\"$LLVM_PREFIX/lib\""
    echo "  export Z3_SYS_Z3_HEADER=\"$Z3_PREFIX/include/z3.h\""
    echo ""
}

# Install dependencies on Debian/Ubuntu
install_debian() {
    info "Detected Debian/Ubuntu-based system"
    check_sudo

    info "Updating package lists..."
    $SUDO apt-get update -qq

    # Try to find the best available LLVM version (minimum 16)
    LLVM_VERSION=""
    for v in 19 18 17 16; do
        if apt-cache show "libclang-${v}-dev" >/dev/null 2>&1; then
            LLVM_VERSION=$v
            break
        fi
    done

    if [[ -z "$LLVM_VERSION" ]]; then
        error "Could not find a suitable LLVM version (16+). Please install LLVM 16 or later manually.

  On Ubuntu 22.04+, you can add the LLVM apt repository:
    wget https://apt.llvm.org/llvm.sh
    chmod +x llvm.sh
    sudo ./llvm.sh 16"
    fi

    info "Installing LLVM $LLVM_VERSION and Z3..."
    $SUDO apt-get install -y \
        llvm-${LLVM_VERSION}-dev \
        libclang-${LLVM_VERSION}-dev \
        clang-${LLVM_VERSION} \
        libz3-dev \
        pkg-config \
        build-essential

    # Set up environment
    export LLVM_CONFIG_PATH="/usr/bin/llvm-config-${LLVM_VERSION}"
    export LIBCLANG_PATH="/usr/lib/llvm-${LLVM_VERSION}/lib"

    success "Dependencies installed (LLVM $LLVM_VERSION)"
}

# Install dependencies on Fedora
install_fedora() {
    info "Detected Fedora"
    check_sudo

    info "Installing LLVM and Z3..."
    $SUDO dnf install -y \
        llvm-devel \
        clang-devel \
        clang-libs \
        z3-devel \
        pkg-config \
        gcc \
        gcc-c++

    success "Dependencies installed"
}

# Install dependencies on CentOS/RHEL
install_centos() {
    info "Detected CentOS/RHEL"
    check_sudo

    # Check CentOS version
    if [[ -f /etc/centos-release ]]; then
        CENTOS_VERSION=$(rpm -E %{rhel})
    else
        CENTOS_VERSION=$(rpm -E %{rhel} 2>/dev/null || echo "8")
    fi

    if [[ "$CENTOS_VERSION" -ge 8 ]]; then
        info "Enabling PowerTools/CRB repository..."
        $SUDO dnf install -y epel-release
        # CentOS 8 Stream / RHEL 8
        $SUDO dnf config-manager --set-enabled powertools 2>/dev/null || \
        $SUDO dnf config-manager --set-enabled crb 2>/dev/null || \
        warn "Could not enable PowerTools/CRB repo"

        info "Installing LLVM and Z3..."
        $SUDO dnf install -y \
            llvm-devel \
            clang-devel \
            clang-libs \
            z3-devel \
            pkg-config \
            gcc \
            gcc-c++
    else
        error "CentOS/RHEL 7 or earlier is not supported. Please upgrade to version 8+."
    fi

    success "Dependencies installed"
}

# Install dependencies on Arch Linux
install_arch() {
    info "Detected Arch Linux"
    check_sudo

    info "Installing LLVM and Z3..."
    $SUDO pacman -Sy --needed --noconfirm \
        llvm \
        clang \
        z3 \
        pkgconf \
        base-devel

    success "Dependencies installed"
}

# Check if Rust/Cargo is installed
check_rust() {
    if ! command_exists cargo; then
        warn "Rust/Cargo not found. Installing via rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
        success "Rust installed"
    else
        success "Rust/Cargo found: $(cargo --version)"
    fi
}

# Build and install rusty-cpp
install_rusty_cpp() {
    info "Building and installing rusty-cpp..."

    # Check if we're in the rusty-cpp directory
    if [[ -f "Cargo.toml" ]] && grep -q "rusty-cpp" Cargo.toml 2>/dev/null; then
        info "Installing from local directory..."
        cargo install --path .
    else
        info "Installing from crates.io..."
        cargo install rusty-cpp
    fi

    success "rusty-cpp installed successfully!"
}

# Verify installation
verify_installation() {
    echo ""
    info "Verifying installation..."

    if command_exists rusty-cpp-checker; then
        success "rusty-cpp-checker is available in PATH"
        echo ""
        echo -e "${GREEN}Installation complete!${NC}"
        echo ""
        echo "Usage:"
        echo "  rusty-cpp-checker your_file.cpp"
        echo ""
        echo "For more information:"
        echo "  rusty-cpp-checker --help"
    else
        warn "rusty-cpp-checker not found in PATH"
        echo "Make sure ~/.cargo/bin is in your PATH:"
        echo "  export PATH=\"\$HOME/.cargo/bin:\$PATH\""
    fi
}

# Main installation flow
main() {
    print_banner

    detect_os
    info "Detected OS: $OS"

    case "$OS" in
        macos)
            install_macos
            ;;
        debian)
            install_debian
            ;;
        fedora)
            install_fedora
            ;;
        centos)
            install_centos
            ;;
        arch)
            install_arch
            ;;
        unknown)
            error "Unsupported operating system. Please install dependencies manually:
  - LLVM/Clang 16+ with development headers
  - Z3 SMT solver with development headers
  - Rust toolchain

Then run: cargo install rusty-cpp"
            ;;
    esac

    check_rust
    install_rusty_cpp
    verify_installation
}

# Run main function
main "$@"
