#!/bin/bash
set -e

# Print colorful messages
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}✨ Installing rustify CLI...${NC}"

# Check for required tools
command -v curl >/dev/null 2>&1 || { 
    echo "❌ curl is required but not installed. Please install curl first."
    exit 1 
}

# Create temporary directory for downloads
TMP_DIR=$(mktemp -d)
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

# Set binary name based on OS and architecture
case "${OS}" in
    "linux")
        echo -e "${BLUE}📡 Detected Linux operating system${NC}"
        BINARY_NAME="rustify-x86_64-unknown-linux-gnu.tar.gz"
        ;;
    "darwin")
        echo -e "${BLUE}📡 Detected macOS operating system${NC}"
        if [ "$ARCH" = "arm64" ]; then
            BINARY_NAME="rustify-aarch64-apple-darwin.tar.gz"
        else
            BINARY_NAME="rustify-x86_64-apple-darwin.tar.gz"
        fi
        ;;
    *)
        echo "❌ Unsupported operating system: $OS"
        exit 1
        ;;
esac

# GitHub repository information
GITHUB_REPO="duggal1/rustify"
VERSION="v0.1.0"  # Update this when you release new versions
DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/${BINARY_NAME}"

# Download binary
echo -e "${BLUE}⬇️  Downloading rustify...${NC}"
curl -L --progress-bar "${DOWNLOAD_URL}" -o "$TMP_DIR/rustify.tar.gz"

# Extract binary
tar xzf "$TMP_DIR/rustify.tar.gz" -C "$TMP_DIR"

# Install to system
if [ -w "/usr/local/bin" ]; then
    echo -e "${BLUE}📥 Installing to /usr/local/bin${NC}"
    mv "$TMP_DIR/rustify" "/usr/local/bin/rustify"
else
    echo -e "${BLUE}📥 Installing to ~/.local/bin${NC}"
    mkdir -p ~/.local/bin
    mv "$TMP_DIR/rustify" ~/.local/bin/rustify
    
    # Add to PATH if needed
    if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc 2>/dev/null || true
        echo -e "${BLUE}📝 Added ~/.local/bin to PATH${NC}"
    fi
fi

# Make binary executable
chmod +x "/usr/local/bin/rustify" 2>/dev/null || chmod +x "$HOME/.local/bin/rustify"

# Verify installation
if command -v rustify >/dev/null 2>&1; then
    echo -e "${GREEN}✅ rustify installed successfully! 🎉${NC}"
    echo -e "${BLUE}🔧 Run 'rustify --help' to get started${NC}"
else
    echo "❌ Installation failed. Please try again or install manually."
    exit 1
fi

# Print version
echo -e "${BLUE}📋 Installed version:${NC}"
rustify --version || echo "Version information not available"
