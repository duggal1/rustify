use std::process::Command;
use std::{fs, io};

pub fn create_distribution() -> io::Result<()> {
    println!("📦 Creating distribution packages...");

    // Create dist directory
    fs::create_dir_all("dist")?;

    // Build for different platforms
    let targets = vec![
        "x86_64-unknown-linux-gnu",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
        "x86_64-pc-windows-msvc"
    ];

    for target in targets {
        println!("Building for {}", target);
        Command::new("cargo")
            .args(["build", "--release", "--target", target])
            .status()?;

        // Copy binary to dist folder
        let binary_name = if target.contains("windows") {
            "rust-dockerize.exe"
        } else {
            "rust-dockerize"
        };

        let source = format!("target/{}/release/{}", target, binary_name);
        let dest = format!("dist/rust-dockerize-{}", target);
        
        fs::copy(source, dest)?;
    }

    // Create installation script
    let install_script = r#"#!/bin/bash
set -e

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

# Download URL base
RELEASE_URL="https://github.com/yourusername/rust-dockerize/releases/latest/download"

# Determine binary name
case "${OS}" in
    "linux")
        BINARY="rust-dockerize-x86_64-unknown-linux-gnu"
        ;;
    "darwin")
        if [ "$ARCH" = "arm64" ]; then
            BINARY="rust-dockerize-aarch64-apple-darwin"
        else
            BINARY="rust-dockerize-x86_64-apple-darwin"
        fi
        ;;
    *)
        echo "Unsupported operating system: $OS"
        exit 1
        ;;
esac

# Download binary
echo "Downloading rust-dockerize..."
curl -L "${RELEASE_URL}/${BINARY}" -o rust-dockerize

# Make binary executable
chmod +x rust-dockerize

# Move to PATH
sudo mv rust-dockerize /usr/local/bin/

echo "✅ rust-dockerize installed successfully!"
"#;

    fs::write("dist/install.sh", install_script)?;

    println!("✅ Distribution packages created in ./dist");
    Ok(())
} 