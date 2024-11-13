#!/bin/bash
set -e

# Print colorful messages with gradient effect
GREEN='\033[0;32m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${PURPLE}✨ ${CYAN}Installing rustify CLI...${NC}"

# Check for required tools
command -v curl >/dev/null 2>&1 || { 
    echo -e "${RED}❌ ${PURPLE}curl is required but not installed. Please install curl first.${NC}"
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
        echo -e "${CYAN}📡 ${BLUE}Detected Linux operating system${NC}"
        BINARY_NAME="rustify-x86_64-unknown-linux-gnu"
        ;;
    "darwin")
        echo -e "${CYAN}📡 ${BLUE}Detected macOS operating system${NC}"
        if [ "$ARCH" = "arm64" ]; then
            BINARY_NAME="rustify-aarch64-apple-darwin"
        else
            BINARY_NAME="rustify-x86_64-apple-darwin"
        fi
        ;;
    *)
        echo -e "${RED}❌ ${PURPLE}Unsupported operating system: $OS${NC}"
        exit 1
        ;;
esac

# GitHub repository information
GITHUB_REPO="duggal1/rustify"
VERSION="v0.1.0"  # Update this when you release new versions
DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/${BINARY_NAME}"

echo -e "${CYAN}⬇️ ${BLUE}Downloading rustify...${NC}"
HTTP_RESPONSE=$(curl -L --write-out "HTTPSTATUS:%{http_code}" -o "$TMP_DIR/rustify" "${DOWNLOAD_URL}")
HTTP_STATUS=$(echo "$HTTP_RESPONSE" | tr -d '\n' | sed -e 's/.*HTTPSTATUS://')

if [ "$HTTP_STATUS" -ne 200 ]; then
    echo -e "${RED}❌ ${PURPLE}Download failed (HTTP status: $HTTP_STATUS)${NC}"
    echo -e "${PURPLE}Download URL: ${BLUE}${DOWNLOAD_URL}${NC}"
    exit 1
fi

# Make binary executable
chmod +x "$TMP_DIR/rustify"

# Install to system
if [ -w "/usr/local/bin" ]; then
    echo -e "${CYAN}📥 ${BLUE}Installing to /usr/local/bin${NC}"
    mv "$TMP_DIR/rustify" "/usr/local/bin/rustify"
else
    echo -e "${CYAN}📥 ${BLUE}Installing to ~/.local/bin${NC}"
    mkdir -p ~/.local/bin
    mv "$TMP_DIR/rustify" ~/.local/bin/rustify
    
    # Add to PATH if needed
    if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc 2>/dev/null || true
        echo -e "${CYAN}📝 ${BLUE}Added ~/.local/bin to PATH${NC}"
    fi
fi

# Verify installation
if command -v rustify >/dev/null 2>&1; then
    echo -e "${GREEN}✅ ${CYAN}rustify installed successfully! 🎉${NC}"
    echo -e "${CYAN}🔧 ${BLUE}Run 'rustify --help' to get started${NC}"
else
    echo -e "${RED}❌ ${PURPLE}Installation failed. Please try again or install manually.${NC}"
    exit 1
fi

# Print version
echo -e "${CYAN}📋 ${BLUE}Installed version:${NC}"
rustify --version || echo -e "${RED}Version information not available${NC}"
