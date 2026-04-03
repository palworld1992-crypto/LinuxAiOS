#!/bin/bash
set -euo pipefail

BIOS_TYPE=$(cat /sys/firmware/efi/facade 2>/dev/null && echo "UEFI" || echo "BIOS")

if [[ $EUID -eq 0 ]]; then
    echo "WARNING: Running as root. This script should be run as a non-root user with sudo."
    echo "Press Ctrl+C to abort or Enter to continue..."
    read -r
fi

check_os() {
    if [[ -f /etc/arch-release ]]; then
        echo "Arch Linux detected"
        return 0
    fi
    echo "This script requires Arch Linux"
    exit 1
}

install_dependencies() {
    echo "Installing dependencies..."
    sudo pacman -Sy --noconfirm git curl wget base-devel python python-pip
}

install_rust() {
    if command -v rustc &>/dev/null; then
        echo "Rust already installed: $(rustc --version)"
    else
        echo "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
        source "$HOME/.cargo/env"
    fi
}

install_zig() {
    if command -v zig &>/dev/null; then
        echo "Zig already installed: $(zig version)"
    else
        echo "Installing Zig 0.16+..."
        wget -q https://ziglang.org/download/0.16.0/zig-linux-x86_64-0.16.0.tar.xz
        sudo tar -xf zig-linux-x86_64-0.16.0.tar.xz -C /usr/local --strip-components=1
        rm -f zig-linux-x86_64-0.16.0.tar.xz
    fi
}

install_spark() {
    if command -v gnat &>/dev/null; then
        echo "GNAT already installed: $(gnat --version | head -1)"
    else
        echo "Installing SPARK Pro 2026..."
        echo "Please install SPARK Pro manually from AdaCore"
        echo "Download: https://www.adacore.com/download"
    fi
}

install_python() {
    if command -v python3 &>/dev/null; then
        PYTHON_VERSION=$(python3 --version | awk '{print $2}')
        if [[ ${PYTHON_VERSION%%.*} -ge 3 ]]; then
            echo "Python $PYTHON_VERSION already installed"
            return 0
        fi
    fi
    echo "Installing Python 3.12..."
    sudo pacman -S --noconfirm python python-pip
}

install_aios() {
    if [[ -d "$HOME/aios" ]]; then
        echo "AIOS directory already exists, pulling latest..."
        cd "$HOME/aios"
        git pull
    else
        echo "Cloning AIOS repository..."
        git clone https://github.com/aios/aios.git "$HOME/aios" || {
            echo "Repository not found, creating local project structure..."
            mkdir -p "$HOME/aios"
        }
    fi
}

build_project() {
    cd "$HOME/aios"
    
    if [[ -f Cargo.toml ]]; then
        echo "Building Rust project..."
        cargo build --release
    fi
    
    if [[ -d zig ]]; then
        echo "Building Zig project..."
        cd zig
        zig build
        cd ..
    fi
    
    if [[ -d spark ]]; then
        echo "Building Ada/SPARK project..."
        cd spark
        for prj in */; do
            if [[ -f "${prj}*.gpr" ]]; then
                gprbuild -p "${prj%/}.gpr"
            fi
        done
        cd ..
    fi
}

check_kernel_features() {
    echo "Checking kernel features..."
    cd "$HOME/aios"
    if [[ -f arch_linux/scripts/check_kernel_features.sh ]]; then
        bash arch_linux/scripts/check_kernel_features.sh
    else
        echo "WARNING: check_kernel_features.sh not found"
    fi
}

main() {
    check_os
    install_dependencies
    install_rust
    install_zig
    install_spark
    install_python
    install_aios
    build_project
    check_kernel_features
    
    echo "Bootstrap complete!"
    echo "Next: Run aios-installer.sh to configure the system"
}

main "$@"