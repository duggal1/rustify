#!/bin/bash
set -e

# Print colorful messages
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}✨ Installing rust-dockerize CLI...${NC}"

# Your GitHub repository information
GITHUB_REPO="duggal1/rustify"
VERSION="v0.1.0"  # Update this with your latest version

# Direct download URL (no token needed for public repos)
DOWNLOAD_BASE="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}"
DOWNLOAD_URL="${DOWNLOAD_BASE}/${BINARY_NAME}.tar.gz"

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
        BINARY_NAME="rust-dockerize-x86_64-unknown-linux-gnu"
        ;;
    "darwin")
        echo -e "${BLUE}📡 Detected macOS operating system${NC}"
        if [ "$ARCH" = "arm64" ]; then
            BINARY_NAME="rust-dockerize-aarch64-apple-darwin"
        else
            BINARY_NAME="rust-dockerize-x86_64-apple-darwin"
        fi
        ;;
    *)
        echo "❌ Unsupported operating system: $OS"
        exit 1
        ;;
esac

# Download binary
echo -e "${BLUE}⬇️  Downloading rust-dockerize...${NC}"
curl -L "$DOWNLOAD_URL" -o "$TMP_DIR/rust-dockerize.tar.gz"

# Extract the binary
echo -e "${BLUE}🔍 Extracting rust-dockerize...${NC}"
tar -xzf "$TMP_DIR/rust-dockerize.tar.gz" -C "$TMP_DIR"

# Make binary executable
chmod +x "$TMP_DIR/rust-dockerize"

# Install to system
if [ -w "/usr/local/bin" ]; then
    echo -e "${BLUE}📥 Installing to /usr/local/bin${NC}"
    mv "$TMP_DIR/rust-dockerize" "/usr/local/bin/rust-dockerize"
else
    echo -e "${BLUE}📥 Installing to ~/.local/bin${NC}"
    mkdir -p ~/.local/bin
    mv "$TMP_DIR/rust-dockerize" ~/.local/bin/rust-dockerize
    
    # Add to PATH if needed
    if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc 2>/dev/null || true
        echo -e "${BLUE}📝 Added ~/.local/bin to PATH${NC}"
    fi
fi

# Verify installation
if command -v rust-dockerize >/dev/null 2>&1; then
    echo -e "${GREEN}✅ rust-dockerize installed successfully! 🎉${NC}"
    echo -e "${BLUE}🔧 Run 'rust-dockerize --help' to get started${NC}"
else
    echo "❌ Installation failed. Please try again or install manually."
    exit 1
fi

# Print version
echo -e "${BLUE}📋 Installed version:${NC}"
rust-dockerize --version
