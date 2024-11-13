#!/bin/bash
set -e

# Print colorful messages with gradient effect
GREEN='\033[0;32m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${PURPLE}✨ ${CYAN}Installing rustify CLI...${NC}"

# Check for required tools
command -v curl >/dev/null 2>&1 || { 
    echo -e "${RED}❌ ${PURPLE}curl is required but not installed. Please install curl first.${NC}"
    exit 1 
}

command -v tar >/dev/null 2>&1 || { 
    echo -e "${RED}❌ ${PURPLE}tar is required but not installed. Please install tar first.${NC}"
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
        BINARY_NAME="rustify-x86_64-unknown-linux-gnu.tar.gz"
        ;;
    "darwin")
        echo -e "${CYAN}📡 ${BLUE}Detected macOS operating system${NC}"
        if [ "$ARCH" = "arm64" ]; then
            BINARY_NAME="rustify-aarch64-apple-darwin.tar.gz"
        else
            BINARY_NAME="rustify-x86_64-apple-darwin.tar.gz"
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

# Download the binary
echo -e "${CYAN}⬇️ ${BLUE}Downloading rustify...${NC}"
HTTP_RESPONSE=$(curl -L --write-out "HTTPSTATUS:%{http_code}" --progress-bar "${DOWNLOAD_URL}" -o "$TMP_DIR/rustify.tar.gz")
HTTP_STATUS=$(echo "$HTTP_RESPONSE" | tr -d '\n' | sed -e 's/.*HTTPSTATUS://')

if [ "$HTTP_STATUS" -ne 200 ]; then
    echo -e "${RED}❌ ${PURPLE}Download failed (HTTP status: $HTTP_STATUS)${NC}"
    echo -e "${PURPLE}Download URL: ${BLUE}${DOWNLOAD_URL}${NC}"
    exit 1
fi

# Verify the downloaded file exists and has size > 0
if [ ! -s "$TMP_DIR/rustify.tar.gz" ]; then
    echo -e "${RED}❌ ${PURPLE}Downloaded file is empty or does not exist${NC}"
    exit 1
fi

# Extract binary
echo -e "${CYAN}📦 ${BLUE}Extracting rustify...${NC}"
if ! tar xzf "$TMP_DIR/rustify.tar.gz" -C "$TMP_DIR"; then
    echo -e "${RED}❌ ${PURPLE}Failed to extract archive${NC}"
    exit 1
fi

# Verify binary exists after extraction
if [ ! -f "$TMP_DIR/rustify" ]; then
    echo -e "${RED}❌ ${PURPLE}Binary not found in archive${NC}"
    exit 1
fi

# Make binary executable
chmod +x "$TMP_DIR/rustify"

# Install to system
INSTALL_DIR=""
if [ -w "/usr/local/bin" ]; then
    INSTALL_DIR="/usr/local/bin"
    echo -e "${CYAN}📥 ${BLUE}Installing to /usr/local/bin${NC}"
else
    INSTALL_DIR="$HOME/.local/bin"
    echo -e "${CYAN}📥 ${BLUE}Installing to ~/.local/bin${NC}"
    mkdir -p "$INSTALL_DIR"
fi

# Move binary to installation directory
if ! mv "$TMP_DIR/rustify" "$INSTALL_DIR/rustify"; then
    echo -e "${RED}❌ ${PURPLE}Failed to install binary${NC}"
    exit 1
fi

# Add to PATH if needed
if [ "$INSTALL_DIR" = "$HOME/.local/bin" ]; then
    if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc 2>/dev/null || true
        echo -e "${CYAN}📝 ${BLUE}Added ~/.local/bin to PATH${NC}"
        # Export PATH for immediate use
        export PATH="$HOME/.local/bin:$PATH"
    fi
fi

# Verify installation
if ! command -v rustify >/dev/null 2>&1; then
    echo -e "${RED}❌ ${PURPLE}Installation failed. Binary not found in PATH${NC}"
    echo -e "${PURPLE}Installation directory: ${BLUE}${INSTALL_DIR}${NC}"
    exit 1
fi

# Test binary execution
if ! "$INSTALL_DIR/rustify" --version >/dev/null 2>&1; then
    echo -e "${RED}❌ ${PURPLE}Binary verification failed. The installed binary may be corrupted${NC}"
    exit 1
fi

echo -e "${GREEN}✅ ${CYAN}rustify installed successfully! 🎉${NC}"
echo -e "${CYAN}🔧 ${BLUE}Run 'rustify --help' to get started${NC}"

# Print version
echo -e "${CYAN}📋 ${BLUE}Installed version:${NC}"
"$INSTALL_DIR/rustify" --version || echo -e "${RED}Version information not available${NC}"
