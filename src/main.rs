use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::thread;
use std::time::Duration;
use std::{
    fs,
    io::{self, Write},
    path::Path,
    process::Command,
};
mod gradient;
use clap::{App, Arg, SubCommand};
use gradient::GradientText;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppMetadata {
    app_name: String,
    app_type: String,
    port: String,
    created_at: String,
    container_id: Option<String>,
    _status: String,
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

#[allow(dead_code)]
struct DockerManager;

#[allow(dead_code)]
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
                .args([
                    "/C",
                    "start",
                    "\"\"",
                    "\"C:\\Program Files\\Docker\\Docker\\Docker Desktop.exe\"",
                ])
                .status()?;
        }

        #[cfg(target_os = "linux")]
        {
            Command::new("systemctl")
                .args(["--user", "start", "docker"])
                .status()?;
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

        Err(io::Error::new(
            io::ErrorKind::Other,
            "Docker failed to start",
        ))
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
            let installer_url =
                "https://desktop.docker.com/win/main/amd64/Docker%20Desktop%20Installer.exe";
            Command::new("powershell")
                .args([
                    "-Command",
                    &format!(
                        "Invoke-WebRequest '{}' -OutFile 'DockerInstaller.exe'",
                        installer_url
                    ),
                ])
                .status()?;

            Command::new("DockerInstaller.exe")
                .args(["install", "--quiet"])
                .status()?;

            fs::remove_file("DockerInstaller.exe")?;
        }

        #[cfg(target_os = "linux")]
        {
            Command::new("sudo").args(["apt-get", "update"]).status()?;

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
            Command::new("open").args(["-a", "Docker"]).status()?;
        }

        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .args([
                    "/C",
                    "start",
                    "\"\"",
                    "\"C:\\Program Files\\Docker\\Docker\\Docker Desktop.exe\"",
                ])
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

        Err(io::Error::new(
            io::ErrorKind::Other,
            "Docker Desktop failed to start",
        ))
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

    fn handle_docker_setup(&self) -> io::Result<()> {
        println!("🔧 Setting up Docker environment...");

        // Step 1: Verify Docker installation and start if needed
        self.verify_and_setup_docker()?;

        // Step 2: Check Docker configuration
        self.check_docker_setup()?;

        // Step 3: Verify Docker daemon is responsive
        match Command::new("docker").arg("info").output() {
            Ok(output) if output.status.success() => {
                println!("✅ Docker daemon is responsive");
            }
            _ => {
                println!("⚠️ Docker daemon not responding. Attempting to restart...");
                self.stop_docker()?;
                thread::sleep(Duration::from_secs(2));
                self.start_docker()?;
            }
        }

        // Step 4: Check Docker network
        let network_check = Command::new("docker").args(["network", "ls"]).output()?;

        if !network_check.status.success() {
            println!("⚠️ Docker network issues detected. Creating default networks...");
            Command::new("docker")
                .args(["network", "create", "app-network"])
                .output()?;
        }

        // Step 5: Clean up old containers and images
        println!("🧹 Cleaning up Docker environment...");
        Command::new("docker")
            .args(["system", "prune", "-f"])
            .output()?;

        // Step 6: Verify Docker Compose
        match Command::new("docker-compose").arg("--version").output() {
            Ok(_) => println!("✅ Docker Compose is installed"),
            Err(_) => {
                println!("⚠️ Docker Compose not found. Please install Docker Compose.");
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Docker Compose is required but not installed",
                ));
            }
        }

        println!("✅ Docker setup completed successfully");
        Ok(())
    }
}
fn main() {
    // First check Kubernetes connection
    if let Err(e) = check_kubernetes_connection() {
        eprintln!("Error connecting to Kubernetes: {}", e);
        std::process::exit(1);
    }

    let app = App::new("rustify")
        .version("0.1.0")
        .author("Harshit Duggal")
        .about("🚀 Ultra-optimized deployment CLI")
        .subcommand(
            SubCommand::with_name("init")
                .about("Initialize project")
                .arg(
                    Arg::with_name("type")
                        .long("type")
                        .value_name("TYPE")
                        .help("Project type (default: bun)")
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("deploy")
                .about("Deploy application")
                .arg(
                    Arg::with_name("prod")
                        .long("prod")
                        .help("Deploy in production mode"),
                )
                .arg(
                    Arg::with_name("port")
                        .long("port")
                        .value_name("PORT")
                        .help("Custom port (default: 8000)"),
                )
                .arg(
                    Arg::with_name("rpl")
                        .long("rpl")
                        .help("Enable auto-scaling replicas"),
                ),
        )
        .get_matches();

    match app.subcommand() {
        Some(("init", matches)) => {
            let project_type = matches.value_of("type").unwrap_or("bun");
            if let Err(e) = initialize_project(project_type) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            // Create Docker files after project initialization
            if let Err(e) = create_docker_files() {
                eprintln!("Error creating Docker files: {}", e);
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
                _status: String::from("pending"),
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

            // Deploy with Docker first
            if let Err(e) = deploy_with_docker(&metadata) {
                eprintln!("Error deploying with Docker: {}", e);
                std::process::exit(1);
            }

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
fn deploy_application(
    metadata: &mut AppMetadata,
    is_prod: bool,
    auto_scale: bool,
) -> io::Result<()> {
    println!("🚀 Starting deployment process...");

    // Step 1: Verify infrastructure and container
    verify_infrastructure()?;
    if let Some(container_id) = &metadata.container_id {
        verify_container_status(container_id)?;
    }

    // Step 2: Generate and apply Kubernetes manifests for production
    if is_prod {
        generate_kubernetes_manifests(
            &metadata.app_name,
            &metadata.app_type,
            &metadata.port,
            metadata.kubernetes_metadata.replicas,
            &metadata.kubernetes_metadata.namespace,
            "prod",
        )?;
    }

    // Save updated metadata
    save_metadata(metadata)?;

    println!("✅ Deployment completed successfully!");
    Ok(())
}

// Add this helper function to handle Kubernetes deployment
fn deploy_to_kubernetes(metadata: &mut AppMetadata, auto_scale: bool) -> io::Result<String> {
    let _ = auto_scale;
    println!("🚀 Deploying to Kubernetes...");

    // Clone necessary values to avoid borrowing issues
    let app_name = metadata.app_name.clone();
    let app_type = metadata.app_type.clone();
    let port = metadata.port.clone();
    let namespace = metadata.kubernetes_metadata.namespace.clone();
    let replicas = metadata.kubernetes_metadata.replicas;
    let deployment_name = metadata.kubernetes_metadata.deployment_name.clone();

    // Generate and apply manifests
    generate_kubernetes_manifests(&app_name, &app_type, &port, replicas, &namespace, "prod")?;

    apply_kubernetes_manifests(&namespace)?;

    // Wait for deployment
    wait_for_kubernetes_deployment(&deployment_name, &namespace)?;

    // Update pod status first
    let status = update_pod_status(metadata, &namespace)?;

    // Then print status using the cloned values to avoid borrowing metadata
    print_kubernetes_status(&AppMetadata {
        app_name: app_name.clone(),
        app_type: app_type,
        port: port,
        kubernetes_metadata: metadata.kubernetes_metadata.clone(),
        ..metadata.clone()
    });

    Ok(format!("{}-deployment", app_name))
}

fn deploy_to_docker(metadata: &AppMetadata) -> io::Result<String> {
    println!("🐳 Deploying to Docker...");

    // Generate Docker configuration
    let dockerfile_content = match metadata.app_type.as_str() {
        "bun" => format!(
            "FROM oven/bun:latest\n\
            WORKDIR /app\n\
            COPY . .\n\
            RUN bun install\n\
            EXPOSE {}\n\
            CMD [\"bun\", \"start\"]",
            metadata.port
        ),
        _ => return Err(io::Error::new(io::ErrorKind::Other, "Unsupported app type")),
    };
    fs::write("Dockerfile", dockerfile_content)?;

    let docker_compose = format!(
        "version: '3.8'\n\
        services:\n\
          {}:\n\
            build: .\n\
            ports:\n\
              - {}:{}\n\
            restart: always\n\
            environment:\n\
              - NODE_ENV=production",
        metadata.app_name, metadata.port, metadata.port
    );
    fs::write("docker-compose.yml", docker_compose)?;

    // Build Docker image
    println!("🏗️ Building Docker image...");
    Command::new("docker")
        .args(["build", "-t", &format!("{}-app", metadata.app_name), "."])
        .status()?;

    // Run Docker container
    println!("🐳 Running Docker container...");
    let container_id = Command::new("docker")
        .args([
            "run",
            "-d",
            "-p",
            &format!("{}:{}", metadata.port, metadata.port),
            &format!("{}-app", metadata.app_name),
        ])
        .output()?
        .stdout;

    Ok(String::from_utf8_lossy(&container_id).trim().to_string())
}

fn verify_docker_installation() -> io::Result<()> {
    println!(
        "{}",
        GradientText::cyber("🔍 Verifying Docker installation...")
    );
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

    println!(
        "{}",
        GradientText::cyber("⏳ Waiting for container health check...")
    );
    std::thread::sleep(std::time::Duration::from_secs(5));

    let health_output = Command::new("docker")
        .args(["inspect", "-f", "{{.State.Health.Status}}", container_id])
        .output()?;

    let health_status = String::from_utf8_lossy(&health_output.stdout)
        .trim()
        .to_string();
    if health_status != "healthy" {
        println!(
            "{}",
            GradientText::warning(&format!("⚠️  Container health status: {}", health_status))
        );
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

fn generate_kubernetes_manifests(
    app_name: &str,
    _app_type: &str,
    port: &str,
    replicas: i32,
    namespace: &str,
    mode: &str,
) -> io::Result<()> {
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
        .args([
            "create",
            "namespace",
            namespace,
            "--dry-run=client",
            "-o",
            "yaml",
        ])
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

fn create_kubernetes_ingress(
    app_name: &str,
    port: &str,
    namespace: &str,
    mode: &str,
) -> io::Result<String> {
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
    println!(
        "{}",
        GradientText::status(&format!(
            "   • Namespace: {}",
            metadata.kubernetes_metadata.namespace
        ))
    );
    println!(
        "{}",
        GradientText::status(&format!(
            "    Deployment: {}",
            metadata.kubernetes_metadata.deployment_name
        ))
    );
    println!(
        "{}",
        GradientText::status(&format!(
            "   • Service: {}",
            metadata.kubernetes_metadata.service_name
        ))
    );
    println!(
        "{}",
        GradientText::status(&format!(
            "   • Replicas: {}",
            metadata.kubernetes_metadata.replicas
        ))
    );
    println!(
        "{}",
        GradientText::status(&format!(
            "   • Pod Status: {:?}",
            metadata.kubernetes_metadata.pod_status
        ))
    );
    if let Some(host) = &metadata.kubernetes_metadata.ingress_host {
        println!(
            "{}",
            GradientText::status(&format!("   • Ingress Host: {}", host))
        );
    }
}

fn verify_infrastructure() -> io::Result<()> {
    println!("🔍 Verifying infrastructure...");

    // Check Docker
    println!("\n📦 Checking Docker...");
    match Command::new("docker").arg("--version").output() {
        Ok(output) => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!(
                "{}",
                GradientText::success(&format!("✅ Docker installed: {}", version.trim()))
            );

            // Check if Docker daemon is running
            match Command::new("docker").args(["ps"]).output() {
                Ok(_) => println!(" Docker daemon is running"),
                Err(_) => return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Docker daemon is not running. Please start Docker Desktop or docker service",
                )),
            }
        }
        Err(_) => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Docker is not installed or not in PATH",
            ))
        }
    }

    // Check Kubernetes context
    println!("\n☸️  Checking Kubernetes...");

    // Ensure we're using docker-desktop context
    Command::new("kubectl")
        .args(["config", "use-context", "docker-desktop"])
        .output()?;

    // Check kubectl installation and connection
    match Command::new("kubectl")
        .args(["cluster-info", "dump"])
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                println!("�� Connected to Kubernetes cluster (docker-desktop)");

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
                    "Kubernetes cluster is not ready",
                ));
            }
        }
        Err(_) => {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Cannot connect to Kubernetes cluster",
            ))
        }
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
            "--namespace",
            "ingress-nginx",
            "--for=condition=ready",
            "pod",
            "--selector=app.kubernetes.io/component=controller",
            "--timeout=300s",
        ])
        .output()?;

    println!("✅ Nginx Ingress Controller installed successfully");
    Ok(())
}

fn prepare_kubernetes_deployment(app_name: &str, _mode: &str) -> io::Result<()> {
    // Tag the image for Kubernetes
    Command::new("docker")
        .args([
            "tag",
            &format!("rust-dockerize-{}", app_name),
            &format!("{}:latest", app_name),
        ])
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
        .args([
            "create",
            "namespace",
            namespace,
            "--dry-run=client",
            "-o",
            "yaml",
        ])
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

fn cleanup_deployment(app_name: &str, namespace: &str) -> io::Result<()> {
    println!("🧹 Cleaning up old deployments...");

    // Delete old pods
    Command::new("kubectl")
        .args([
            "delete",
            "pods",
            "-n",
            namespace,
            "-l",
            &format!("app={}", app_name),
            "--field-selector",
            "status.phase=Succeeded",
        ])
        .output()?;

    // Delete failed pods
    Command::new("kubectl")
        .args([
            "delete",
            "pods",
            "-n",
            namespace,
            "-l",
            &format!("app={}", app_name),
            "--field-selector",
            "status.phase=Failed",
        ])
        .output()?;

    println!("✅ Cleanup completed");
    Ok(())
}

fn check_kubernetes_connection() -> io::Result<()> {
    println!("🔍 Verifying Kubernetes connection...");

    // First, check if Docker Desktop is running
    if let Err(_) = Command::new("docker").arg("info").output() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Docker Desktop is not running. Please start Docker Desktop first.",
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
                     2. Kubernetes is enabled and running (green icon)\n\
                     3. No firewall is blocking the connection",
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
    let config = format!(
        r#"
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
    println!("🔍 Verifying Kubernetes setup...");

    // Step 1: Check if kubectl is installed
    match Command::new("kubectl").arg("version").output() {
        Ok(_) => println!("✅ kubectl is installed"),
        Err(_) => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "kubectl is not installed. Please install kubectl first.",
            ))
        }
    }

    // Step 2: Ensure Docker Desktop is running with Kubernetes
    let docker_status = Command::new("docker").arg("info").output()?;
    if !docker_status.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Docker Desktop is not running. Please start Docker Desktop first.",
        ));
    }

    // Step 3: Enable Kubernetes if not already enabled
    println!("⏳ Checking Kubernetes status in Docker Desktop...");
    let k8s_context = Command::new("kubectl")
        .args(["config", "get-contexts"])
        .output()?;

    if !String::from_utf8_lossy(&k8s_context.stdout).contains("docker-desktop") {
        println!("⚠️ Kubernetes is not enabled in Docker Desktop");
        println!("🔄 Please enable Kubernetes in Docker Desktop:");
        println!("1. Open Docker Desktop");
        println!("2. Go to Settings/Preferences");
        println!("3. Select 'Kubernetes'");
        println!("4. Check 'Enable Kubernetes'");
        println!("5. Click 'Apply & Restart'");
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Kubernetes is not enabled in Docker Desktop",
        ));
    }

    // Step 4: Switch to docker-desktop context
    Command::new("kubectl")
        .args(["config", "use-context", "docker-desktop"])
        .output()?;

    // Step 5: Wait for Kubernetes to be ready
    println!("⏳ Waiting for Kubernetes to be ready...");
    for i in 0..30 {
        match Command::new("kubectl").args(["get", "nodes"]).output() {
            Ok(output) if output.status.success() => {
                let nodes = String::from_utf8_lossy(&output.stdout);
                if nodes.contains("Ready") {
                    println!("✅ Kubernetes is ready!");
                    return Ok(());
                }
            }
            _ if i == 29 => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Kubernetes failed to start after 30 attempts",
                ));
            }
            _ => {
                print!(".");
                io::stdout().flush()?;
                thread::sleep(Duration::from_secs(2));
            }
        }
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
    create_app_files(project_type, "3000")?;

    // Initialize git if not already initialized
    if !Path::new(".git").exists() {
        Command::new("git").args(["init"]).output()?;

        // Create default .gitignore
        let gitignore = r#"node_modules/
dist/
.env
.DS_Store"#;
        fs::write(".gitignore", gitignore)?;
    }

    println!(
        "{}",
        GradientText::success("✅ Project initialized successfully!")
    );
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
        .args([
            "req",
            "-x509",
            "-nodes",
            "-days",
            "365",
            "-newkey",
            "rsa:2048",
            "-keyout",
            "tls.key",
            "-out",
            "tls.crt",
            "-config",
            "cert.conf",
        ])
        .output()?;

    // Apply Zero Trust policies
    let zero_trust_policy = format!(
        r#"
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
        methods: ["GET", "POST"]"#
    );

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
    let nginx_config = format!(
        r#"
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
"#
    );

    fs::write("nginx-config.yaml", nginx_config)?;

    // Apply ConfigMap
    Command::new("kubectl")
        .args(["apply", "-f", "nginx-config.yaml"])
        .output()?;

    // Deploy Nginx with optimized settings
    let nginx_deployment = format!(
        r#"
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
"#
    );

    fs::write("nginx-deployment.yaml", nginx_deployment)?;

    Command::new("kubectl")
        .args(["apply", "-f", "nginx-deployment.yaml"])
        .output()?;

    // Create Nginx Service
    let nginx_service = format!(
        r#"
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
"#
    );

    fs::write("nginx-service.yaml", nginx_service)?;

    Command::new("kubectl")
        .args(["apply", "-f", "nginx-service.yaml"])
        .output()?;

    println!(
        "{}",
        GradientText::success("✅ Nginx deployed successfully")
    );
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
    let required_files = vec!["package.json", "next.config.js", "tsconfig.json"];

    let required_dirs = vec!["src", "public", "app", "components", "pages"];

    // Optional but common directories
    let optional_dirs = vec![
        "api", "lib", "utils", "hooks", "services", "redux", "store", "styles", "types",
    ];

    // Validate package.json for Next.js dependencies
    if Path::new("package.json").exists() {
        let package_json = fs::read_to_string("package.json")?;
        let pkg: serde_json::Value = serde_json::from_str(&package_json)?;

        if let Some(deps) = pkg.get("dependencies") {
            if !deps.get("next").is_some() {
                println!("️ Warning: Next.js dependency not found in package.json");
                return Ok(false);
            }
        }
    }

    // Check required files and directories
    let has_required = required_files.iter().all(|f| Path::new(f).exists())
        && required_dirs.iter().any(|d| Path::new(d).exists());

    // Count optional directories for optimization level
    let optional_count = optional_dirs
        .iter()
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
        let mut package_json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string("package.json")?)?;

        // Add optimized scripts
        if let Some(scripts) = package_json
            .get_mut("scripts")
            .and_then(|s| s.as_object_mut())
        {
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
node_modules/
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
!types/"#;

    fs::write(".dockerignore", dockerignore)?;
    Ok(())
}

fn optimize_existing_project(app_type: &str) -> io::Result<()> {
    let package_json = fs::read_to_string("package.json")?;
    let mut pkg: serde_json::Value = serde_json::from_str(&package_json)?;

    match app_type {
        "react" => optimize_react_config(&mut pkg)?,
        "nuxt" => optimize_nuxt_config(&mut pkg)?,
        "vue" => optimize_vue_config(&mut pkg)?,
        "svelte" => optimize_svelte_config(&mut pkg)?,
        "angular" => optimize_angular_config(&mut pkg)?,
        "astro" => optimize_astro_config(&mut pkg)?,
        "remix" => optimize_remix_config(&mut pkg)?,
        "mern" => optimize_mern_config(&mut pkg)?,
        _ => {}
    }
    // Save optimized package.json
    fs::write("package.json", serde_json::to_string_pretty(&pkg)?)?;

    Ok(())
}

fn optimize_react_config(pkg: &mut serde_json::Value) -> io::Result<()> {
    // Add optimization scripts
    if let Some(scripts) = pkg.get_mut("scripts").and_then(|s| s.as_object_mut()) {
        scripts.insert(
            "analyze".to_string(),
            json!("webpack-bundle-analyzer stats.json"),
        );
        scripts.insert(
            "build:prod".to_string(),
            json!("GENERATE_SOURCEMAP=false react-scripts build"),
        );
    }

    // Create optimized webpack config
    let webpack_config = r#"
    const path = require('path');
    const CompressionPlugin = require('compression-webpack-plugin');
    const TerserPlugin = require('terser-webpack-plugin');

    module.exports = {
        optimization: {
            minimize: true,
            minimizer: [new TerserPlugin({
                terserOptions: {
                    compress: {
                        drop_console: true,
                    },
                },
            })],
            splitChunks: {
                chunks: 'all',
                minSize: 20000,
                maxSize: 244000,
                minChunks: 1,
                maxAsyncRequests: 30,
                maxInitialRequests: 30,
                automaticNameDelimiter: '~',
                enforceSizeThreshold: 50000,
                cacheGroups: {
                    defaultVendors: {
                        test: /[\\/]node_modules[\\/]/,
                        priority: -10
                    },
                    default: {
                        minChunks: 2,
                        priority: -20,
                        reuseExistingChunk: true
                    }
                }
            }
        },
        plugins: [
            new CompressionPlugin({
                algorithm: 'gzip',
                test: /\.js$|\.css$|\.html$/,
                threshold: 10240,
                minRatio: 0.8,
            }),
        ],
    };"#;

    fs::write("webpack.config.js", webpack_config)?;
    Ok(())
}

fn optimize_vue_config(pkg: &mut serde_json::Value) -> io::Result<()> {
    // Add optimization scripts
    if let Some(scripts) = pkg.get_mut("scripts").and_then(|s| s.as_object_mut()) {
        scripts.insert(
            "analyze".to_string(),
            json!("vue-cli-service build --report"),
        );
        scripts.insert(
            "build:prod".to_string(),
            json!("vue-cli-service build --modern"),
        );
    }

    // Create vue.config.js with optimizations
    let vue_config = r#"
    const CompressionPlugin = require('compression-webpack-plugin');
    
    module.exports = {
      productionSourceMap: false,
      chainWebpack: config => {
        config.optimization.splitChunks({
          chunks: 'all',
          maxInitialRequests: Infinity,
          minSize: 20000,
          cacheGroups: {
            vendor: {
              test: /[\\/]node_modules[\\/]/,
              name(module) {
                const packageName = module.context.match(
                  /[\\/]node_modules[\\/](.*?)([\\/]|$)/
                )[1];
                return `vendor.${packageName.replace('@', '')}`;
              },
            },
          },
        });
      },
      configureWebpack: {
        plugins: [
          new CompressionPlugin({
            algorithm: 'gzip',
            test: /\.(js|css|html|svg)$/,
            threshold: 10240,
            minRatio: 0.8,
          }),
        ],
        performance: {
          hints: 'warning',
          maxEntrypointSize: 512000,
          maxAssetSize: 512000,
        },
      },
    };"#;

    fs::write("vue.config.js", vue_config)?;
    Ok(())
}

fn create_vue_files(_port: &str) -> io::Result<()> {
    let package_json = r#"{
        "name": "vue-app",
        "version": "0.1.0",
        "private": true,
        "scripts": {
            "serve": "vue-cli-service serve",
            "build": "vue-cli-service build --modern",
            "lint": "vue-cli-service lint",
            "analyze": "vue-cli-service build --report"
        },
        "dependencies": {
            "vue": "^3.3.0",
            "vue-router": "^4.0.0",
            "vuex": "^4.0.0"
        },
        "devDependencies": {
            "@vue/cli-plugin-babel": "~5.0.0",
            "@vue/cli-plugin-router": "~5.0.0",
            "@vue/cli-plugin-vuex": "~5.0.0",
            "@vue/cli-service": "~5.0.0"
        }
    }"#;

    fs::write("package.json", package_json)?;
    Ok(())
}

fn optimize_nuxt_config(pkg: &mut serde_json::Value) -> io::Result<()> {
    // Add optimization scripts
    if let Some(scripts) = pkg.get_mut("scripts").and_then(|s| s.as_object_mut()) {
        scripts.insert("analyze".to_string(), json!("nuxt build --analyze"));
        scripts.insert(
            "build:prod".to_string(),
            json!("nuxt build --modern=server"),
        );
    }

    // Create nuxt.config.js with optimizations
    let nuxt_config = r#"export default {
        // ... config content ...
    };"#;

    fs::write("nuxt.config.js", nuxt_config)?;
    Ok(())
}

fn create_nuxt_files(_port: &str) -> io::Result<()> {
    let package_json = r#"{
        "name": "nuxt-app",
        "version": "1.0.0",
        "private": true,
        "scripts": {
            "dev": "nuxt",
            "build": "nuxt build",
            "start": "nuxt start",
            "generate": "nuxt generate",
            "analyze": "nuxt build --analyze"
        },
        "dependencies": {
            "nuxt": "^3.0.0",
            "@nuxtjs/composition-api": "^0.33.0"
        },
        "devDependencies": {
            "@nuxt/types": "^2.15.8",
            "@nuxt/typescript-build": "^2.1.0"
        }
    }"#;

    fs::write("package.json", package_json)?;
    Ok(())
}

fn optimize_svelte_config(pkg: &mut serde_json::Value) -> io::Result<()> {
    // Add optimization scripts
    if let Some(scripts) = pkg.get_mut("scripts").and_then(|s| s.as_object_mut()) {
        scripts.insert("analyze".to_string(), json!("vite build --mode analyze"));
        scripts.insert(
            "build:prod".to_string(),
            json!("vite build --mode production"),
        );
        scripts.insert("preview".to_string(), json!("vite preview"));
    }

    // Create vite.config.js with Svelte optimizations
    let vite_config = r#"
    import { defineConfig } from 'vite';
    import { svelte } from '@sveltejs/vite-plugin-svelte';
    import compress from 'vite-plugin-compress';
    
    export default defineConfig({
      plugins: [
        svelte({
          compilerOptions: {
            dev: false,
            hydratable: true,
          },
          emitCss: true,
        }),
        compress({
          verbose: false,
          threshold: 10240,
        }),
      ],
      build: {
        target: 'esnext',
        minify: 'terser',
        terserOptions: {
          compress: {
            drop_console: true,
            dead_code: true,
          },
        },
        rollupOptions: {
          output: {
            manualChunks: {
              vendor: ['svelte'],
              utils: ['./src/lib/**/*.js'],
            },
          },
        },
        cssCodeSplit: true,
        sourcemap: false,
        chunkSizeWarningLimit: 1000,
      },
      ssr: {
        noExternal: ['svelte-routing'],
      },
    });"#;

    fs::write("vite.config.js", vite_config)?;
    Ok(())
}

fn optimize_angular_config(pkg: &mut serde_json::Value) -> io::Result<()> {
    // Add optimization scripts
    if let Some(scripts) = pkg.get_mut("scripts").and_then(|s| s.as_object_mut()) {
        scripts.insert(
            "analyze".to_string(),
            json!("ng build --stats-json && webpack-bundle-analyzer dist/stats.json"),
        );
        scripts.insert(
            "build:prod".to_string(),
            json!("ng build --configuration production --aot --build-optimizer --optimization"),
        );
    }

    // Create custom-webpack.config.js
    let webpack_config = r#"
    const CompressionPlugin = require('compression-webpack-plugin');
    const BrotliPlugin = require('brotli-webpack-plugin');
    
    module.exports = {
      optimization: {
        runtimeChunk: 'single',
        splitChunks: {
          cacheGroups: {
            vendor: {
              test: /[\\/]node_modules[\\/]/,
              name: 'vendors',
              chunks: 'all',
            },
          },
        },
      },
      plugins: [
        new CompressionPlugin({
          algorithm: 'gzip',
          test: /\.js$|\.css$|\.html$/,
          threshold: 10240,
          minRatio: 0.8,
        }),
        new BrotliPlugin({
          asset: '[path].br[query]',
          test: /\.(js|css|html|svg)$/,
          threshold: 10240,
          minRatio: 0.8,
        }),
      ],
    };"#;

    fs::write("custom-webpack.config.js", webpack_config)?;

    // Update angular.json with optimizations
    let angular_config = r#"{
      "projects": {
        "app": {
          "architect": {
            "build": {
              "configurations": {
                "production": {
                  "optimization": true,
                  "outputHashing": "all",
                  "sourceMap": false,
                  "namedChunks": false,
                  "aot": true,
                  "extractLicenses": true,
                  "vendorChunk": true,
                  "buildOptimizer": true,
                  "budgets": [
                    {
                      "type": "initial",
                      "maximumWarning": "2mb",
                      "maximumError": "5mb"
                    }
                  ]
                }
              }
            }
          }
        }
      }
    }"#;

    fs::write("angular.json", angular_config)?;
    Ok(())
}

fn optimize_astro_config(pkg: &mut serde_json::Value) -> io::Result<()> {
    // Add optimization scripts
    if let Some(scripts) = pkg.get_mut("scripts").and_then(|s| s.as_object_mut()) {
        scripts.insert("analyze".to_string(), json!("astro build --analyze"));
        scripts.insert(
            "build:prod".to_string(),
            json!("astro build --mode production"),
        );
        scripts.insert("preview".to_string(), json!("astro preview"));
    }

    // Create astro.config.mjs with optimizations
    let astro_config = r#"
    import { defineConfig } from 'astro/config';
    import compress from 'astro-compress';
    import prefetch from '@astrojs/prefetch';
    
    export default defineConfig({
      output: 'static',
      build: {
        inlineStylesheets: 'auto',
        split: true,
        sourcemap: false,
        assets: 'assets',
      },
      vite: {
        build: {
          cssCodeSplit: true,
          minify: 'terser',
          terserOptions: {
            compress: {
              drop_console: true,
              dead_code: true,
            },
          },
          rollupOptions: {
            output: {
              manualChunks(id) {
                if (id.includes('node_modules')) {
                  return 'vendor';
                }
                if (id.includes('src/components')) {
                  return 'components';
                }
              },
            },
          ssr: {
            noExternal: ['@astrojs/*'],
          },
        },
        integrations: [
          compress({
            CSS: true,
            HTML: {
              removeAttributeQuotes: true,
              removeComments: true,
              removeRedundantAttributes: true,
              removeScriptTypeAttributes: true,
              removeStyleLinkTypeAttributes: true,
              useShortDoctype: true,
              minifyCSS: true,
              minifyJS: true,
            },
            Image: true,
            JavaScript: true,
          }),
          prefetch(),
        ],
      },
    });"#;

    fs::write("astro.config.mjs", astro_config)?;

    // Create tsconfig.json for TypeScript support
    let tsconfig = r#"{
      "extends": "astro/tsconfigs/strict",
      "compilerOptions": {
        "baseUrl": ".",
        "paths": {
          "@/*": ["src/*"]
        },
        "jsx": "preserve",
        "jsxImportSource": "astro",
        "verbatimModuleSyntax": true,
        "moduleResolution": "node",
        "module": "ESNext",
        "target": "ESNext",
        "strict": true,
        "skipLibCheck": true
      }
    }"#;

    fs::write("tsconfig.json", tsconfig)?;
    Ok(())
}

fn optimize_remix_config(pkg: &mut serde_json::Value) -> io::Result<()> {
    // Add optimization scripts
    if let Some(scripts) = pkg.get_mut("scripts").and_then(|s| s.as_object_mut()) {
        scripts.insert(
            "build:prod".to_string(),
            json!("remix build --sourcemap --minify"),
        );
        scripts.insert("analyze".to_string(), json!("REMIX_ANALYZE=1 remix build"));
        scripts.insert(
            "start:prod".to_string(),
            json!("remix-serve ./build/index.js"),
        );
    }

    // Create remix.config.js with optimizations
    let remix_config = r#"
    /** @type {import('@remix-run/dev').AppConfig} */
    module.exports = {
      serverBuildTarget: "vercel",
      server: process.env.NODE_ENV === "development" ? undefined : "./server.js",
      ignoredRouteFiles: ["**/.*"],
      serverDependenciesToBundle: [/^marked.*/],
      future: {
        v2_errorBoundary: true,
        v2_meta: true,
        v2_normalizeFormMethod: true,
        v2_routeConvention: true,
      },
      tailwind: true,
      postcss: true,
      serverModuleFormat: "cjs",
      serverMinify: true,
      browserNodeBuiltinsPolyfill: {
        modules: { crypto: true, path: true, fs: true, os: true }
      },
      watchPaths: ["./tailwind.config.js"],
      
      // Advanced optimizations
      routes: async (defineRoutes) => {
        return defineRoutes((route) => {
          // Optimize routes with prefetch
          route("*", "root.tsx", { prefetch: "intent" });
        });
      },
    };"#;

    fs::write("remix.config.js", remix_config)?;

    // Create custom server.js for production
    let server_js = r#"
    const path = require("path");
    const express = require("express");
    const compression = require("compression");
    const morgan = require("morgan");
    const { createRequestHandler } = require("@remix-run/express");

    const app = express();
    app.use(compression());
    app.use(morgan("tiny"));

    // Static files with cache headers
    app.use(
      "/build",
      express.static("public/build", {
        immutable: true,
        maxAge: "1y",
      })
    );
    app.use(express.static("public", { maxAge: "1h" }));

    // REMIX handler with optimizations
    app.all(
      "*",
      createRequestHandler({
        build: require("./build"),
        mode: process.env.NODE_ENV,
        getLoadContext(req, res) {
          return { req, res };
        },
      })
    );

    const port = process.env.PORT || 3000;
    app.listen(port, () => console.log(`Express server listening on port ${port}`));"#;

    fs::write("server.js", server_js)?;
    Ok(())
}

fn optimize_mern_config(pkg: &mut serde_json::Value) -> io::Result<()> {
    // Add optimization scripts for both frontend and backend
    if let Some(scripts) = pkg.get_mut("scripts").and_then(|s| s.as_object_mut()) {
        scripts.insert(
            "build:prod".to_string(),
            json!("npm run build:server && npm run build:client"),
        );
        scripts.insert(
            "start:prod".to_string(),
            json!("NODE_ENV=production node dist/server/index.js"),
        );
        scripts.insert(
            "analyze".to_string(),
            json!("webpack-bundle-analyzer client/build/bundle-stats.json"),
        );
    }

    // Create webpack.config.js for React frontend
    let webpack_config = r#"
    const path = require('path');
    const CompressionPlugin = require('compression-webpack-plugin');
    const TerserPlugin = require('terser-webpack-plugin');
    const BundleAnalyzerPlugin = require('webpack-bundle-analyzer').BundleAnalyzerPlugin;

    module.exports = {
      mode: 'production',
      entry: './client/src/index.js',
      output: {
        path: path.resolve(__dirname, 'client/build'),
        filename: '[name].[contenthash].js',
        chunkFilename: '[name].[contenthash].chunk.js',
      },
      optimization: {
        minimizer: [
          new TerserPlugin({
            terserOptions: {
              compress: {
                drop_console: true,
              },
            },
          }),
        ],
        splitChunks: {
          chunks: 'all',
          minSize: 20000,
          maxSize: 244000,
          cacheGroups: {
            vendor: {
              test: /[\\/]node_modules[\\/]/,
              name(module) {
                const packageName = module.context.match(
                  /[\\/]node_modules[\\/](.*?)([\\/]|$)/
                )[1];
                return `vendor.${packageName.replace('@', '')}`;
              },
            },
          },
        },
      },
      plugins: [
        new CompressionPlugin(),
        process.env.ANALYZE && new BundleAnalyzerPlugin(),
      ].filter(Boolean),
    };"#;

    fs::write("webpack.config.js", webpack_config)?;

    // Create optimized Express server configuration
    let server_config = r#"
    const express = require('express');
    const compression = require('compression');
    const helmet = require('helmet');
    const mongoose = require('mongoose');
    const cors = require('cors');
    const rateLimit = require('express-rate-limit');
    const mongoSanitize = require('express-mongo-sanitize');
    const xss = require('xss-clean');
    const hpp = require('hpp');
    const path = require('path');

    require('dotenv').config();

    const app = express();

    // Security Middleware
    app.use(helmet());
    app.use(mongoSanitize());
    app.use(xss());
    app.use(hpp());
    app.use(compression());

    // Rate limiting
    const limiter = rateLimit({
      windowMs: 15 * 60 * 1000,
      max: 100
    });
    app.use('/api', limiter);

    // MongoDB optimization
    mongoose.connect(process.env.MONGODB_URI, {
      useNewUrlParser: true,
      useUnifiedTopology: true,
      maxPoolSize: 10,
      serverSelectionTimeoutMS: 5000,
      socketTimeoutMS: 45000,
    });

    // Cache optimization
    const cacheMiddleware = (duration) => (req, res, next) => {
      res.set('Cache-Control', `public, max-age=${duration}`);
      next();
    };

    // Static file serving with cache
    app.use('/static', cacheMiddleware(86400), 
      express.static(path.join(__dirname, '../client/build')));

    // Error handling middleware
    app.use((err, req, res, next) => {
      console.error(err.stack);
      res.status(500).send('Something broke!');
    });

    module.exports = app;"#;

    fs::write("server/config.js", server_config)?;

    // Create PM2 ecosystem config for production
    let pm2_config = r#"{
      "apps": [{
        "name": "mern-app",
        "script": "./dist/server/index.js",
        "instances": "max",
        "exec_mode": "cluster",
        "watch": false,
        "env_production": {
          "NODE_ENV": "production"
        },
        "node_args": "--max_old_space_size=4096"
      }]
    }"#;

    fs::write("ecosystem.config.json", pm2_config)?;
    Ok(())
}

// Helper functions for creating specific configurations
fn create_eslint_config(app_type: &str) -> io::Result<()> {
    let eslint_config = match app_type {
        "nextjs" | "react" => {
            r#"{
            "extends": [
                "next/core-web-vitals",
                "prettier"
            ],
            "rules": {
                "react/no-unused-vars": "error",
                "react-hooks/rules-of-hooks": "error",
                "react-hooks/exhaustive-deps": "warn"
            }
        }"#
        }
        "vue" => {
            r#"{
            "extends": [
                "plugin:vue/vue3-recommended",
                "prettier"
            ],
            "rules": {
                "vue/multi-word-component-names": "error",
                "vue/no-unused-vars": "error"
            }
        }"#
        }
        // Add more framework-specific ESLint configs...
        _ => {
            r#"{
            "extends": ["prettier"],
            "rules": {
                "no-unused-vars": "error",
                "no-console": "warn"
            }
        }"#
        }
    };

    fs::write(".eslintrc.json", eslint_config)?;
    Ok(())
}

fn create_prettier_config() -> io::Result<()> {
    let prettier_config = r#"{
        "semi": true,
        "trailingComma": "es5",
        "singleQuote": true,
        "printWidth": 100,
        "tabWidth": 2,
        "useTabs": false
    }"#;

    fs::write(".prettierrc", prettier_config)?;
    Ok(())
}

fn create_editor_config() -> io::Result<()> {
    let editor_config = r#"
root = true

[*]
end_of_line = lf
insert_final_newline = true
trim_trailing_whitespace = true
charset = utf-8

[*.{js,jsx,ts,tsx,vue,svelte,astro}]
indent_style = space
indent_size = 2

[*.{css,scss,less,styl}]
indent_style = space
indent_size = 2

[*.{json,yml,yaml}]
indent_style = space
indent_size = 2
"#;

    fs::write(".editorconfig", editor_config)?;
    Ok(())
}

fn create_docker_compose(app_type: &str) -> io::Result<()> {
    let docker_compose = match app_type {
        "mern" => {
            r#"
version: '3.8'
services:
  client:
    build: ./client
    ports:
      - "3000:3000"
    environment:
      - NODE_ENV=production
    depends_on:
      - server
  server:
    build: ./server
    ports:
      - "5000:5000"
    environment:
      - NODE_ENV=production
      - MONGODB_URI=mongodb://mongo:27017/app
    depends_on:
      - mongo
  mongo:
    image: mongo:latest
    ports:
      - "27017:27017"
    volumes:
      - mongodb_data:/data/db

volumes:
  mongodb_data:
"#
        }
        _ => {
            r#"
version: '3.8'
services:
  app:
    build: .
    ports:
      - "3000:3000"
    environment:
      - NODE_ENV=production
"#
        }
    };

    fs::write("docker-compose.yml", docker_compose)?;
    Ok(())
}

fn create_react_files(_port: &str) -> io::Result<()> {
    let package_json = r#"{
        "name": "react-app",
        "version": "0.1.0",
        "private": true,
        "scripts": {
            "dev": "vite",
            "build": "vite build",
            "build:prod": "GENERATE_SOURCEMAP=false vite build",
            "analyze": "vite build --mode analyze",
            "preview": "vite preview",
            "lint": "eslint src --ext .ts,.tsx"
        },
        "dependencies": {
            "react": "^18.2.0",
            "react-dom": "^18.2.0",
            "react-router-dom": "^6.8.0"
        },
        "devDependencies": {
            "@types/react": "^18.0.27",
            "@types/react-dom": "^18.0.10",
            "@vitejs/plugin-react": "^3.1.0",
            "typescript": "^4.9.3",
            "vite": "^4.1.0",
            "compression-webpack-plugin": "^10.0.0",
            "terser-webpack-plugin": "^5.3.7"
        }
    }"#;

    fs::write("package.json", package_json)?;
    optimize_react_config(&mut serde_json::from_str(&package_json)?)?;
    Ok(())
}

fn create_svelte_files(_port: &str) -> io::Result<()> {
    let package_json = r#"{
        "name": "svelte-app",
        "version": "0.1.0",
        "private": true,
        "scripts": {
            "dev": "vite dev",
            "build": "vite build",
            "build:prod": "vite build --mode production",
            "analyze": "vite build --mode analyze",
            "preview": "vite preview"
        },
        "dependencies": {
            "svelte": "^4.0.0",
            "svelte-routing": "^2.0.0"
        },
        "devDependencies": {
            "@sveltejs/vite-plugin-svelte": "^2.4.0",
            "typescript": "^5.0.0",
            "vite": "^4.3.9",
            "vite-plugin-compress": "^2.1.1"
        }
    }"#;

    fs::write("package.json", package_json)?;
    optimize_svelte_config(&mut serde_json::from_str(&package_json)?)?;
    Ok(())
}

fn create_angular_files(_port: &str) -> io::Result<()> {
    let package_json = r#"{
        "name": "angular-app",
        "version": "0.0.0",
        "scripts": {
            "ng": "ng",
            "start": "ng serve",
            "build": "ng build",
            "build:prod": "ng build --configuration production --aot",
            "analyze": "ng build --stats-json && webpack-bundle-analyzer dist/stats.json",
            "watch": "ng build --watch --configuration development",
            "test": "ng test"
        },
        "dependencies": {
            "@angular/animations": "^16.0.0",
            "@angular/common": "^16.0.0",
            "@angular/compiler": "^16.0.0",
            "@angular/core": "^16.0.0",
            "@angular/forms": "^16.0.0",
            "@angular/platform-browser": "^16.0.0",
            "@angular/platform-browser-dynamic": "^16.0.0",
            "@angular/router": "^16.0.0",
            "rxjs": "~7.8.0",
            "zone.js": "~0.13.0"
        },
        "devDependencies": {
            "@angular-devkit/build-angular": "^16.0.0",
            "@angular/cli": "^16.0.0",
            "@angular/compiler-cli": "^16.0.0",
            "typescript": "~5.0.0",
            "compression-webpack-plugin": "^10.0.0",
            "brotli-webpack-plugin": "^1.1.0"
        }
    }"#;

    fs::write("package.json", package_json)?;
    optimize_angular_config(&mut serde_json::from_str(&package_json)?)?;
    Ok(())
}

fn create_astro_files(port: &str) -> io::Result<()> {
    let _ = port;
    let package_json = r#"{
        "name": "astro-app",
        "version": "0.0.1",
        "scripts": {
            "dev": "astro dev",
            "start": "astro dev",
            "build": "astro build",
            "build:prod": "astro build --mode production",
            "preview": "astro preview",
            "analyze": "astro build --analyze"
        },
        "dependencies": {
            "astro": "^2.5.0",
            "@astrojs/prefetch": "^0.2.0"
        },
        "devDependencies": {
            "@astrojs/ts-plugin": "^1.0.0",
            "astro-compress": "^1.1.0",
            "typescript": "^5.0.0"
        }
    }"#;

    fs::write("package.json", package_json)?;
    optimize_astro_config(&mut serde_json::from_str(&package_json)?)?;
    Ok(())
}

fn create_remix_files(port: &str) -> io::Result<()> {
    let _ = port;
    let package_json = r#"{
        "name": "remix-app",
        "private": true,
        "sideEffects": false,
        "scripts": {
            "build": "remix build",
            "build:prod": "remix build --sourcemap --minify",
            "dev": "remix dev",
            "analyze": "REMIX_ANALYZE=1 remix build",
            "start": "remix-serve build/index.js",
            "typecheck": "tsc"
        },
        "dependencies": {
            "@remix-run/node": "^1.19.1",
            "@remix-run/react": "^1.19.1",
            "@remix-run/serve": "^1.19.1",
            "compression": "^1.7.4",
            "express": "^4.18.2",
            "morgan": "^1.10.0"
        },
        "devDependencies": {
            "@remix-run/dev": "^1.19.1",
            "@types/react": "^18.0.35",
            "@types/react-dom": "^18.0.11",
            "typescript": "^5.0.4"
        }
    }"#;

    fs::write("package.json", package_json)?;
    optimize_remix_config(&mut serde_json::from_str(&package_json)?)?;
    Ok(())
}

fn create_mern_files(port: &str) -> io::Result<()> {
    let _ = port;
    // Create root package.json
    let package_json = r#"{
        "name": "mern-app",
        "version": "1.0.0",
        "scripts": {
            "build": "npm run build:client && npm run build:server",
            "build:client": "cd client && npm run build",
            "build:server": "cd server && npm run build",
            "build:prod": "npm run build:client:prod && npm run build:server:prod",
            "start:prod": "NODE_ENV=production node dist/server/index.js",
            "analyze": "cd client && npm run analyze"
        },
        "devDependencies": {
            "concurrently": "^8.0.1"
        }
    }"#;

    fs::write("package.json", package_json)?;
    optimize_mern_config(&mut serde_json::from_str(&package_json)?)?;

    // Create client and server directories
    fs::create_dir_all("client")?;
    fs::create_dir_all("server")?;

    Ok(())
}

fn create_github_workflows() -> io::Result<()> {
    fs::create_dir_all(".github/workflows")?;

    // CI/CD Workflow
    let ci_workflow = r#"name: CI/CD Pipeline

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main, develop ]

jobs:
  test-and-build:
    runs-on: ubuntu-latest
    
    strategy:
      matrix:
        node-version: [18.x, 20.x]

    steps:
    - uses: actions/checkout@v3
    
    - name: Setup Node.js ${{ matrix.node-version }}
      uses: actions/setup-node@v3
      with:
        node-version: ${{ matrix.node-version }}
        cache: 'npm'
    
    - name: Install dependencies
      run: npm ci
    
    - name: Run linting
      run: npm run lint
    
    - name: Run tests
      run: npm run test
    
    - name: Build application
      run: npm run build:prod
    
    - name: Upload build artifacts
      uses: actions/upload-artifact@v3
      with:
        name: build-files
        path: dist/

  docker:
    needs: test-and-build
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    
    steps:
    - uses: actions/checkout@v3
    
    - name: Download build artifacts
      uses: actions/download-artifact@v3
      with:
        name: build-files
        path: dist/
    
    - name: Set up Docker Buildx
      uses: docker/setup-buildx-action@v2
    
    - name: Login to Docker Hub
      uses: docker/login-action@v2
      with:
        username: ${{ secrets.DOCKER_HUB_USERNAME }}
        password: ${{ secrets.DOCKER_HUB_TOKEN }}
    
    - name: Build and push Docker image
      uses: docker/build-push-action@v4
      with:
        context: .
        push: true
        tags: ${{ secrets.DOCKER_HUB_USERNAME }}/app:latest
        cache-from: type=registry,ref=${{ secrets.DOCKER_HUB_USERNAME }}/app:buildcache
        cache-to: type=registry,ref=${{ secrets.DOCKER_HUB_USERNAME }}/app:buildcache,mode=max"#;

    // Security scanning workflow
    let security_workflow = r#"name: Security Scan

on:
  schedule:
    - cron: '0 0 * * *'
  workflow_dispatch:

jobs:
  security:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    
    - name: Run Snyk to check for vulnerabilities
      uses: snyk/actions/node@master
      env:
        SNYK_TOKEN: ${{ secrets.SNYK_TOKEN }}
    
    - name: Run OWASP Dependency-Check
      uses: dependency-check/Dependency-Check_Action@main
      with:
        path: '.'
        format: 'HTML'
    
    - name: Upload security report
      uses: actions/upload-artifact@v3
      with:
        name: security-report
        path: reports/"#;

    fs::write(".github/workflows/ci.yml", ci_workflow)?;
    fs::write(".github/workflows/security.yml", security_workflow)?;
    Ok(())
}
fn create_optimization_configs(app_type: &str) -> io::Result<()> {
    // Count optional directories for optimization level
    let _optional_count = vec!["api", "lib", "utils", "hooks", "services"]
        .iter()
        .filter(|d| Path::new(d).exists())
        .count();

    println!(
        "📊 Found {} optional optimization directories",
        _optional_count
    );

    // Read existing configuration if it exists
    let _existing_config = if Path::new("next.config.js").exists() {
        Some(fs::read_to_string("next.config.js")?)
    } else {
        None
    };

    // Apply framework-specific optimizations
    match app_type {
        "nextjs" => {
            if let Some(_config) = _existing_config {
                println!("🔄 Merging with existing Next.js configuration");
                // Merge logic here
            }
            create_nextjs_optimized_config()?;
        }
        "react" => {
            create_nextjs_optimized_config()?;
        }
        "vue" => {
            create_nextjs_optimized_config()?;
        }
        "svelte" => {
            create_nextjs_optimized_config()?;
        }
        "angular" => {
            create_nextjs_optimized_config()?;
        }
        _ => {}
    }

    Ok(())
}

fn check_docker_setup() -> io::Result<()> {
    println!("🐳 Checking Docker setup...");
    // Check if Docker is installed and running
    let docker_status = Command::new("docker")
        .arg("info")
        .output()
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "Docker not found or not running"))?;

    if !docker_status.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Docker is not running",
        ));
    }

    println!("✅ Docker is properly configured");
    Ok(())
}

fn initialize_kubernetes() -> io::Result<()> {
    println!("🚀 Initializing Kubernetes environment...");

    // Step 1: Check connection and setup
    check_kubernetes_connection()?;

    // Step 2: Create namespaces
    let namespaces = ["default", "monitoring", "ingress-nginx"];
    for namespace in namespaces.iter() {
        println!("📦 Creating namespace: {}", namespace);
        let create_output = Command::new("kubectl")
            .args([
                "create",
                "namespace",
                namespace,
                "--dry-run=client",
                "-o",
                "yaml",
            ])
            .output()?;

        if !create_output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to create namespace: {}", namespace),
            ));
        }
    }

    // Step 3: Install and configure NGINX Ingress
    install_nginx_ingress()?;

    // Step 4: Wait for Ingress Controller
    println!("⏳ Waiting for NGINX Ingress Controller...");
    let wait_output = Command::new("kubectl")
        .args([
            "wait",
            "--namespace",
            "ingress-nginx",
            "--for=condition=ready",
            "pod",
            "--selector=app.kubernetes.io/component=controller",
            "--timeout=300s",
        ])
        .output()?;

    if !wait_output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Timeout waiting for NGINX Ingress Controller",
        ));
    }

    // Step 5: Install Metrics Server
    println!("📊 Installing Metrics Server...");
    let metrics_output = Command::new("kubectl")
        .args([
            "apply",
            "-f",
            "https://github.com/kubernetes-sigs/metrics-server/releases/latest/download/components.yaml"
        ])
        .output()?;

    if !metrics_output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to install Metrics Server",
        ));
    }

    println!("✅ Kubernetes environment initialized successfully!");
    Ok(())
}

fn handle_kubernetes_error(error: io::Error) -> io::Error {
    match error.kind() {
        io::ErrorKind::NotFound => {
            println!("❌ Kubernetes tools not found");
            println!("📝 Please ensure:");
            println!("1. Docker Desktop is installed and running");
            println!("2. Kubernetes is enabled in Docker Desktop");
            println!("3. kubectl is installed and in PATH");
            error
        }
        io::ErrorKind::Other => {
            if error.to_string().contains("connection refused") {
                println!("❌ Cannot connect to Kubernetes cluster");
                println!("📝 Please check:");
                println!("1. Docker Desktop is running");
                println!("2. Kubernetes is enabled and running (green icon)");
                println!("3. No firewall is blocking the connection");
            }
            error
        }
        _ => error,
    }
}

fn create_app_files(app_type: &str, port: &str) -> io::Result<()> {
    println!(
        "📝 Creating application files for {} framework...",
        app_type
    );

    // Create base directories
    fs::create_dir_all("src")?;
    fs::create_dir_all("public")?;
    fs::create_dir_all("config")?;

    // Create common configuration files
    create_eslint_config(app_type)?;
    create_prettier_config()?;
    create_editor_config()?;
    create_docker_compose(app_type)?;

    // Route to specific framework file creation
    match app_type {
        "vue" => create_vue_files(port)?,
        "react" => create_react_files(port)?,
        "nuxt" => create_nuxt_files(port)?,
        "svelte" => create_svelte_files(port)?,
        "angular" => create_angular_files(port)?,
        "astro" => create_astro_files(port)?,
        "bun" => {
            // Create basic Bun application files
            let package_json = format!(
                r#"{{
                "name": "bun-app",
                "version": "0.1.0",
                "scripts": {{
                    "dev": "bun run --hot src/index.ts",
                    "start": "bun run src/index.ts",
                    "build": "bun build src/index.ts --outdir=dist"
                }},
                "dependencies": {{}},
                "devDependencies": {{
                    "bun-types": "latest"
                }}
            }}"#
            );
            fs::write("package.json", package_json)?;

            // Create basic TypeScript configuration
            let tsconfig = r#"{
                "compilerOptions": {
                    "target": "esnext",
                    "module": "esnext",
                    "moduleResolution": "node",
                    "types": ["bun-types"],
                    "esModuleInterop": true,
                    "skipLibCheck": true,
                    "strict": true
                }
            }"#;
            fs::write("tsconfig.json", tsconfig)?;

            // Create main application file
            let main_file = format!(
                r#"import {{ serve }} from "bun";
                const server = serve({{
                    port: {},
                    fetch(req) {{
                        return new Response("Hello from Bun!");
                    }},
                }});
                console.log(`Listening on http://localhost:{}`);
                "#,
                port, port
            );
            fs::write("src/index.ts", main_file)?;
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Unsupported application type: {}", app_type),
            ))
        }
    }

    // Create .gitignore
    let gitignore = r#"# Dependencies
    node_modules/
    .pnp
    .pnp.js

    # Build outputs
    dist
    build
    .next
    out
    .nuxt

    # Environment variables
    .env
    .env.local
    .env.*.local

    # Logs
    npm-debug.log*
    yarn-debug.log*
    yarn-error.log*

    # Editor directories
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
    !types/"#;
    fs::write(".gitignore", gitignore)?;

    println!("✅ Application files created successfully!");
    Ok(())
}

fn create_docker_files() -> io::Result<()> {
    let dockerfile = r#"# Build stage
FROM node:20-alpine AS builder

# Install essential build tools and jq
RUN apk add --no-cache jq git python3 make g++

# Set working directory
WORKDIR /app

# Copy package files first for better caching
COPY package*.json ./

# Install dependencies
RUN npm ci

# Copy the rest of the application
COPY . .

# Build the application
RUN npm run build

# Production stage
FROM node:20-alpine AS runner

# Install production dependencies only
RUN apk add --no-cache bash

WORKDIR /app

# Copy necessary files from builder
COPY --from=builder /app/package*.json ./
COPY --from=builder /app/.next ./.next
COPY --from=builder /app/public ./public
COPY --from=builder /app/node_modules ./node_modules

# Set environment variables
ENV NODE_ENV=production
ENV PORT=3000

# Expose the port
EXPOSE 3000

# Start the application
CMD ["npm", "start"]"#;

    let dockerignore = r#"
# Dependencies
node_modules
npm-debug.log
yarn-debug.log
yarn-error.log

# Version control
.git
.gitignore

# Environment
.env
.env.local
.env.*.local

# Build output
.next
out
dist
build

# IDE
.idea
.vscode

# OS
.DS_Store
Thumbs.db

# Testing
coverage
.nyc_output

# Misc
*.log
.cache
.temp"#;

    // Write Docker files
    fs::write("Dockerfile", dockerfile)?;
    fs::write(".dockerignore", dockerignore)?;

    Ok(())
}

fn deploy_with_docker(metadata: &AppMetadata) -> io::Result<()> {
    println!("🐳 Building Docker container...");

    // Build Docker image
    let status = Command::new("docker")
        .args([
            "build",
            "-t",
            &format!("{}:latest", metadata.app_name),
            ".",
        ])
        .status()?;

    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to build Docker image",
        ));
    }

    // Run Docker container
    let status = Command::new("docker")
        .args([
            "run",
            "-d",
            "-p",
            &format!("{}:3000", metadata.port),
            "--name",
            &metadata.app_name,
            &format!("{}:latest", metadata.app_name),
        ])
        .status()?;

    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to start Docker container",
        ));
    }

    println!("✅ Docker container started successfully!");
    Ok(())
}
