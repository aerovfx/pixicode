#!/bin/bash
# Pixicode Install Script
# Usage: curl -fsSL https://raw.githubusercontent.com/pixicode/pixicode/main/install | bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
REPO="pixicode/pixicode"
INSTALL_DIR="${PIXICODE_INSTALL_DIR:-$HOME/.pixicode}"
BIN_DIR="$INSTALL_DIR/bin"

echo_info() {
    echo -e "${GREEN}✓${NC} $1"
}

echo_warn() {
    echo -e "${YELLOW}!${NC} $1"
}

echo_error() {
    echo -e "${RED}✗${NC} $1"
}

# Detect platform
detect_platform() {
    local os arch
    
    case "$(uname -s)" in
        Linux*)  os="linux";;
        Darwin*) os="macos";;
        *)       echo_error "Unsupported OS: $(uname -s)"; exit 1;;
    esac
    
    case "$(uname -m)" in
        x86_64)  arch="x86_64";;
        arm64)   arch="aarch64";;
        aarch64) arch="aarch64";;
        *)       echo_error "Unsupported architecture: $(uname -m)"; exit 1;;
    esac
    
    echo "${os}-${arch}"
}

# Download and install
install() {
    local platform="$1"
    local version="$2"
    local tarball="pixicode-${version}-${platform}.tar.gz"
    local download_url="https://github.com/${REPO}/releases/download/${version}/${tarball}"
    
    echo_info "Downloading pixicode for ${platform}..."
    
    # Create install directory
    mkdir -p "$BIN_DIR"
    
    # Download and extract
    if command -v curl &> /dev/null; then
        curl -fsSL "$download_url" | tar xz -C "$BIN_DIR"
    elif command -v wget &> /dev/null; then
        wget -qO- "$download_url" | tar xz -C "$BIN_DIR"
    else
        echo_error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi
    
    # Make executable
    chmod +x "$BIN_DIR/pixicode"
    
    echo_info "pixicode installed to $BIN_DIR"
}

# Setup PATH
setup_path() {
    local shell_rc=""
    
    # Detect shell
    if [[ -n "$ZSH_VERSION" ]]; then
        shell_rc="$HOME/.zshrc"
    elif [[ -n "$BASH_VERSION" ]]; then
        shell_rc="$HOME/.bashrc"
    elif [[ -n "$FISH_VERSION" ]]; then
        shell_rc="$HOME/.config/fish/config.fish"
    fi
    
    # Add to PATH if not already present
    if [[ -n "$shell_rc" ]] && ! grep -q "$BIN_DIR" "$shell_rc" 2>/dev/null; then
        echo "" >> "$shell_rc"
        echo "# Pixicode" >> "$shell_rc"
        echo "export PATH=\"$BIN_DIR:\$PATH\"" >> "$shell_rc"
        echo_info "Added pixicode to PATH in $shell_rc"
        echo_warn "Please restart your terminal or run: source $shell_rc"
    else
        echo_warn "Please add $BIN_DIR to your PATH manually"
    fi
}

# Main
main() {
    echo "🚀 Pixicode Installer"
    echo ""
    
    # Check for --version flag
    local version="latest"
    if [[ "$1" == "--version" ]]; then
        version="$2"
    fi
    
    # Get latest version if not specified
    if [[ "$version" == "latest" ]]; then
        version=$(curl -s https://api.github.com/repos/${REPO}/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
        if [[ -z "$version" ]]; then
            echo_error "Failed to fetch latest version"
            exit 1
        fi
    fi
    
    echo_info "Installing version: $version"
    
    # Detect platform
    local platform
    platform=$(detect_platform)
    echo_info "Detected platform: $platform"
    
    # Install
    install "$platform" "$version"
    
    # Setup PATH
    setup_path
    
    echo ""
    echo_info "Installation complete!"
    echo ""
    echo "To get started:"
    echo "  1. Restart your terminal or run: source ~/.bashrc (or ~/.zshrc)"
    echo "  2. Run: pixicode --help"
    echo ""
}

main "$@"
