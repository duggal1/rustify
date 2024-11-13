use std::{env, fs, io::{self, Write}, path::Path, process::Command};
use chrono::Local;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;
use std::thread;
use std::time::Duration;
mod gradient;
use gradient::GradientText;
use tokio::runtime::Runtime;
use futures::future::join_all;
use std::sync::Arc;
use parking_lot::RwLock;
use dashmap::DashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppMetadata {
    app_name: String,
    app_type: String,
    port: String,
    created_at: String,
    container_id: Option<String>,
    status: String,
    kubernetes_enabled: bool,
    #[serde(default)]
    kubernetes_metadata: KubernetesMetadata,
    #[serde(default)]
    performance_metrics: PerformanceMetrics,
    #[serde(default)]
    scaling_config: ScalingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct KubernetesMetadata {
    #[serde(default = "default_namespace")]
    namespace: String,
    #[serde(default)]
    deployment_name: String,
    #[serde(default)]
    service_name: String,
    #[serde(default)]
    replicas: i32,
    #[serde(default)]
    pod_status: Vec<String>,
    #[serde(default)]
    ingress_host: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceMetrics {
    #[serde(default)]
    avg_response_time_ms: f64,
    #[serde(default)]
    requests_per_second: u64,
    #[serde(default)]
    error_rate: f64,
    #[serde(default)]
    memory_usage_mb: f64,
    #[serde(default)]
    cpu_usage_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScalingConfig {
    #[serde(default)]
    auto_scale_threshold: f64,
    #[serde(default = "default_min_instances")]
    min_instances: u32,
    #[serde(default = "default_max_instances")]
    max_instances: u32,
    #[serde(default)]
    scale_up_cooldown: u64,
    #[serde(default)]
    scale_down_cooldown: u64,
}

struct DockerManager;

impl DockerManager {
    fn new() -> Self {
        DockerManager
    }

    fn launch_docker_desktop(&self) -> io::Result<()> {
        println!("{}", GradientText::cyber("🚀 Launching Docker Desktop..."));
        
        #[cfg(target_os = "macos")]
        {
            Command::new("open")
                .args(["-a", "Docker"])
                .output()?;
        }

        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .args(["/C", "start", "\"\"", "\"C:\\Program Files\\Docker\\Docker\\Docker Desktop.exe\""])
                .output()?;
        }

        #[cfg(target_os = "linux")]
        {
            Command::new("systemctl")
                .args(["--user", "start", "docker-desktop"])
                .output()?;
        }

        // Wait for Docker Desktop to start
        println!("⏳ Waiting for Docker Desktop to start...");
        for i in 0..30 {
            if i > 0 {
                thread::sleep(Duration::from_secs(2));
            }

            match Command::new("docker").arg("info").output() {
                Ok(output) if output.status.success() => {
                    println!("\n✅ Docker Desktop is running!");
                    return Ok(());
                }
                _ if i == 29 => {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "Docker Desktop failed to start after 60 seconds"
                    ));
                }
                _ => {
                    print!(".");
                    io::stdout().flush()?;
                }
            }
        }

        Ok(())
    }

    fn verify_and_setup_docker(&self) -> io::Result<()> {
        println!("{}", GradientText::cyber("🔍 Checking Docker installation..."));
        
        match Command::new("docker").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout);
                println!("{} {}", 
                    GradientText::success("✅ Docker installed:"),
                    GradientText::info(&version.trim())
                );
                
                match Command::new("docker").arg("info").output() {
                    Ok(output) if output.status.success() => {
                        println!("{}", GradientText::success("✅ Docker Desktop is running"));
                    }
                    _ => {
                        println!("{}", GradientText::warning("⏳ Docker Desktop is not running. Attempting to start..."));
                        self.launch_docker_desktop()?;
                    }
                }
            }
            _ => {
                println!("{}", GradientText::error("❌ Docker is not installed"));
                self.install_docker()?;

                println!("🚀 Starting Docker Desktop for the first time...");
                self.launch_docker_desktop()?;
                // Pull some common images in advance
                println!("📥 Pulling common Docker images...");
                let common_images = [
                    "node:18-alpine",
                    "oven/bun:latest",
                    "nginx:alpine",
                    "mongo:latest",
                    "postgres:alpine"
                ];
                
                for image in common_images.iter() {
                    print!("Pulling {}... ", image);
                    io::stdout().flush()?;
                    match Command::new("docker")
                        .args(["pull", image])
                        .output() {
                            Ok(_) => println!("✅"),
                            Err(_) => println!("❌"),
                        }
                }
            }
        }

        Ok(())
    }

    fn install_docker(&self) -> io::Result<()> {
        println!("🐳 Docker Desktop is not installed. Starting installation process...");

        #[cfg(target_os = "macos")]
        {
            // Check if Homebrew is installed
            if Command::new("brew").arg("--version").output().is_err() {
                println!("📦 Installing Homebrew first...");
                let brew_install = r#"/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)""#;
                Command::new("bash")
                    .arg("-c")
                    .arg(brew_install)
                    .status()?;
            }

            println!("📥 Installing Docker Desktop via Homebrew...");
            Command::new("brew")
                .args(["install", "--cask", "docker"])
                .status()?;
        }

        #[cfg(target_os = "windows")]
        {
            println!("📥 Downloading Docker Desktop Installer...");
            let installer_url = "https://desktop.docker.com/win/main/amd64/Docker%20Desktop%20Installer.exe";
            Command::new("powershell")
                .args([
                    "-Command",
                    &format!("Invoke-WebRequest '{}' -OutFile 'DockerDesktopInstaller.exe'", installer_url)
                ])
                .status()?;

            println!("🔧 Installing Docker Desktop...");
            Command::new("DockerDesktopInstaller.exe")
                .args(["install", "--quiet"])
                .status()?;

            // Cleanup installer
            let _ = fs::remove_file("DockerDesktopInstaller.exe");
        }

        #[cfg(target_os = "linux")]
        {
            println!("📥 Installing Docker on Linux...");
            
            // Update package list
            Command::new("sudo")
                .args(["apt-get", "update"])
                .status()?;

            // Install prerequisites
            Command::new("sudo")
                .args([
                    "apt-get", "install", "-y",
                    "ca-certificates", "curl", "gnupg", "lsb-release"
                ])
                .status()?;

            // Add Docker's official GPG key
            println!("🔑 Adding Docker's GPG key...");
            Command::new("bash")
                .arg("-c")
                .arg("curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg")
                .status()?;

            // Set up the stable repository
            println!("📦 Setting up Docker repository...");
            let repo_command = r#"echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null"#;
            Command::new("bash")
                .arg("-c")
                .arg(repo_command)
                .status()?;

            // Install Docker Engine
            println!("🔧 Installing Docker Engine...");
            Command::new("sudo")
                .args(["apt-get", "update"])
                .status()?;
            Command::new("sudo")
                .args([
                    "apt-get", "install", "-y",
                    "docker-ce", "docker-ce-cli", "containerd.io"
                ])
                .status()?;

            // Add user to docker group
            println!("👤 Adding current user to docker group...");
            Command::new("sudo")
                .args(["usermod", "-aG", "docker", &whoami::username()])
                .status()?;
        }

        println!("✅ Docker installation completed!");
        println!("⚠️  You may need to restart your computer for all changes to take effect.");
        
        // Ask user if they want to restart now
        print!("Would you like to restart now? [y/N]: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if input.trim().to_lowercase() == "y" {
            println!("🔄 Restarting system...");
            #[cfg(target_os = "macos")]
            Command::new("sudo")
                .args(["shutdown", "-r", "now"])
                .status()?;

            #[cfg(target_os = "windows")]
            Command::new("shutdown")
                .args(["/r", "/t", "0"])
                .status()?;

            #[cfg(target_os = "linux")]
            Command::new("sudo")
                .args(["reboot"])
                .status()?;
        }

        Ok(())
    }

    // Add other Docker-related methods here...
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    match args.get(1).map(String::as_str) {
        Some("deploy") => {
            if args.len() < 8 {
                eprintln!("Usage: rust-dockerize deploy --app <app-name> --type <app-type> --port <port> [--k8s] [--replicas <count>] [--namespace <name>] [--mode <dev|prod>]");
                return;
            }
            let k8s_enabled = args.contains(&String::from("--k8s"));
            let mode = args.iter()
                .position(|x| x == "--mode")
                .and_then(|i| args.get(i + 1))
                .map(String::as_str)
                .unwrap_or("dev");

            let replicas = if mode == "prod" {
                args.iter()
                    .position(|x| x == "--replicas")
                    .and_then(|i| args.get(i + 1))
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(3) // Default to 3 replicas in production
            } else {
                1 // Single replica in dev mode
            };

            let namespace = args.iter()
                .position(|x| x == "--namespace")
                .and_then(|i| args.get(i + 1))
                .unwrap_or(&String::from(if mode == "prod" { "production" } else { "development" }))
                .to_string();

            if let Err(e) = deploy_app(&args, k8s_enabled, replicas, &namespace, mode) {
                eprintln!("Error: {}", e);
            }
        }
        Some("check") => {
            println!("🔍 Checking infrastructure status...");
            if let Err(e) = verify_infrastructure() {
                eprintln!("❌ Infrastructure check failed: {}", e);
            }
        }
        Some("cleanup") => {
            if let Err(e) = cleanup_deployment() {
                eprintln!("Error during cleanup: {}", e);
            }
        }
        Some("status") => {
            if let Err(e) = check_kubernetes_status() {
                eprintln!("Error checking status: {}", e);
            }
        }
        _ => {
            eprintln!("Usage:");
            eprintln!("  rust-dockerize deploy --app <app-name> --type <app-type> --port <port> [--k8s] [--replicas <count>] [--namespace <name>] [--mode <dev|prod>]");
            eprintln!("  rust-dockerize check");
            eprintln!("  rust-dockerize cleanup");
            eprintln!("  rust-dockerize status");
        }
    }
}
fn deploy_app(args: &[String], k8s_enabled: bool, replicas: i32, namespace: &str, mode: &str) -> io::Result<()> {
    let metadata = AppMetadata {
        app_name: args[3].clone(),
        app_type: args[5].clone(), 
        port: args[7].clone(),
        created_at: Local::now().to_rfc3339(),
        container_id: None,
        status: String::from("pending"),
        kubernetes_enabled: k8s_enabled,
        kubernetes_metadata: KubernetesMetadata {
            namespace: namespace.to_string(),
            deployment_name: format!("{}-deployment", args[3]),
            service_name: format!("{}-service", args[3]), 
            replicas,
            pod_status: vec![],
            ingress_host: None,
        },
        performance_metrics: PerformanceMetrics::default(),
        scaling_config: ScalingConfig::default(),
    };
    
    println!("🚀 Starting deployment process for {} ({})", metadata.app_name, metadata.app_type);
    
    // Create Docker manager instance and verify setup
    let docker_manager = DockerManager::new();
    docker_manager.verify_and_setup_docker()?;
    
    let current_dir = env::current_dir()?;
    println!("📂 Working directory: {}", current_dir.display());
    // Initialize metadata 
    let mut metadata = metadata;

    // Detect project structure and framework
    fn detect_project_structure(dir: &Path) -> io::Result<(String, String)> {
        let mut app_type = String::from("unknown");
        let mut entry_point = String::from("src/main.rs");

        // Check for package.json
        let package_json_path = dir.join("package.json");
        if package_json_path.exists() {
            let contents = fs::read_to_string(package_json_path)?;
            if contents.contains("\"next\"") {
                app_type = String::from("nextjs");
                entry_point = String::from("pages/index.js");
            } else if contents.contains("\"react\"") {
                app_type = String::from("react"); 
                entry_point = String::from("src/index.js");
            } else if contents.contains("\"@remix-run/react\"") {
                app_type = String::from("remix");
                entry_point = String::from("app/root.tsx");
            }
        }

        // Check for Cargo.toml
        let cargo_toml_path = dir.join("Cargo.toml");
        if cargo_toml_path.exists() {
            app_type = String::from("rust");
            entry_point = String::from("src/main.rs");
        }

        Ok((app_type, entry_point))
    }

    let (detected_app_type, entry_point) = detect_project_structure(&current_dir)?;
    let app_type = if detected_app_type != "unknown" { &detected_app_type } else { &metadata.app_type };

    // Create necessary files
    create_app_files(app_type, &metadata.port, &entry_point)?;
    
    // Initialize performance monitoring
    let metrics = Arc::new(RwLock::new(PerformanceMetrics::default()));
    let cache = Arc::new(DashMap::new());
    // Create async runtime for concurrent operations
    let rt = Runtime::new()?;
    rt.block_on(async {
        let app_name = metadata.app_name.clone();
        let namespace = metadata.kubernetes_metadata.namespace.clone();
        let mode = metadata.status.clone();
        
        let futures = vec![
            tokio::spawn({
                let app_name = app_name.clone();
                let namespace = namespace.clone();
                let mode = mode.clone();
                async move { setup_monitoring(&app_name, &namespace, &mode) }
            }),
            tokio::spawn({
                let cache = cache.clone();
                async move { setup_caching(cache).await }
            }),
            tokio::spawn({
                let mode = mode.clone();
                async move { setup_load_balancing(&mode).await }
            }),
        ];

        // Handle the results properly
        for result in join_all(futures).await {
            match result {
                Ok(_) => (),
                Err(e) => eprintln!("Task error: {}", e),
            }
        }
        
        Ok::<(), io::Error>(())
    })?;

    fn optimize_for_framework(app_type: &str, mode: &str) -> io::Result<()> {
        match app_type {
            "nextjs" => {
                if mode == "prod" {
                    // Production optimizations for Next.js
                    println!("Applying Next.js production optimizations...");
                    Command::new("npm")
                        .args(&["run", "build"])
                        .status()?;
                }
            }
            "react" => {
                if mode == "prod" {
                    // Production optimizations for React
                    println!("Applying React production optimizations...");
                    Command::new("npm")
                        .args(&["run", "build"])
                        .status()?;
                }
            }
            "remix" => {
                if mode == "prod" {
                    // Production optimizations for Remix
                    println!("Applying Remix production optimizations...");
                    Command::new("npm")
                        .args(&["run", "build"])
                        .status()?;
                }
            }
            _ => println!("No specific optimizations for framework: {}", app_type),
        }
        Ok(())
    }

    // Framework-specific optimizations
    optimize_for_framework(app_type, mode)?;

    // Generate and write Dockerfile
    println!("📝 Generating Dockerfile...");
    let package_json = r#"{
        "next": "^13.4.12",
        "react": "^18.2.0", 
        "react-dom": "^18.2.0"
    }"#;
    fs::write("package.json", package_json)?;
    Ok(())
}

fn create_nextjs_files(port: &str) -> io::Result<()> {
    let package_json = r#"{
        "name": "nextjs-app",
        "version": "0.1.0",
        "private": true,
        "scripts": {
            "dev": "next dev",
            "build": "next build",
            "start": "next start"
        },
        "dependencies": {
            "next": "^13.4.12",
            "react": "^18.2.0",
            "react-dom": "^18.2.0"
        }
    }"#;
    fs::write("package.json", package_json)?;
    Ok(())
}

fn create_app_files(app_type: &str, port: &str, entry_point: &str) -> io::Result<()> {
    if !Path::new("package.json").exists() {
        match app_type {
            "nextjs" => create_nextjs_files(port)?,
            "react" => {
                let package_json = r#"{
                    "name": "react-app",
                    "version": "0.1.0",
                    "private": true,
                    "dependencies": {
                        "react": "^18.2.0",
                        "react-dom": "^18.2.0",
                        "react-scripts": "5.0.1"
                    },
                    "scripts": {
                        "start": "react-scripts start",
                        "build": "react-scripts build",
                        "test": "react-scripts test",
                        "eject": "react-scripts eject"
                    }
                }"#;
                fs::write("package.json", package_json)?;
            },
            "remix" => {
                let package_json = r#"{
                    "name": "remix-app",
                    "private": true,
                    "sideEffects": false,
                    "scripts": {
                        "build": "remix build",
                        "dev": "remix dev",
                        "start": "remix-serve build"
                    },
                    "dependencies": {
                        "@remix-run/node": "^1.19.1",
                        "@remix-run/react": "^1.19.1",
                        "@remix-run/serve": "^1.19.1",
                        "react": "^18.2.0",
                        "react-dom": "^18.2.0"
                    }
                }"#;
                fs::write("package.json", package_json)?;
            },
            "astro" => {
                let package_json = r#"{
                    "name": "astro-app",
                    "type": "module",
                    "version": "0.0.1",
                    "scripts": {
                        "dev": "astro dev",
                        "start": "astro dev",
                        "build": "astro build",
                        "preview": "astro preview"
                    },
                    "dependencies": {
                        "astro": "^2.10.1"
                    }
                }"#;
                fs::write("package.json", package_json)?;
            },
            "vue" => {
                let package_json = r#"{
                    "name": "vue-app",
                    "version": "0.1.0",
                    "private": true,
                    "scripts": {
                        "serve": "vue-cli-service serve",
                        "build": "vue-cli-service build"
                    },
                    "dependencies": {
                        "core-js": "^3.8.3",
                        "vue": "^3.2.13"
                    }
                }"#;
                fs::write("package.json", package_json)?;
            },
            "nuxt" => {
                let package_json = r#"{
                    "name": "nuxt-app",
                    "private": true,
                    "scripts": {
                        "build": "nuxt build",
                        "dev": "nuxt dev",
                        "start": "nuxt start"
                    },
                    "dependencies": {
                        "nuxt": "^3.6.5"
                    }
                }"#;
                fs::write("package.json", package_json)?;
            },
            "mern" => {
                let package_json = r#"{
                    "name": "mern-app",
                    "version": "1.0.0",
                    "scripts": {
                        "start": "node index.js",
                        "dev": "nodemon index.js"
                    },
                    "dependencies": {
                        "express": "^4.18.2",
                        "mongoose": "^7.4.1",
                        "cors": "^2.8.5",
                        "dotenv": "^16.3.1"
                    },
                    "devDependencies": {
                        "nodemon": "^3.0.1"
                    }
                }"#;
                fs::write("package.json", package_json)?;

                let index_content = r#"const express = require('express');
const mongoose = require('mongoose');
const cors = require('cors');
require('dotenv').config();

const app = express();
app.use(cors());
app.use(express.json());

mongoose.connect(process.env.MONGODB_URI || 'mongodb://localhost/mern-app');

app.get('/api/test', (req, res) => {
    res.json({ message: 'MERN API is working!' });
});

const PORT = process.env.PORT || 3000;
app.listen(PORT, () => {
    console.log(`Server running on port ${PORT}`);
});"#;
                fs::write("index.js", index_content)?;
            },
            "node" => {
                let package_json = r#"{
                    "name": "node-app",
                    "version": "1.0.0",
                    "scripts": {
                        "start": "node index.js",
                        "dev": "nodemon index.js"
                    },
                    "dependencies": {
                        "express": "^4.18.2"
                    },
                    "devDependencies": {
                        "nodemon": "^3.0.1"
                    }
                }"#;
                fs::write("package.json", package_json)?;

                let index_content = r#"const express = require('express');
const app = express();

app.get('/', (req, res) => {
    res.send('Hello from Node.js!');
});

const PORT = process.env.PORT || 3000;
app.listen(PORT, () => {
    console.log(`Server running on port ${PORT}`);
});"#;
                fs::write("index.js", index_content)?;
            },
            "vanilla" => {
                let index_html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Vanilla JS App</title>
    <link rel="stylesheet" href="styles.css">
</head>
<body>
    <div id="app">
        <h1>Welcome to Vanilla JS App</h1>
    </div>
    <script src="app.js"></script>
</body>
</html>"#;
                fs::write("index.html", index_html)?;

                let styles_css = r#"body {
    font-family: Arial, sans-serif;
    margin: 0;
    padding: 20px;
    background-color: #f0f0f0;
}

#app {
    max-width: 800px;
    margin: 0 auto;
    background-color: white;
    padding: 20px;
    border-radius: 8px;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}"#;
                fs::write("styles.css", styles_css)?;

                let app_js = r#"document.addEventListener('DOMContentLoaded', () => {
    console.log('Vanilla JS App is running!');
});"#;
                fs::write("app.js", app_js)?;
            },
            _ => {}
        }
    }

    // Create or update .dockerignore
    let dockerignore_content = r#"
node_modules
npm-debug.log
Dockerfile
.dockerignore
.git
.gitignore
README.md
"#;
    fs::write(".dockerignore", dockerignore_content.trim())?;

    Ok(())
}

fn generate_dockerfile(app_type: &str) -> String {
    let base = r#"
FROM node:18-alpine AS builder

# Install system dependencies
RUN apk add --no-cache \
    bash \
    curl \
    nginx \
    supervisor \
    redis \
    postgresql-client

# Install Bun with version pinning
ARG BUN_VERSION=1.0.0
RUN curl -fsSL https://bun.sh/install | bash -s "bun-v${BUN_VERSION}"
ENV PATH="/root/.bun/bin:${PATH}"

# Set up performance monitoring
RUN npm install -g clinic autocannon

# Production optimizations
ENV NODE_ENV=production
ENV BUN_JS_ALLOCATIONS=1000000
ENV BUN_RUNTIME_CALLS=100000

WORKDIR /app

# Copy package files
COPY package*.json ./
"#;

    // Add framework-specific optimizations
    let framework_optimizations = match app_type {
        "nextjs" => r#"
# Next.js optimizations
RUN bun install --production
RUN bun run build

# Enable compression and caching
RUN bun add compression helmet redis connect-redis"#,
        // Add other framework optimizations...
        _ => ""
    };

    format!("{}{}", base, framework_optimizations)
}

fn generate_nginx_config(mode: &str) -> io::Result<()> {
    let worker_processes = if mode == "prod" { "auto" } else { "2" };
    let worker_connections = if mode == "prod" { "2048" } else { "1024" };

    let config = format!(r#"
user nginx;
worker_processes {worker_processes};
worker_rlimit_nofile 100000;
pid /var/run/nginx.pid;

events {{
    worker_connections {worker_connections};
    use epoll;
    multi_accept on;
}}

http {{
    include /etc/nginx/mime.types;
    default_type application/octet-stream;

    # Optimization
    sendfile on;
    tcp_nopush on;
    tcp_nodelay on;
    keepalive_timeout 65;
    keepalive_requests 100000;
    types_hash_max_size 2048;
    server_tokens off;

    # Buffer size
    client_body_buffer_size 128k;
    client_max_body_size 10m;
    client_header_buffer_size 1k;
    large_client_header_buffers 4 4k;
    output_buffers 1 32k;
    postpone_output 1460;

    # Timeouts
    client_header_timeout 3m;
    client_body_timeout 3m;
    send_timeout 3m;

    # Compression
    gzip on;
    gzip_min_length 1000;
    gzip_proxied expired no-cache no-store private auth;
    gzip_types text/plain text/css application/json application/javascript text/xml application/xml application/xml+rss text/javascript;

    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header Referrer-Policy "no-referrer-when-downgrade" always;
    add_header Content-Security-Policy "default-src 'self' http: https: data: blob: 'unsafe-inline'" always;
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;

    # Logging
    access_log /var/log/nginx/access.log combined buffer=512k flush=1m;
    error_log /var/log/nginx/error.log warn;

    upstream backend {{
        least_conn;
        server localhost:3000 max_fails=3 fail_timeout=30s;
        server localhost:3001 max_fails=3 fail_timeout=30s;
        keepalive 32;
    }}

    server {{
        listen 80;
        listen [::]:80;
        server_name _;

        location / {{
            proxy_pass http://backend;
            proxy_http_version 1.1;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection 'upgrade';
            proxy_set_header Host $host;
            proxy_cache_bypass $http_upgrade;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
            proxy_buffering on;
            proxy_buffer_size 128k;
            proxy_buffers 4 256k;
            proxy_busy_buffers_size 256k;
        }}

        location /health {{
            access_log off;
            return 200 'healthy\n';
        }}
    }}
}}"#);

    fs::write("nginx.conf", config)?;
    Ok(())
}

fn generate_supervisor_config() -> io::Result<()> {
    let supervisor_conf = r#"[supervisord]
nodaemon=true
logfile=/var/log/supervisord.log
pidfile=/var/run/supervisord.pid

[program:nginx]
command=nginx -g 'daemon off;'
autostart=true
autorestart=true
stdout_logfile=/dev/stdout
stdout_logfile_maxbytes=0
stderr_logfile=/dev/stderr
stderr_logfile_maxbytes=0

[program:app]
command=npm start
directory=/app
autostart=true
autorestart=true
stdout_logfile=/dev/stdout
stdout_logfile_maxbytes=0
stderr_logfile=/dev/stderr
stderr_logfile_maxbytes=0"#;

    fs::write("supervisord.conf", supervisor_conf)?;
    Ok(())
}

fn generate_docker_compose(app_name: &str, port: &str) -> String {
    format!(
        r#"version: '3.8'
services:
  {app_name}:
    build: .
    ports:
      - "{port}:{port}"
    environment:
      - PORT={port}
      - BUN_ENV=production
    healthcheck:
      test: ["CMD", "bun", "run", "index.js", "--health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 10s
    restart: unless-stopped
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3""#
    )
}

fn deploy_haproxy(namespace: &str) -> io::Result<()> {
    println!("{}", GradientText::cyber("📦 Deploying HAProxy..."));

    // Create HAProxy ConfigMap
    let haproxy_config = r#"
global
    daemon
    maxconn 256

defaults
    mode http
    timeout connect 5000ms
    timeout client 50000ms
    timeout server 50000ms

frontend http-in
    bind *:80
    default_backend servers

backend servers
    balance roundrobin
    option httpchk GET /health HTTP/1.1\r\nHost:\ localhost
    http-check expect status 200
    default-server inter 3s fall 3 rise 2
    server server1 127.0.0.1:8080 check weight 100 maxconn 3000
    server server2 127.0.0.1:8081 check weight 100 maxconn 3000
    server server3 127.0.0.1:8082 check weight 100 maxconn 3000
    compression algo gzip
    compression type text/plain text/css application/javascript"#;

    // Apply HAProxy ConfigMap
    let config_map = format!(r#"
apiVersion: v1
kind: ConfigMap
metadata:
  name: haproxy-config
  namespace: {}
data:
  haproxy.cfg: |
    {}
"#, namespace, haproxy_config);

    fs::write("haproxy-config.yaml", config_map)?;
    
    Command::new("kubectl")
        .args(["apply", "-f", "haproxy-config.yaml"])
        .output()?;

    // Deploy HAProxy
    let haproxy_deployment = format!(r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: haproxy
  namespace: {}
spec:
  replicas: 1
  selector:
    matchLabels:
      app: haproxy
  template:
    metadata:
      labels:
        app: haproxy
    spec:
      containers:
      - name: haproxy
        image: haproxy:2.4
        ports:
        - containerPort: 80
        volumeMounts:
        - name: config
          mountPath: /usr/local/etc/haproxy/
      volumes:
      - name: config
        configMap:
          name: haproxy-config
"#, namespace);

    fs::write("haproxy-deployment.yaml", haproxy_deployment)?;

    Command::new("kubectl")
        .args(["apply", "-f", "haproxy-deployment.yaml"])
        .output()?;

    // Create HAProxy Service
    let haproxy_service = format!(r#"
apiVersion: v1
kind: Service
metadata:
  name: haproxy
  namespace: {}
spec:
  type: LoadBalancer
  ports:
  - port: 80
    targetPort: 80
  selector:
    app: haproxy
"#, namespace);

    fs::write("haproxy-service.yaml", haproxy_service)?;

    Command::new("kubectl")
        .args(["apply", "-f", "haproxy-service.yaml"]) 
        .output()?;

    println!("{}", GradientText::success("✅ HAProxy deployed successfully"));
    Ok(())
}

fn deploy_to_kubernetes(
    metadata: &mut AppMetadata,
    app_name: &str,
    app_type: &str,
    port: &str,
    replicas: i32,
    namespace: &str,
    mode: &str,
) -> io::Result<()> {
    println!("{}", GradientText::cyber("🎡 Initializing Kubernetes deployment..."));

    // Ensure Kubernetes is ready
    verify_kubernetes_setup()?;

    // Create namespace with advanced configuration
    create_namespace_with_quotas(namespace, mode)?;

    // Deploy HAProxy
    deploy_haproxy(namespace)?;

    // Generate and apply Kubernetes manifests
    generate_kubernetes_manifests(app_name, app_type, port, replicas, namespace, mode)?;
    apply_kubernetes_manifests(namespace)?;

    // Set up monitoring and advanced networking
    setup_monitoring(app_name, namespace, mode)?;
    setup_network_policies(app_name, namespace)?;

    // Configure auto-scaling
    if mode == "prod" {
        setup_autoscaling(app_name, namespace, replicas)?;
    }

    // Wait for deployment
    wait_for_kubernetes_deployment(&metadata.kubernetes_metadata.deployment_name, namespace)?;

    // Update status
    update_pod_status(metadata, namespace)?;

    println!("{}", GradientText::success("✅ Kubernetes deployment completed successfully!"));
    print_kubernetes_status(metadata);

    Ok(())
}

fn verify_docker_installation() -> io::Result<()> {
    println!("{}", GradientText::cyber("🔍 Verifying Docker installation..."));
    match Command::new("docker").arg("--version").output() {
        Ok(output) => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("{}", GradientText::success(&format!("✅ Docker installed: {}", version.trim())));
            
            // Check if Docker Desktop is running
            match Command::new("docker").arg("info").output() {
                Ok(output) if output.status.success() => {
                    println!("{}", GradientText::success("✅ Docker Desktop is running"));
                }
                _ => {
                    println!("{}", GradientText::warning("⏳ Docker Desktop is not running. Attempting to start..."));
                    let docker_manager = DockerManager::new();
                    docker_manager.launch_docker_desktop()?;
                }
            }
            Ok(())
        }
        Err(_) => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Docker is not installed. Please install Docker Desktop first: https://www.docker.com/products/docker-desktop"
        )),
    }
}

fn verify_container_status(container_id: &str) -> io::Result<()> {
    let output = Command::new("docker")
        .args(["inspect", "-f", "{{.State.Running}}", container_id])
        .output()?;

    if String::from_utf8_lossy(&output.stdout).trim() != "true" {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Container is not running",
        ));
    }

    println!("{}", GradientText::cyber("⏳ Waiting for container health check..."));
    std::thread::sleep(std::time::Duration::from_secs(5));

    let health_output = Command::new("docker")
        .args(["inspect", "-f", "{{.State.Health.Status}}", container_id])
        .output()?;

    let health_status = String::from_utf8_lossy(&health_output.stdout).trim().to_string();
    if health_status != "healthy" {
        println!("{}", GradientText::warning(&format!("⚠️  Container health status: {}", health_status)));
    } else {
        println!("{}", GradientText::success("✅ Container is healthy"));
    }

    Ok(())
}

fn save_metadata(metadata: &AppMetadata) -> io::Result<()> {
    let json = serde_json::to_string_pretty(metadata)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    fs::write(".container-metadata.json", json)
}

fn generate_kubernetes_manifests(app_name: &str, _app_type: &str, port: &str, replicas: i32, namespace: &str, mode: &str) -> io::Result<()> {
    let resources = if mode == "prod" {
        r#"
        resources:
          requests:
            cpu: "1"
            memory: "2Gi"
          limits:
            cpu: "2"
            memory: "4Gi""#
    } else {
        r#"
        resources:
          requests:
            cpu: "500m"
            memory: "512Mi"
          limits:
            cpu: "1"
            memory: "1Gi""#
    };

    let deployment = format!(
        r#"apiVersion: apps/v1
kind: Deployment
metadata:
  name: {app_name}-deployment
  namespace: {namespace}
spec:
  replicas: {replicas}
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 25%
      maxUnavailable: 25%
  selector:
    matchLabels:
      app: {app_name}
  template:
    metadata:
      labels:
        app: {app_name}
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "{port}"
    spec:
      containers:
      - name: {app_name}
        image: {app_name}:latest
        imagePullPolicy: Never
        ports:
        - containerPort: {port}
          protocol: TCP
        env:
        - name: PORT
          value: "{port}"
        - name: NODE_ENV
          value: "{mode}"
        {resources}
        livenessProbe:
          httpGet:
            path: /health
            port: {port}
          initialDelaySeconds: 15
          periodSeconds: 20
          timeoutSeconds: 5
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /health
            port: {port}
          initialDelaySeconds: 5
          periodSeconds: 10
          timeoutSeconds: 3
          successThreshold: 1
          failureThreshold: 3
        startupProbe:
          httpGet:
            path: /health
            port: {port}
          failureThreshold: 30
          periodSeconds: 10
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
      topologySpreadConstraints:
      - maxSkew: 1
        topologyKey: kubernetes.io/hostname
        whenUnsatisfied: DoNotSchedule
        labelSelector:
          matchLabels:
            app: {app_name}"#
    );

    fs::write("k8s-deployment.yaml", deployment)?;
    
    // Generate service with session affinity
    let service = format!(
        r#"apiVersion: v1
kind: Service
metadata:
  name: {app_name}-service
  namespace: {namespace}
  annotations:
    prometheus.io/scrape: "true"
    prometheus.io/port: "{port}"
spec:
  selector:
    app: {app_name}
  ports:
  - port: {port}
    targetPort: {port}
  sessionAffinity: ClientIP
  sessionAffinityConfig:
    clientIP:
      timeoutSeconds: 10800
  type: ClusterIP"#
    );

    fs::write("k8s-service.yaml", service)?;
    Ok(())
}

fn apply_kubernetes_manifests(namespace: &str) -> io::Result<()> {
    // Create namespace if it doesn't exist
    Command::new("kubectl")
        .args(["create", "namespace", namespace, "--dry-run=client", "-o", "yaml"])
        .output()?;

    // Apply manifests
    Command::new("kubectl")
        .args(["apply", "-f", "k8s-deployment.yaml"])
        .output()?;

    Command::new("kubectl")
        .args(["apply", "-f", "k8s-service.yaml"])
        .output()?;

    Ok(())
}

fn wait_for_kubernetes_deployment(deployment_name: &str, namespace: &str) -> io::Result<()> {
    println!("⏳ Waiting for deployment to be ready...");
    
    let status = Command::new("kubectl")
        .args([
            "rollout",
            "status",
            "deployment",
            deployment_name,
            "-n",
            namespace,
            "--timeout=300s",
        ])
        .status()?;

    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Deployment failed to roll out",
        ));
    }

    Ok(())
}

fn update_pod_status(metadata: &mut AppMetadata, namespace: &str) -> io::Result<()> {
    let output = Command::new("kubectl")
        .args([
            "get",
            "pods",
            "-l",
            &format!("app={}", metadata.app_name),
            "-n",
            namespace,
            "-o",
            "jsonpath={.items[*].status.phase}",
        ])
        .output()?;

    metadata.kubernetes_metadata.pod_status = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .map(String::from)
        .collect();

    Ok(())
}

fn create_kubernetes_ingress(app_name: &str, port: &str, namespace: &str, mode: &str) -> io::Result<String> {
    let ingress = format!(
        r#"apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: {app_name}-ingress
  namespace: {namespace}
  annotations:
    nginx.ingress.kubernetes.io/rewrite-target: /
spec:
  rules:
  - host: {app_name}.local
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: {app_name}-service
            port:
              number: {port}"#
    );

    fs::write("k8s-ingress.yaml", ingress)?;

    Command::new("kubectl")
        .args(["apply", "-f", "k8s-ingress.yaml"])
        .output()?;

    Ok(format!("{}.local", app_name))
}

fn print_kubernetes_status(metadata: &AppMetadata) {
    println!("\n{}", GradientText::cyber("📊 Kubernetes Status:"));
    println!("{}", GradientText::status(&format!("   • Namespace: {}", metadata.kubernetes_metadata.namespace)));
    println!("{}", GradientText::status(&format!("   • Deployment: {}", metadata.kubernetes_metadata.deployment_name)));
    println!("{}", GradientText::status(&format!("   • Service: {}", metadata.kubernetes_metadata.service_name)));
    println!("{}", GradientText::status(&format!("   • Replicas: {}", metadata.kubernetes_metadata.replicas)));
    println!("{}", GradientText::status(&format!("   • Pod Status: {:?}", metadata.kubernetes_metadata.pod_status)));
    if let Some(host) = &metadata.kubernetes_metadata.ingress_host {
        println!("{}", GradientText::status(&format!("   • Ingress Host: {}", host)));
    }
}

fn verify_infrastructure() -> io::Result<()> {
    println!("🔍 Verifying infrastructure...");

    // Check Docker
    println!("\n📦 Checking Docker...");
    match Command::new("docker").arg("--version").output() {
        Ok(output) => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("{}", GradientText::success(&format!("✅ Docker installed: {}", version.trim())));
            
            // Check if Docker daemon is running
            match Command::new("docker").args(["ps"]).output() {
                Ok(_) => println!("✅ Docker daemon is running"),
                Err(_) => return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Docker daemon is not running. Please start Docker Desktop or docker service"
                )),
            }
        }
        Err(_) => return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Docker is not installed or not in PATH"
        )),
    }

    // Check Kubernetes context
    println!("\n☸️  Checking Kubernetes...");
    
    // Ensure we're using docker-desktop context
    Command::new("kubectl")
        .args(["config", "use-context", "docker-desktop"])
        .output()?;

    // Check kubectl installation and connection
    match Command::new("kubectl").args(["cluster-info", "dump"]).output() {
        Ok(output) => {
            if output.status.success() {
                println!("✅ Connected to Kubernetes cluster (docker-desktop)");
                
                // Verify core components
                let core_namespaces = Command::new("kubectl")
                    .args(["get", "namespaces"])
                    .output()?;
                println!("\n📊 Available Namespaces:");
                println!("{}", String::from_utf8_lossy(&core_namespaces.stdout));

                // Check if nginx ingress controller is installed
                let ingress_pods = Command::new("kubectl")
                    .args(["get", "pods", "-n", "ingress-nginx"])
                    .output();
                
                if ingress_pods.is_err() {
                    println!("\n⚠️  Nginx Ingress Controller not found. Installing...");
                    install_nginx_ingress()?;
                }
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Kubernetes cluster is not ready"
                ));
            }
        }
        Err(_) => return Err(io::Error::new(
            io::ErrorKind::Other,
            "Cannot connect to Kubernetes cluster"
        )),
    }

    Ok(())
}

fn install_nginx_ingress() -> io::Result<()> {
    // Add Nginx Ingress Controller repository
    Command::new("kubectl")
        .args([
            "apply",
            "-f",
            "https://raw.githubusercontent.com/kubernetes/ingress-nginx/controller-v1.8.2/deploy/static/provider/cloud/deploy.yaml"
        ])
        .output()?;

    // Wait for the ingress controller to be ready
    println!("⏳ Waiting for Nginx Ingress Controller to be ready...");
    Command::new("kubectl")
        .args([
            "wait",
            "--namespace", "ingress-nginx",
            "--for=condition=ready", "pod",
            "--selector=app.kubernetes.io/component=controller",
            "--timeout=300s"
        ])
        .output()?;

    println!("✅ Nginx Ingress Controller installed successfully");
    Ok(())
}

fn prepare_kubernetes_deployment(app_name: &str, mode: &str) -> io::Result<()> {
    // Tag the image for Kubernetes
    Command::new("docker")
        .args(["tag", &format!("rust-dockerize-{}", app_name), &format!("{}:latest", app_name)])
        .output()?;

    println!("✅ Docker image tagged for Kubernetes");
    Ok(())
}

fn create_namespace_with_quotas(namespace: &str, mode: &str) -> io::Result<()> {
    // Create namespace with resource quotas based on mode
    let quota_spec = if mode == "prod" {
        r#"
apiVersion: v1
kind: ResourceQuota
metadata:
  name: compute-quota
spec:
  hard:
    requests.cpu: "4"
    requests.memory: 8Gi
    limits.cpu: "8"
    limits.memory: 16Gi""#
    } else {
        r#"
apiVersion: v1
kind: ResourceQuota
metadata:
  name: compute-quota
spec:
  hard:
    requests.cpu: "1"
    requests.memory: 2Gi
    limits.cpu: "2"
    limits.memory: 4Gi""#
    };

    fs::write("quota.yaml", quota_spec)?;
    
    // Create namespace and apply quota
    Command::new("kubectl")
        .args(["create", "namespace", namespace, "--dry-run=client", "-o", "yaml"])
        .output()?;
    
    Command::new("kubectl")
        .args(["apply", "-f", "quota.yaml", "-n", namespace])
        .output()?;

    Ok(())
}

fn setup_monitoring(app_name: &str, namespace: &str, mode: &str) -> io::Result<()> {
    // Configure Prometheus monitoring
    let monitoring_config = format!(
        r#"apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: {app_name}-monitor
  namespace: {namespace}
spec:
  selector:
    matchLabels:
      app: {app_name}
  endpoints:
  - port: metrics"#
    );

    fs::write("monitoring.yaml", monitoring_config)?;
    Command::new("kubectl")
        .args(["apply", "-f", "monitoring.yaml"])
        .output()?;

    Ok(())
}

fn setup_autoscaling(app_name: &str, namespace: &str, min_replicas: i32) -> io::Result<()> {
    let hpa_config = format!(
        r#"apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: {app_name}-hpa
  namespace: {namespace}
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: {app_name}-deployment
  minReplicas: {min_replicas}
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70"#
    );

    fs::write("hpa.yaml", hpa_config)?;
    Command::new("kubectl")
        .args(["apply", "-f", "hpa.yaml"])
        .output()?;

    Ok(())
}

fn cleanup_deployment() -> io::Result<()> {
    println!("{}", GradientText::cyber("🧹 Starting cleanup process..."));

    if let Ok(metadata_content) = fs::read_to_string(".container-metadata.json") {
        if let Ok(metadata) = serde_json::from_str::<AppMetadata>(&metadata_content) {
            println!("{}", GradientText::info("🔄 Stopping containers..."));
            Command::new("docker")
                .args(["compose", "down", "--remove-orphans"])
                .output()?;

            println!("{}", GradientText::info("🗑️  Removing Docker images..."));
            Command::new("docker")
                .args(["rmi", &format!("rust-dockerize-{}", metadata.app_name), &format!("{}:latest", metadata.app_name)])
                .output()?;

            if !metadata.kubernetes_metadata.namespace.is_empty() {
                println!("{}", GradientText::info("☸️  Cleaning up Kubernetes resources..."));
                Command::new("kubectl")
                    .args(["delete", "-f", "k8s-deployment.yaml", "--ignore-not-found"])
                    .output()?;
                Command::new("kubectl")
                    .args(["delete", "-f", "k8s-service.yaml", "--ignore-not-found"])
                    .output()?;
                Command::new("kubectl")
                    .args(["delete", "-f", "k8s-ingress.yaml", "--ignore-not-found"])
                    .output()?;
            }
        }
    }

    println!("{}", GradientText::info("🗑️  Removing generated files..."));
    let files_to_remove = [
        "Dockerfile", "docker-compose.yml", ".container-metadata.json",
        "k8s-deployment.yaml", "k8s-service.yaml", "k8s-ingress.yaml"
    ];
    for file in files_to_remove {
        let _ = fs::remove_file(file);
    }

    println!("{}", GradientText::success("✅ Cleanup completed successfully"));
    Ok(())
}

fn verify_kubernetes_connection() -> io::Result<()> {
    println!("🔍 Verifying Kubernetes connection...");

    // First, check if Docker Desktop is running
    if let Err(_) = Command::new("docker").arg("info").output() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Docker Desktop is not running. Please start Docker Desktop first."
        ));
    }

    // Try to get the current context
    let context_output = Command::new("kubectl")
        .args(["config", "current-context"])
        .output()?;

    if !context_output.status.success() {
        // If no context is set, try to set docker-desktop context
        println!("⚠️  No Kubernetes context set. Attempting to set docker-desktop context...");
        
        // List available contexts
        let contexts_output = Command::new("kubectl")
            .args(["config", "get-contexts", "-o", "name"])
            .output()?;
        
        let contexts = String::from_utf8_lossy(&contexts_output.stdout);
        
        if contexts.contains("docker-desktop") {
            Command::new("kubectl")
                .args(["config", "use-context", "docker-desktop"])
                .output()?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Docker Desktop Kubernetes is not enabled. Please enable it in Docker Desktop settings."
            ));
        }
    }

    // Verify cluster connectivity with retry
    for i in 0..3 {
        if i > 0 {
            println!("⏳ Retrying connection ({}/3)...", i + 1);
            std::thread::sleep(std::time::Duration::from_secs(5));
        }

        match Command::new("kubectl").args(["cluster-info"]).output() {
            Ok(output) if output.status.success() => {
                println!("✅ Successfully connected to Kubernetes cluster");
                return Ok(());
            }
            _ if i == 2 => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Failed to connect to Kubernetes cluster after 3 attempts. Please check:\n\
                     1. Docker Desktop is running\n\
                     2. Kubernetes is enabled in Docker Desktop settings\n\
                     3. Kubernetes is running (green icon in Docker Desktop)"
                ));
            }
            _ => continue,
        }
    }

    Ok(())
}

fn check_kubernetes_status() -> io::Result<()> {
    println!("📊 Checking Kubernetes status...");
    
    // Check Docker Desktop status
    println!("\n🐳 Docker Desktop status:");
    match Command::new("docker").arg("info").output() {
        Ok(output) => {
            if output.status.success() {
                println!("✅ Docker Desktop is running");
            } else {
                println!("❌ Docker Desktop is not running properly");
            }
        }
        Err(_) => println!("❌ Docker Desktop is not running"),
    }

    // Check Kubernetes status
    println!("\n☸️  Kubernetes status:");
    match Command::new("kubectl").args(["cluster-info"]).output() {
        Ok(output) => {
            if output.status.success() {
                println!("✅ Kubernetes is running");
                
                // Show component status
                if let Ok(components) = Command::new("kubectl")
                    .args(["get", "componentstatuses", "-o", "wide"])
                    .output() 
                {
                    println!("\nComponent Status:");
                    println!("{}", String::from_utf8_lossy(&components.stdout));
                }
            } else {
                println!("❌ Kubernetes is not running properly");
            }
        }
        Err(_) => println!("❌ Kubernetes is not running"),
    }

    Ok(())
}

fn generate_haproxy_config(mode: &str) -> io::Result<()> {
    let config = format!(r#"global
    maxconn 100000
    log /dev/log local0
    user haproxy
    group haproxy
    daemon
    nbproc 4
    cpu-map auto:1/1-4 0-3
    ssl-default-bind-ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256
    ssl-default-bind-options no-sslv3 no-tlsv10 no-tlsv11

defaults
    log global
    mode http
    option httplog
    option dontlognull
    option forwardfor
    option http-server-close
    timeout connect 5000
    timeout client 50000
    timeout server 50000
    timeout http-request 15s
    timeout http-keep-alive 15s

frontend stats
    bind *:8404
    stats enable
    stats uri /stats
    stats refresh 10s
    stats admin if LOCALHOST

frontend http_front
    bind *:80
    bind *:443 ssl crt /etc/ssl/private/cert.pem
    http-request redirect scheme https unless {{ ssl_fc }}
    
    # Advanced security headers
    http-response set-header Strict-Transport-Security "max-age=31536000; includeSubDomains; preload"
    http-response set-header X-Frame-Options "SAMEORIGIN"
    http-response set-header X-Content-Type-Options "nosniff"
    
    # Rate limiting
    stick-table type ip size 100k expire 30s store conn_cur,conn_rate(3s),http_req_rate(10s)
    http-request track-sc0 src
    http-request deny deny_status 429 if {{ sc_http_req_rate(0) gt 10 }}
    
    default_backend apps

backend apps
    balance roundrobin
    option httpchk HEAD /health HTTP/1.1\r\nHost:\ localhost
    http-check expect status 200
    server app1 127.0.0.1:3000 check weight 100 maxconn 3000
    server app2 127.0.0.1:3001 check weight 100 maxconn 3000
    "#
    );

    fs::write("haproxy.cfg", config)?;
    Ok(())
}

fn setup_network_policies(app_name: &str, namespace: &str) -> io::Result<()> {
    let network_policy = format!(
        r#"apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: {app_name}-network-policy
  namespace: {namespace}
spec:
  podSelector:
    matchLabels:
      app: {app_name}
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: {namespace}
    ports:
    - protocol: TCP
      port: 80
  egress:
  - to:
    - namespaceSelector:
        matchLabels:
          name: kube-system
    ports:
    - protocol: TCP"#
    );

    fs::write("network-policy.yaml", network_policy)?;
    
    Command::new("kubectl")
        .args(["apply", "-f", "network-policy.yaml"])
        .output()?;

    Ok(())
}

fn verify_kubernetes_setup() -> io::Result<()> {
    println!("{}", GradientText::cyber("🔍 Verifying Kubernetes setup..."));

    // Check if kubectl is installed
    match Command::new("kubectl").arg("version").output() {
        Ok(_) => println!("{}", GradientText::success("✅ kubectl is installed")),
        Err(_) => return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "kubectl is not installed. Please install kubectl first."
        )),
    }

    // Check if Kubernetes is running
    match Command::new("kubectl").args(["cluster-info"]).output() {
        Ok(output) if output.status.success() => {
            println!("{}", GradientText::success("✅ Kubernetes cluster is running"));
        },
        _ => return Err(io::Error::new(
            io::ErrorKind::Other,
            "Kubernetes cluster is not running. Please start your Kubernetes cluster."
        )),
    }

    Ok(())
}

// Add high-performance caching layer
async fn setup_caching(cache: Arc<DashMap<String, Vec<u8>>>) -> io::Result<()> {
    // Initialize Redis connection
    let redis_config = r#"
maxmemory 2gb
maxmemory-policy allkeys-lru
activerehashing yes
appendonly yes
appendfsync everysec
no-appendfsync-on-rewrite yes
auto-aof-rewrite-percentage 100
auto-aof-rewrite-min-size 64mb
"#;
    fs::write("redis.conf", redis_config)?;
    
    Ok(())
}

// Enhanced load balancing configuration
async fn setup_load_balancing(mode: &str) -> io::Result<()> {
    let haproxy_config = format!(
        r#"global
    maxconn 100000
    ssl-server-verify none
    tune.ssl.default-dh-param 2048
    stats socket /var/run/haproxy.sock mode 600 level admin
    stats timeout 2m

defaults
    mode http
    timeout connect 10s
    timeout client 30s
    timeout server 30s
    option httplog
    option dontlognull
    option http-server-close
    option forwardfor except 127.0.0.0/8
    option redispatch
    retries 3
    maxconn 3000

frontend main
    bind *:80
    bind *:443 ssl crt /etc/ssl/private/cert.pem
    http-request redirect scheme https unless {{ ssl_fc }}
    
    # Advanced security headers
    http-response set-header Strict-Transport-Security "max-age=31536000; includeSubDomains; preload"
    http-response set-header X-Frame-Options "SAMEORIGIN"
    http-response set-header X-Content-Type-Options "nosniff"
    
    # Rate limiting
    stick-table type ip size 100k expire 30s store conn_cur,conn_rate(3s),http_req_rate(10s)
    http-request track-sc0 src
    http-request deny deny_status 429 if {{ sc_http_req_rate(0) gt 10 }}
    
    default_backend apps

backend apps
    balance roundrobin
    option httpchk HEAD /health HTTP/1.1\r\nHost:\ localhost
    http-check expect status 200
    server app1 127.0.0.1:3000 check weight 100 maxconn 3000
    server app2 127.0.0.1:3001 check weight 100 maxconn 3000
    "#
    );
    
    fs::write("haproxy.cfg", haproxy_config)?;
    Ok(())
}

fn default_namespace() -> String {
    "default".to_string()
}

fn default_min_instances() -> u32 {
    1
}

fn default_max_instances() -> u32 {
    5
}
