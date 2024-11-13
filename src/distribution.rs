use std::process::Command;
use std::{fs, io};

pub fn create_distribution() -> io::Result<()> {
    println!("📦 Creating distribution packages...");

    // Create dist directory
    fs::create_dir_all("dist")?;

    // Build for different platforms
    let targets = vec![
        ("x86_64-unknown-linux-gnu", "rustify-linux-amd64"),
        ("x86_64-apple-darwin", "rustify-darwin-amd64"),
        ("aarch64-apple-darwin", "rustify-darwin-arm64"),
    ];

    for (target, binary_name) in targets {
        println!("Building for {}", target);
        Command::new("cargo")
            .args(["build", "--release", "--target", target])
            .status()?;

        // Package binary
        let source = format!("target/{}/release/rustify", target);
        let dest = format!("dist/{}.tar.gz", binary_name);
        
        Command::new("tar")
            .args(["-czf", &dest, "-C", &format!("target/{}/release", target), "rustify"])
            .status()?;

        // Create checksum
        Command::new("shasum")
            .args(["-a", "256", &dest])
            .output()?;
    }

    println!("✅ Distribution packages created in ./dist");
    Ok(())
} 