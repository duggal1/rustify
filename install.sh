#!/bin/bash
set -eo pipefail

# Print colorful messages with gradient effect
GREEN='\033[0;32m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${PURPLE}✨ ${CYAN}Installing rustify CLI v2.0.6...${NC}"

# Add error handling for curl commands
curl_with_retry() {
    local retry=0
    local max_retries=3
    local timeout=10
    while [ $retry -lt $max_retries ]; do
        if curl -fsSL --connect-timeout $timeout "$@"; then
            return 0
        fi
        retry=$((retry + 1))
        echo -e "${CYAN}Retry $retry/$max_retries...${NC}"
        sleep 2
    done
    return 1
}

# GitHub repository information
GITHUB_REPO="duggal1/rustify"
LATEST_URL="https://api.github.com/repos/${GITHUB_REPO}/releases/latest"

# Verify release exists
echo -e "${CYAN}🔍 Verifying release...${NC}"
RELEASE_CHECK=$(curl -s -o /dev/null -w "%{http_code}" $LATEST_URL)
if [ "$RELEASE_CHECK" != "200" ]; then
    echo -e "${RED}❌ Unable to access release. Please check:${NC}"
    echo -e "${BLUE}https://github.com/${GITHUB_REPO}/releases${NC}"
    exit 1
fi

# Get latest version
VERSION=$(curl -s $LATEST_URL | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
if [ -z "$VERSION" ]; then
    echo -e "${RED}❌ Failed to fetch latest version${NC}"
    exit 1
fi

echo -e "${CYAN}📦 Latest version: ${VERSION}${NC}"

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
    "linux")
        OS_NAME="linux"
        ;;
    "darwin")
        OS_NAME="darwin"
        ;;
    *)
        echo -e "${RED}❌ Unsupported operating system: $OS${NC}"
        exit 1
        ;;
esac

case "$ARCH" in
    "x86_64")
        ARCH_NAME="amd64"
        ;;
    "aarch64" | "arm64")
        ARCH_NAME="arm64"
        ;;
    *)
        echo -e "${RED}❌ Unsupported architecture: $ARCH${NC}"
        exit 1
        ;;
esac

BINARY_NAME="rustify-${OS_NAME}-${ARCH_NAME}.tar.gz"
DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/${BINARY_NAME}"

# Create temporary directory
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

# Download binary
echo -e "${CYAN}⬇️ Downloading rustify...${NC}"
if ! curl -L --progress-bar "$DOWNLOAD_URL" -o "$TMP_DIR/$BINARY_NAME"; then
    echo -e "${RED}❌ Download failed${NC}"
    exit 1
fi

# Extract binary
echo -e "${CYAN}📦 Extracting...${NC}"
tar xzf "$TMP_DIR/$BINARY_NAME" -C "$TMP_DIR"

# Install binary
INSTALL_DIR="/usr/local/bin"
if [ ! -w "$INSTALL_DIR" ]; then
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
fi

echo -e "${CYAN}📥 Installing to ${INSTALL_DIR}...${NC}"
mv "$TMP_DIR/rustify" "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/rustify"

# Verify installation
if ! command -v rustify >/dev/null 2>&1; then
    echo -e "${RED}❌ Installation failed${NC}"
    exit 1
fi

# Verify version
INSTALLED_VERSION=$("$INSTALL_DIR/rustify" --version)
echo -e "${GREEN}✅ Successfully installed ${INSTALLED_VERSION}${NC}"
echo -e "${CYAN}🚀 Run 'rustify --help' to get started${NC}"
