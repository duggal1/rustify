use std::{env, fs, io::{self, Write}, path::Path, process::Command};
use chrono::Local;
use dashmap::DashMap;
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
use clap::{App, SubCommand, Arg};

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

    fn verify_and_setup_docker(&self) -> io::Result<()> {
        println!("🔍 Checking Docker installation...");

        // First check if Docker is installed
        match Command::new("docker").arg("--version").output() {
            Ok(_) => {
                println!("✅ Docker is installed");
                
                // Then check if Docker is running
                match Command::new("docker").arg("info").output() {
                    Ok(_) => {
                        println!("✅ Docker is running");
                        Ok(())
                    }
                    Err(_) => {
                        println!("⏳ Starting Docker...");
                        self.start_docker()?;
                        Ok(())
                    }
                }
            }
            Err(_) => {
                println!("❌ Docker not found. Installing Docker...");
                self.install_docker()?;
                println!("⏳ Starting Docker for first time...");
                self.start_docker()?;
                Ok(())
            }
        }
    }

    fn start_docker(&self) -> io::Result<()> {
        #[cfg(target_os = "macos")]
        {
            Command::new("open").args(["-a", "Docker"]).status()?;
        }

        #[cfg(target_os = "windows")] 
        {
            Command::new("cmd")
                .args(["/C", "start", "\"\"", "\"C:\\Program Files\\Docker\\Docker\\Docker Desktop.exe\""])
                .status()?;
        }

        #[cfg(target_os = "linux")]
        {
            Command::new("systemctl").args(["--user", "start", "docker"]).status()?;
        }

        // Wait for Docker to be ready
        println!("⏳ Waiting for Docker to start...");
        for _ in 0..30 {
            match Command::new("docker").arg("info").output() {
                Ok(_) => {
                    println!("✅ Docker is now running!");
                    return Ok(());
                }
                Err(_) => {
                    thread::sleep(Duration::from_secs(2));
                    print!(".");
                    io::stdout().flush()?;
                }
            }
        }

        Err(io::Error::new(io::ErrorKind::Other, "Docker failed to start"))
    }

    fn stop_docker(&self) -> io::Result<()> {
        println!("Stopping Docker...");

        #[cfg(target_os = "macos")]
        {
            Command::new("osascript")
                .args(["-e", "quit app \"Docker\""])
                .status()?;
        }

        #[cfg(target_os = "windows")]
        {
            Command::new("taskkill")
                .args(["/IM", "Docker Desktop.exe", "/F"])
                .status()?;
        }

        #[cfg(target_os = "linux")]
        {
            Command::new("systemctl")
                .args(["--user", "stop", "docker"])
                .status()?;
        }

        println!("✅ Docker stopped");
        Ok(())
    }

    fn install_docker(&self) -> io::Result<()> {
        println!("📥 Installing Docker...");

        #[cfg(target_os = "macos")]
        {
            Command::new("brew")
                .args(["install", "--cask", "docker"])
                .status()?;
        }

        #[cfg(target_os = "windows")]
        {
            let installer_url = "https://desktop.docker.com/win/main/amd64/Docker%20Desktop%20Installer.exe";
            Command::new("powershell")
                .args([
                    "-Command",
                    &format!("Invoke-WebRequest '{}' -OutFile 'DockerInstaller.exe'", installer_url)
                ])
                .status()?;

            Command::new("DockerInstaller.exe")
                .args(["install", "--quiet"])
                .status()?;

            fs::remove_file("DockerInstaller.exe")?;
        }

        #[cfg(target_os = "linux")]
        {
            Command::new("sudo")
                .args(["apt-get", "update"])
                .status()?;

            Command::new("sudo")
                .args(["apt-get", "install", "-y", "docker.io"])
                .status()?;

            Command::new("sudo")
                .args(["systemctl", "enable", "docker"])
                .status()?;

            Command::new("sudo") 
                .args(["usermod", "-aG", "docker", &whoami::username()])
                .status()?;
        }

        println!("✅ Docker installed successfully");
        println!("⚠️  You may need to restart your system");
        Ok(())
    }

    fn launch_docker_desktop(&self) -> io::Result<()> {
        #[cfg(target_os = "macos")]
        {
            Command::new("open")
                .args(["-a", "Docker"])
                .status()?;
        }

        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .args(["/C", "start", "\"\"", "\"C:\\Program Files\\Docker\\Docker\\Docker Desktop.exe\""])
                .status()?;
        }

        #[cfg(target_os = "linux")]
        {
            Command::new("systemctl")
                .args(["--user", "start", "docker"])
                .status()?;
        }

        // Wait for Docker to be ready
        println!("⏳ Waiting for Docker Desktop to start...");
        for _ in 0..30 {
            match Command::new("docker").arg("info").output() {
                Ok(_) => {
                    println!("✅ Docker Desktop is now running!");
                    return Ok(());
                }
                Err(_) => {
                    thread::sleep(Duration::from_secs(2));
                    print!(".");
                    io::stdout().flush()?;
                }
            }
        }

        Err(io::Error::new(io::ErrorKind::Other, "Docker Desktop failed to start"))
    }

    fn check_docker_setup(&self) -> io::Result<()> {
        match Command::new("docker").arg("--version").output() {
            Ok(_) => {
                println!("✅ Docker is installed");
                Ok(())
            }
            Err(_) => {
                println!("❌ Docker not found. Installing Docker...");
                self.install_docker()?;
                println!("⏳ Starting Docker for first time...");
                self.start_docker()?;
                Ok(())
            }
        }
    }
}

fn main() {
    let app = App::new("rustify")
        .version("0.1.0")
        .author("Harshit Duggal")
        .about("🚀 Ultra-optimized deployment CLI")
        .subcommand(SubCommand::with_name("init")
            .about("Initialize project")
            .arg(Arg::with_name("type")
                .long("type")
                .value_name("TYPE")
                .help("Project type (default: bun)")
                .takes_value(true)))
        .subcommand(SubCommand::with_name("deploy")
            .about("Deploy application")
            .arg(Arg::with_name("prod")
                .long("prod")
                .help("Deploy in production mode"))
            .arg(Arg::with_name("port")
                .long("port")
                .value_name("PORT")
                .help("Custom port (default: 8000)"))
            .arg(Arg::with_name("rpl")
                .long("rpl")
                .help("Enable auto-scaling replicas")))
        .get_matches();

    match app.subcommand() {
        Some(("init", matches)) => {
            let project_type = matches.value_of("type").unwrap_or("bun");
            if let Err(e) = initialize_project(project_type) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(("deploy", matches)) => {
            let is_prod = matches.is_present("prod");
            let port = matches.value_of("port").unwrap_or("8000");
            let auto_scale = matches.is_present("rpl");
            
            // Create metadata
            let mut metadata = AppMetadata {
                app_name: "app".to_string(),
                app_type: "bun".to_string(),
                port: port.to_string(),
                created_at: Local::now().to_rfc3339(),
                container_id: None,
                status: String::from("pending"),
                kubernetes_enabled: is_prod,
                kubernetes_metadata: KubernetesMetadata {
                    namespace: if is_prod { "production" } else { "development" }.to_string(),
                    deployment_name: "app-deployment".to_string(),
                    service_name: "app-service".to_string(),
                    replicas: if auto_scale { 3 } else { 1 },
                    pod_status: vec![],
                    ingress_host: None,
                },
                performance_metrics: PerformanceMetrics::default(),
                scaling_config: ScalingConfig::default(),
            };

            // Keep all the powerful infrastructure logic
            if let Err(e) = deploy_application(&mut metadata, is_prod, auto_scale) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        _ => {
            println!("Usage:");
            println!("  rustify init [--type <type>]");
            println!("  rustify deploy [--prod] [--port <port>] [--rpl]");
        }
    }
}

fn deploy_application(metadata: &mut AppMetadata, is_prod: bool, auto_scale: bool) -> io::Result<()> {
    println!("🚀 Starting enterprise-grade deployment...");

    // Initialize Docker manager and verify setup
    let docker_manager = DockerManager::new();
    docker_manager.handle_docker_setup()?;

    // First verify all infrastructure
    verify_docker_installation()?;
    verify_kubernetes_setup()?;
    verify_infrastructure()?;
    verify_kubernetes_connection()?;
    check_kubernetes_status()?;
    // Generate necessary configuration files
    let dockerfile = generate_dockerfile(&metadata.app_type);
    fs::write("Dockerfile", dockerfile)?;
    let config = generate_supervisor_config();
    fs::write("supervisord.conf", config)?;
    let docker_compose = generate_docker_compose(&metadata.app_name, &metadata.port);
    fs::write("docker-compose.yml", docker_compose)?;

    // Build and verify container
    println!("🏗️ Building container...");
    Command::new("docker")
        .args(["build", "-t", &format!("rust-dockerize-{}", metadata.app_name), "."])
        .status()?;

    // Start container and verify status
    let container_id = Command::new("docker")
        .args(["run", "-d", &format!("rust-dockerize-{}", metadata.app_name)])
        .output()?;
    let container_id = String::from_utf8_lossy(&container_id.stdout).trim().to_string();
    metadata.container_id = Some(container_id.clone());
    
    verify_container_status(&container_id)?;

    // Initialize async runtime
    let rt = Runtime::new()?;
    
    // Prepare Kubernetes environment
    prepare_kubernetes_deployment(&metadata.app_name, if is_prod { "prod" } else { "dev" })?;
    create_namespace_with_quotas(
        &metadata.kubernetes_metadata.namespace, 
        if is_prod { "prod" } else { "dev" }
    )?;

    // Generate and apply Kubernetes manifests
    generate_kubernetes_manifests(
        &metadata.app_name,
        &metadata.app_type,
        &metadata.port,
        metadata.kubernetes_metadata.replicas,
        &metadata.kubernetes_metadata.namespace,
        if is_prod { "prod" } else { "dev" }
    )?;
    
    apply_kubernetes_manifests(&metadata.kubernetes_metadata.namespace)?;

    // Deploy to Kubernetes if in production
    if is_prod {
        let app_name = metadata.app_name.clone();
        let app_type = metadata.app_type.clone();
        let port = metadata.port.clone();
        let replicas = metadata.kubernetes_metadata.replicas;
        let namespace = metadata.kubernetes_metadata.namespace.clone();
        deploy_to_kubernetes(
            metadata, // Changed from &metadata to metadata since function expects &mut AppMetadata
            &app_name,
            &app_type,
            &port,
            replicas,
            &namespace,
            "prod"
        )?;

        // Setup enterprise infrastructure
        rt.block_on(async {
            setup_security_layer(&app_name, &namespace).await?;
            setup_monitoring(&app_name, &namespace, "prod").await?;
            setup_caching_layer().await?;
            setup_load_balancing("prod").await?;
            
            Ok::<(), io::Error>(())
        })?;

        // Production optimizations
        optimize_kernel_parameters()?;
        optimize_bun_runtime()?;
        enhance_load_balancer_config()?;
        
        if auto_scale {
            setup_autoscaling(
                &app_name,
                &namespace,
                replicas
            )?;
        }
        
        install_nginx_ingress()?;
        let ingress_host = create_kubernetes_ingress(
            &app_name,
            &port,
            &namespace,
            "prod"
        )?;
        metadata.kubernetes_metadata.ingress_host = Some(ingress_host);
        
        deploy_haproxy(&namespace)?;
        deploy_nginx(&namespace)?;
        setup_network_policies(&app_name, &namespace)?;
    } else {
        // Development setup
        println!("🛠️ Deploying development environment...");
        generate_nginx_config("dev")?;
        generate_haproxy_config("dev")?;
        
        Command::new("docker-compose")
            .args(["up", "-d"])
            .status()?;
    }

    // Wait for deployment and update status
    let deployment_name = metadata.kubernetes_metadata.deployment_name.clone();
    let namespace = metadata.kubernetes_metadata.namespace.clone();
    
    wait_for_kubernetes_deployment(&deployment_name, &namespace)?;
    update_pod_status(metadata, &namespace)?;
    print_kubernetes_status(&metadata);
    save_metadata(metadata)?;

    println!("✅ Enterprise deployment complete!");
    Ok(())
}

// Add cleanup functionality
impl Drop for AppMetadata {
    fn drop(&mut self) {
        let docker_manager = DockerManager::new();
        if let Err(e) = docker_manager.stop_docker() {
            eprintln!("Error stopping Docker: {}", e);
        }
        if let Err(e) = cleanup_deployment() {
            eprintln!("Error during cleanup: {}", e);
        }
    }
}

// Add Docker installation handling to DockerManager
impl DockerManager {
    fn handle_docker_setup(&self) -> io::Result<()> {
        match Command::new("docker").arg("--version").output() {
            Ok(_) => {
                println!("✅ Docker is installed");
                Ok(())
            }
            Err(_) => {
                println!("❌ Docker not found. Installing Docker...");
                self.install_docker()?;
                println!("⏳ Starting Docker for first time...");
                self.start_docker()?;
                Ok(())
            }
        }
    }
}
// Add cleanup on program exit

// Helper function for enterprise docker-compose
fn generate_enterprise_docker_compose(app_name: &str, port: &str) -> String {
    format!(
        r#"version: '3.8'
services:
  {app_name}:
    build: .
    ports:
      - "{port}:{port}"
    environment:
      - PORT={port}
      - NODE_ENV=production
      - BUN_ENV=production
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 4G
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:{port}/health"]
      interval: 30s
      timeout: 10s
      retries: 3
    networks:
      - app_net

  redis:
    image: redis:alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
    command: redis-server --appendonly yes
    networks:
      - app_net

  prometheus:
    image: prom/prometheus
    ports:
      - "9090:9090"
    volumes:
      - prometheus_data:/prometheus
    networks:
      - app_net

  grafana:
    image: grafana/grafana
    ports:
      - "3000:3000"
    volumes:
      - grafana_data:/var/lib/grafana
    networks:
      - app_net

networks:
  app_net:
    driver: bridge

volumes:
  redis_data:
  prometheus_data:
  grafana_data:"#
    )
}

fn create_nextjs_files(port: &str) -> io::Result<()> {
    let _ = port;
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
    let _ = entry_point;
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
node_modules/
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
# Multi-stage build for optimization
FROM node:18-alpine AS builder

# Install system dependencies
RUN apk add --no-cache \
    bash \
    curl \
    nginx \
    supervisor \
    redis \
    postgresql-client \
    git \
    python3 \
    make \
    g++

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
ENV NEXT_TELEMETRY_DISABLED=1

WORKDIR /app

# Copy the entire project
COPY . .

# Preserve user's package.json but add optimizations if needed
RUN if [ -f "package.json" ]; then \
    # Backup original package.json
    cp package.json package.json.original && \
    # Add optimization scripts if they don't exist
    jq '. * {"scripts": {. .scripts + {"analyze": "ANALYZE=true next build"}}}' package.json.original > package.json; \
    fi

# Install dependencies based on existing package-lock.json or yarn.lock
RUN if [ -f "yarn.lock" ]; then \
        yarn install --frozen-lockfile --production; \
    elif [ -f "package-lock.json" ]; then \
        npm ci --production; \
    else \
        bun install --production; \
    fi

# Build the application
RUN bun run build

# Production image
FROM node:18-alpine AS runner
WORKDIR /app

# Copy necessary files from builder
COPY --from=builder /app/.next ./.next
COPY --from=builder /app/public ./public
COPY --from=builder /app/package.json ./package.json
COPY --from=builder /app/next.config.js ./next.config.js
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /root/.bun /root/.bun

# Copy all other project files except those in .dockerignore
COPY --from=builder /app/. .

# Install production dependencies only
ENV NODE_ENV=production
ENV PATH="/root/.bun/bin:${PATH}"

# Runtime optimizations
ENV BUN_JS_ALLOCATIONS=1000000
ENV BUN_RUNTIME_CALLS=100000
ENV NEXT_TELEMETRY_DISABLED=1

# Set up health check
HEALTHCHECK --interval=30s --timeout=30s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/api/health || exit 1

# Expose ports
EXPOSE 3000

# Start the application with clustering
CMD ["bun", "run", "start"]"#;

    // Add framework-specific optimizations
    let framework_optimizations = match app_type {
        "nextjs" => r#"

# Next.js specific optimizations
RUN bun add \
    compression \
    helmet \
    redis \
    connect-redis \
    @sentry/nextjs \
    sharp \
    next-pwa

# Enable source maps for production debugging
ENV NEXT_SHARP_PATH=/usr/local/lib/node_modules/sharp
ENV NEXT_OPTIMIZE_IMAGES=true
ENV NEXT_OPTIMIZE_CSS=true"#,
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

fn generate_supervisor_config() -> String {
    r#"[supervisord]
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
stderr_logfile_maxbytes=0"#.to_string()
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
    let rt = Runtime::new()?;
    rt.block_on(async {
        setup_monitoring(app_name, namespace, mode).await?;
        let _ = setup_network_policies(app_name, namespace);
        Ok::<(), io::Error>(())
    })?;

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
                    println!("{}", GradientText::warning("⏳ Docker Desktop is not running.🥲 Attempting to start..."));
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
    let _ = mode;
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
    println!("{}", GradientText::status(&format!("    Deployment: {}", metadata.kubernetes_metadata.deployment_name)));
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

async fn setup_monitoring(app_name: &str, namespace: &str, mode: &str) -> io::Result<()> {
    let _ = app_name;
    let _ = namespace;
    let _ = mode;
    let prometheus_config = r#"
    global:
      scrape_interval: 15s
      evaluation_interval: 15s
    
    scrape_configs:
      - job_name: 'bun-metrics'
        static_configs:
          - targets: ['localhost:3000']
        metrics_path: '/metrics'
    "#;

    let grafana_dashboard = r#"
    {
      "dashboard": {
        "id": null,
        "title": "Bun.js Performance",
        "panels": [
          {
            "title": "Request Rate",
            "type": "graph",
            "datasource": "Prometheus"
          }
        ]
      }
    }"#;

    fs::write("prometheus.yml", prometheus_config)?;
    fs::write("grafana-dashboard.json", grafana_dashboard)?;
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

fn generate_haproxy_config(_mode: &str) -> io::Result<()> {
    let config = format!(r#"
    backend apps
        balance first
        hash-type consistent
        stick-table type string len 32 size 100k expire 30m
        stick store-request req.cook(sessionid)
        
        # Advanced health checks
        option httpchk HEAD /health HTTP/1.1\r\nHost:\ localhost
        http-check expect status 200
        
        # Circuit breaker
        default-server inter 3s fall 3 rise 2 on-marked-down shutdown-sessions
        
        # Dynamic server discovery
        server-template app- 20 127.0.0.1:3000-3020 check resolvers docker init-addr none
    "#);

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
async fn setup_caching_layer() -> io::Result<()> {
    // Multi-layer caching
    setup_redis_cluster().await?;
    setup_varnish_cache().await?;
    // Add Traefik integration for edge caching and CDN
    let traefik_config = r#"
    kind: ConfigMap
    metadata:
      name: traefik-config
    data:
      TRAEFIK_PROVIDERS_FILE_FILENAME: "/config/dynamic.yaml"
      TRAEFIK_API_INSECURE: "false"
      TRAEFIK_API_DASHBOARD: "true"
      TRAEFIK_ENTRYPOINTS_WEB_ADDRESS: ":80"
      TRAEFIK_ENTRYPOINTS_WEBSECURE_ADDRESS: ":443"
      TRAEFIK_CERTIFICATESRESOLVERS_DEFAULT_ACME_EMAIL: "admin@example.com"
      TRAEFIK_CERTIFICATESRESOLVERS_DEFAULT_ACME_STORAGE: "/certs/acme.json"
      TRAEFIK_CERTIFICATESRESOLVERS_DEFAULT_ACME_HTTPCHALLENGE_ENTRYPOINT: "web"
    "#;

    // Configure caching middleware
    let caching_config = r#"
    kind: Middleware
    metadata:
      name: cache-middleware
    spec:
      headers:
        browserXssFilter: true
        customResponseHeaders:
          Cache-Control: "public, max-age=3600"
          X-Cache-Status: "HIT"
    "#;
    let edge_rules = r#"
    cache:
      rules:
        - pattern: "/*"
          edge_ttl: 2h
          browser_ttl: 30m
    "#;
    
    fs::write("traefik-config.yaml", traefik_config)?;
    fs::write("caching-config.yaml", caching_config)?;
    fs::write("edge-rules.yaml", edge_rules)?;

    // Apply configurations
    Command::new("kubectl")
        .args(["apply", "-f", "traefik-config.yaml"])
        .output()?;
    Command::new("kubectl")
        .args(["apply", "-f", "caching-config.yaml"])
        .output()?;
    Command::new("kubectl")
        .args(["apply", "-f", "edge-rules.yaml"])
        .output()?;

    Ok(())
}

// Enhanced load balancing configuration
async fn setup_load_balancing(mode: &str) -> io::Result<()> {
    let _ = mode;
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

fn initialize_project(project_type: &str) -> io::Result<()> {
    println!("{}", GradientText::cyber("🚀 Initializing project..."));
    
    // Create necessary directories
    fs::create_dir_all("src")?;
    
    // Create app files based on project type
    create_app_files(project_type, "3000", "src/index.js")?;
    
    // Initialize git if not already initialized
    if !Path::new(".git").exists() {
        Command::new("git")
            .args(["init"])
            .output()?;
            
        // Create default .gitignore
        let gitignore = r#"node_modules/
dist/
.env
.DS_Store"#;
        fs::write(".gitignore", gitignore)?;
    }
    
    println!("{}", GradientText::success("✅ Project initialized successfully!"));
    Ok(())
}

async fn setup_security_layer(app_name: &str, namespace: &str) -> io::Result<()> {
    println!("🔒 Setting up enterprise security layer...");
    
    // Setup mTLS certificates
    let cert_config = r#"
[req]
distinguished_name = req_distinguished_name
x509_extensions = v3_req
prompt = no

[req_distinguished_name]
C = US
ST = CA
L = San Francisco
O = Enterprise
OU = Security
CN = service.internal

[v3_req]
basicConstraints = CA:FALSE
keyUsage = nonRepudiation, digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth, clientAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = *.service.internal
DNS.2 = localhost"#;

    fs::write("cert.conf", cert_config)?;

    // Generate certificates
    Command::new("openssl")
        .args(["req", "-x509", "-nodes", "-days", "365", "-newkey", "rsa:2048",
               "-keyout", "tls.key", "-out", "tls.crt", "-config", "cert.conf"])
        .output()?;

    // Apply Zero Trust policies
    let zero_trust_policy = format!(r#"
apiVersion: security.istio.io/v1beta1
kind: AuthorizationPolicy
metadata:
  name: {app_name}-zero-trust
  namespace: {namespace}
spec:
  action: ALLOW
  rules:
  - from:
    - source:
        principals: ["cluster.local/ns/{namespace}/sa/{app_name}"]
    to:
    - operation:
        methods: ["GET", "POST"]"#);

    fs::write("zero-trust-policy.yaml", zero_trust_policy)?;
    
    Ok(())
}

async fn setup_redis_cluster() -> io::Result<()> {
    println!("📦 Setting up Redis cluster...");
    
    let redis_config = r#"
port 6379
cluster-enabled yes
cluster-config-file nodes.conf
cluster-node-timeout 5000
appendonly yes
maxmemory 2gb
maxmemory-policy allkeys-lru"#;

    fs::write("redis.conf", redis_config)?;
    
    Ok(())
}

async fn setup_varnish_cache() -> io::Result<()> {
    println!("🚀 Setting up Varnish cache...");
    
    let vcl_config = r#"
vcl 4.0;

backend default {
    .host = "127.0.0.1";
    .port = "8080";
    .probe = {
        .url = "/health";
        .timeout = 2s;
        .interval = 5s;
        .window = 5;
        .threshold = 3;
    }
}

sub vcl_recv {
    if (req.method == "PURGE") {
        return(purge);
    }
}"#;

    fs::write("default.vcl", vcl_config)?;
    
    Ok(())
}

fn deploy_nginx(namespace: &str) -> io::Result<()> {
    println!("{}", GradientText::cyber("📦 Deploying Nginx..."));

    // Create Nginx ConfigMap with optimized configuration
    let nginx_config = format!(r#"
apiVersion: v1
kind: ConfigMap
metadata:
  name: nginx-config
  namespace: {namespace}
data:
  nginx.conf: |
    worker_processes auto;
    worker_rlimit_nofile 100000;
    
    events {{
        worker_connections 4096;
        use epoll;
        multi_accept on;
    }}
    
    http {{
        # Optimization
        sendfile on;
        tcp_nopush on;
        tcp_nodelay on;
        keepalive_timeout 65;
        keepalive_requests 100000;
        
        # Bun.js Optimizations
        upstream bun_servers {{
            least_conn;
            server localhost:3000 max_fails=3 fail_timeout=30s;
            server localhost:3001 max_fails=3 fail_timeout=30s;
            keepalive 32;
        }}
        
        # Security
        ssl_protocols TLSv1.2 TLSv1.3;
        ssl_prefer_server_ciphers on;
        ssl_session_cache shared:SSL:50m;
        ssl_session_timeout 1d;
        
        # Compression
        gzip on;
        gzip_comp_level 6;
        gzip_types text/plain text/css application/json application/javascript;
        
        server {{
            listen 80;
            listen [::]:80;
            listen 443 ssl http2;
            
            # SSL Configuration
            ssl_certificate /etc/nginx/ssl/tls.crt;
            ssl_certificate_key /etc/nginx/ssl/tls.key;
            
            location / {{
                proxy_pass http://bun_servers;
                proxy_http_version 1.1;
                proxy_set_header Upgrade $http_upgrade;
                proxy_set_header Connection 'upgrade';
                proxy_set_header Host $host;
                proxy_cache_bypass $http_upgrade;
                
                # Security headers
                add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
                add_header X-Frame-Options "SAMEORIGIN" always;
                add_header X-Content-Type-Options "nosniff" always;
            }}
            
            location /health {{
                access_log off;
                return 200 'healthy\n';
            }}
        }}
    }}
"#);

    fs::write("nginx-config.yaml", nginx_config)?;

    // Apply ConfigMap
    Command::new("kubectl")
        .args(["apply", "-f", "nginx-config.yaml"])
        .output()?;

    // Deploy Nginx with optimized settings
    let nginx_deployment = format!(r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: nginx
  namespace: {namespace}
spec:
  replicas: 2
  selector:
    matchLabels:
      app: nginx
  template:
    metadata:
      labels:
        app: nginx
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9113"
    spec:
      containers:
      - name: nginx
        image: nginx:mainline
        ports:
        - containerPort: 80
        - containerPort: 443
        resources:
          requests:
            cpu: "500m"
            memory: "512Mi"
          limits:
            cpu: "2"
            memory: "2Gi"
        volumeMounts:
        - name: nginx-config
          mountPath: /etc/nginx/nginx.conf
          subPath: nginx.conf
        - name: ssl-certs
          mountPath: /etc/nginx/ssl
        livenessProbe:
          httpGet:
            path: /health
            port: 80
          initialDelaySeconds: 5
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 80
          initialDelaySeconds: 2
          periodSeconds: 5
      volumes:
      - name: nginx-config
        configMap:
          name: nginx-config
      - name: ssl-certs
        secret:
          secretName: nginx-ssl-certs
"#);

    fs::write("nginx-deployment.yaml", nginx_deployment)?;

    Command::new("kubectl")
        .args(["apply", "-f", "nginx-deployment.yaml"])
        .output()?;

    // Create Nginx Service
    let nginx_service = format!(r#"
apiVersion: v1
kind: Service
metadata:
  name: nginx
  namespace: {namespace}
  annotations:
    prometheus.io/scrape: "true"
    prometheus.io/port: "9113"
spec:
  type: LoadBalancer
  ports:
  - name: http
    port: 80
    targetPort: 80
  - name: https
    port: 443
    targetPort: 443
  selector:
    app: nginx
"#);

    fs::write("nginx-service.yaml", nginx_service)?;

    Command::new("kubectl")
        .args(["apply", "-f", "nginx-service.yaml"])
        .output()?;

    println!("{}", GradientText::success("✅ Nginx deployed successfully"));
    Ok(())
}

fn optimize_kernel_parameters() -> io::Result<()> {
    let sysctl_config = r#"
    # Network optimizations
    net.core.somaxconn = 65535
    net.ipv4.tcp_max_tw_buckets = 1440000
    net.ipv4.ip_local_port_range = 1024 65535
    net.ipv4.tcp_fin_timeout = 15
    net.ipv4.tcp_keepalive_time = 300
    net.ipv4.tcp_max_syn_backlog = 262144
    net.core.netdev_max_backlog = 262144
    
    # Memory optimizations
    vm.swappiness = 10
    vm.dirty_ratio = 60
    vm.dirty_background_ratio = 2
    "#;
    
    fs::write("/etc/sysctl.d/99-performance.conf", sysctl_config)?;
    Command::new("sysctl").args(["-p"]).output()?;
    Ok(())
}

fn optimize_bun_runtime() -> io::Result<()> {
    let config = r#"
    {
      "runtime": {
        "watch": false,
        "minify": true,
        "jsx": "react",
        "jsxImportSource": "react",
        "define": {
          "process.env.NODE_ENV": "production"
        },
        "performance": {
          "maxWorkers": "auto",
          "workerThreads": true,
          "asyncCompression": true,
          "gcInterval": 120000,
          "maxOldSpaceSize": 4096
        }
      }
    }"#;
    
    fs::write("bunfig.toml", config)?;
    Ok(())
}

fn enhance_load_balancer_config() -> io::Result<()> {
    let config = r#"
    backend dynamic_servers {
        dynamic
        check
        balance roundrobin
        hash-type consistent
        server-template bun 10 127.0.0.1:3000-3010 check
        stick-table type ip size 1m expire 30m
        stick store-request req.cook(sessionid)
        
        # Circuit breaker
        default-server inter 1s fastinter 100ms downinter 10s fall 3 rise 2
        
        # Health checks
        option httpchk HEAD /health HTTP/1.1
        http-check expect status 200
    }
    "#;
    fs::write("haproxy-dynamic.cfg", config)?;
    Ok(())
}

fn create_nextjs_optimized_config() -> io::Result<()> {
    // Enhanced Next.js + TypeScript configuration
    let next_config = r#"
    module.exports = {
      reactStrictMode: true,
      experimental: {
        serverActions: true,
        serverComponents: true,
        concurrentFeatures: true,
        optimizeCss: true,
        optimizeImages: true,
        scrollRestoration: true,
        runtime: 'experimental-edge',
      },
      compiler: {
        removeConsole: process.env.NODE_ENV === 'production',
      },
      typescript: {
        ignoreBuildErrors: false,
        tsconfigPath: './tsconfig.json'
      },
      // Bun.js optimizations
      webpack: (config) => {
        config.experiments = { topLevelAwait: true };
        config.cache = {
          type: 'filesystem',
          buildDependencies: {
            config: [__filename],
          },
        };
        return config;
      },
    }"#;

    let tsconfig = r#"{
      "compilerOptions": {
        "target": "esnext",
        "lib": ["dom", "dom.iterable", "esnext"],
        "allowJs": true,
        "skipLibCheck": true,
        "strict": true,
        "forceConsistentCasingInFileNames": true,
        "noEmit": true,
        "incremental": true,
        "esModuleInterop": true,
        "module": "esnext",
        "moduleResolution": "node",
        "resolveJsonModule": true,
        "isolatedModules": true,
        "jsx": "preserve"
      },
      "include": ["next-env.d.ts", "**/*.ts", "**/*.tsx"],
      "exclude": ["node_modules"]
    }"#;

    fs::write("next.config.js", next_config)?;
    fs::write("tsconfig.json", tsconfig)?;
    Ok(())
}

fn validate_nextjs_project() -> io::Result<bool> {
    // Check for essential Next.js files and directories
    let required_files = vec![
        "package.json",
        "next.config.js",
        "tsconfig.json",
    ];

    let required_dirs = vec![
        "src",
        "public",
        "app",
        "components",
        "pages",
    ];

    // Optional but common directories
    let optional_dirs = vec![
        "api",
        "lib",
        "utils",
        "hooks",
        "services",
        "redux",
        "store",
        "styles",
        "types",
    ];

    // Validate package.json for Next.js dependencies
    if Path::new("package.json").exists() {
        let package_json = fs::read_to_string("package.json")?;
        let pkg: serde_json::Value = serde_json::from_str(&package_json)?;
        
        if let Some(deps) = pkg.get("dependencies") {
            if !deps.get("next").is_some() {
                println!("⚠️ Warning: Next.js dependency not found in package.json");
                return Ok(false);
            }
        }
    }

    // Check required files and directories
    let has_required = required_files.iter().all(|f| Path::new(f).exists()) &&
                      required_dirs.iter().any(|d| Path::new(d).exists());

    // Count optional directories for optimization level
    let optional_count = optional_dirs.iter()
        .filter(|d| Path::new(d).exists())
        .count();

    Ok(has_required)
}

fn optimize_existing_nextjs_project() -> io::Result<()> {
    println!("🔍 Analyzing existing Next.js project...");

    // Backup existing configuration
    if Path::new("next.config.js").exists() {
        fs::copy("next.config.js", "next.config.js.backup")?;
    }

    // Read existing next.config.js
    let existing_config = if Path::new("next.config.js").exists() {
        fs::read_to_string("next.config.js")?
    } else {
        String::new()
    };

    // Merge with our optimized config
    let optimized_config = r#"
    const nextConfig = {
      reactStrictMode: true,
      experimental: {
        serverActions: true,
        serverComponents: true,
        concurrentFeatures: true,
        optimizeCss: true,
        optimizeImages: true,
        scrollRestoration: true,
        runtime: 'experimental-edge',
        turbo: {
          loaders: {
            '.js': ['bun-loader'],
            '.ts': ['bun-loader'],
            '.tsx': ['bun-loader'],
          },
        },
      },
      compiler: {
        removeConsole: process.env.NODE_ENV === 'production',
        styledComponents: true,
      },
      typescript: {
        ignoreBuildErrors: false,
        tsconfigPath: './tsconfig.json'
      },
      webpack: (config, { dev, isServer }) => {
        // Keep existing webpack config
        if (typeof existingWebpackConfig === 'function') {
          config = existingWebpackConfig(config, { dev, isServer });
        }

        // Add our optimizations
        config.experiments = { 
          topLevelAwait: true,
          layers: true,
        };
        
        config.cache = {
          type: 'filesystem',
          buildDependencies: {
            config: [__filename],
          },
          compression: 'brotli',
          profile: true,
        };

        // Optimize production builds
        if (!dev) {
          config.optimization = {
            ...config.optimization,
            minimize: true,
            moduleIds: 'deterministic',
            runtimeChunk: 'single',
            splitChunks: {
              chunks: 'all',
              minSize: 20000,
              minChunks: 1,
              maxAsyncRequests: 30,
              maxInitialRequests: 30,
              cacheGroups: {
                default: false,
                vendors: false,
                framework: {
                  chunks: 'all',
                  name: 'framework',
                  test: /(?<!node_modules.*)[\\/]node_modules[\\/](react|react-dom|scheduler|prop-types|use-subscription)[\\/]/,
                  priority: 40,
                  enforce: true,
                },
                lib: {
                  test: /[\\/]node_modules[\\/]/,
                  name(module) {
                    return `lib.${module.context.match(/[\\/]node_modules[\\/](.*?)([\\/]|$)/)[1].replace('@', '')}`;
                  },
                  priority: 30,
                  minChunks: 1,
                  reuseExistingChunk: true,
                },
              },
            },
          };
        }

        return config;
      },
      // Advanced caching strategy
      onDemandEntries: {
        maxInactiveAge: 60 * 60 * 1000,
        pagesBufferLength: 5,
      },
    }

    module.exports = nextConfig;
    "#;

    fs::write("next.config.js", optimized_config)?;
    use serde_json::json; // Import the json macro

    // Update package.json with optimized scripts and dependencies
    if Path::new("package.json").exists() {
        let mut package_json: serde_json::Value = serde_json::from_str(&fs::read_to_string("package.json")?)?;
        
        // Add optimized scripts
        if let Some(scripts) = package_json.get_mut("scripts").and_then(|s| s.as_object_mut()) {
            scripts.insert("dev".to_string(), json!("next dev -p 3000"));
            scripts.insert("build".to_string(), json!("next build"));
            scripts.insert("start".to_string(), json!("next start -p 3000"));
            scripts.insert("analyze".to_string(), json!("ANALYZE=true next build"));
            scripts.insert("lint".to_string(), json!("next lint && prettier --write ."));
        }

        fs::write("package.json", serde_json::to_string_pretty(&package_json)?)?;
    }

    println!("✅ Next.js project optimized successfully!");
    Ok(())
}

fn create_enhanced_dockerignore() -> io::Result<()> {
    let dockerignore = r#"
# Version control
.git
.gitignore
.gitattributes

# Dependencies
node_modules
.pnp
.pnp.js

# Testing
coverage
.nyc_output
cypress/videos
cypress/screenshots

# Next.js build output
.next
out

# Debug
npm-debug.log*
yarn-debug.log*
yarn-error.log*

# Local env files
.env*.local
.env.development
.env.test

# IDE
.idea
.vscode
*.swp
*.swo

# OS
.DS_Store
Thumbs.db

# Build files
*.log
*.pid
*.seed

# Cache
.eslintcache
.cache
.parcel-cache

# Docker
Dockerfile
.dockerignore
docker-compose*.yml

# Temporary files
*.tmp
*.temp
.temp
.tmp

# Keep these files
!package.json
!package-lock.json
!yarn.lock
!next.config.js
!tsconfig.json
!public/
!src/
!app/
!pages/
!components/
!styles/
!lib/
!utils/
!hooks/
!services/
!api/
!types/
"#;

    fs::write(".dockerignore", dockerignore.trim())?;
    Ok(())
}
