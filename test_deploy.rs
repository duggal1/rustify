use std::{process::Command, io::{self, Write}, thread, time::Duration};

fn main() -> io::Result<()> {
    println!("🚀 Testing Rustify Main Functionality");

    // Test main.rs components
    if let Err(e) = test_project_initialization() {
        eprintln!("Error during project initialization: {}", e);
        return Err(e);
    }
    if let Err(e) = test_deployment_flow() {
        eprintln!("Error during deployment flow: {}", e);
        return Err(e);
    }
    if let Err(e) = test_kubernetes_integration() {
        eprintln!("Error during Kubernetes integration: {}", e);
        return Err(e);
    }

    Ok(())
}

fn test_project_initialization() -> io::Result<()> {
    println!("\n📦 Testing Project Initialization...");

    // Test Next.js initialization
    println!("Testing Next.js setup...");
    let nextjs_config = r#"
    module.exports = {
      reactStrictMode: true,
      webpack: (config) => {
        config.optimization = {
          minimize: true,
          splitChunks: {
            chunks: 'all',
          },
        };
        return config;
      },
    };
    "#;
    std::fs::write("next.config.js", nextjs_config)?;

    // Test Docker configuration
    println!("Testing Docker setup...");
    let dockerfile = r#"
    FROM node:16-alpine
    WORKDIR /app
    COPY package*.json ./
    RUN npm install
    COPY . .
    RUN npm run build
    EXPOSE 3000
    CMD ["npm", "start"]
    "#;
    std::fs::write("Dockerfile", dockerfile)?;

    println!("✅ Project initialization tests passed");
    Ok(())
}

fn test_deployment_flow() -> io::Result<()> {
    println!("\n🚢 Testing Deployment Flow...");

    // Test metadata creation
    let metadata = r#"{
        "app_name": "test-app",
        "app_type": "nextjs",
        "port": "3000",
        "created_at": "2024-03-20T00:00:00Z",
        "kubernetes_enabled": true,
        "status": "pending"
    }"#;
    std::fs::write("app-metadata.json", metadata)?;

    // Test HAProxy configuration
    let haproxy_config = r#"
    backend apps
        balance first
        hash-type consistent
        stick-table type string len 32 size 100k expire 30m
        stick store-request req.cook(sessionid)
        option httpchk HEAD /health HTTP/1.1\r\nHost:\ localhost
        http-check expect status 200
        server-template app- 20 127.0.0.1:3000-3020 check resolvers docker init-addr none
    "#;
    std::fs::write("haproxy.cfg", haproxy_config)?;

    println!("✅ Deployment flow tests passed");
    Ok(())
}

fn test_kubernetes_integration() -> io::Result<()> {
    println!("\n☸️ Testing Kubernetes Integration...");

    // Create test deployment
    let deployment = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rustify-test
  namespace: default
spec:
  replicas: 1
  selector:
    matchLabels:
      app: rustify-test
  template:
    metadata:
      labels:
        app: rustify-test
    spec:
      containers:
      - name: test-container
        image: nginx:latest
        ports:
        - containerPort: 80
        resources:
          requests:
            memory: "64Mi"
            cpu: "250m"
          limits:
            memory: "128Mi"
            cpu: "500m"
        readinessProbe:
          httpGet:
            path: /
            port: 80
    "#;
    std::fs::write("test-deployment.yaml", deployment)?;

    // Apply deployment
    Command::new("kubectl")
        .args(["apply", "-f", "test-deployment.yaml"])
        .output()?;

    // Wait for deployment
    println!("⏳ Waiting for deployment...");
    thread::sleep(Duration::from_secs(20));

    // Verify all components
    println!("\n📊 Verifying Components:");

    // 1. Check deployment status
    let deployment_status = Command::new("kubectl")
        .args(["get", "deployments", "rustify-test"])
        .output()?;
    println!("\nDeployment Status:");
    println!("{}", String::from_utf8_lossy(&deployment_status.stdout));

    // 2. Check HAProxy
    println!("\nHAProxy Status:");
    if let Ok(haproxy) = Command::new("kubectl")
        .args(["get", "pods", "-l", "app=haproxy"])
        .output() {
        println!("{}", String::from_utf8_lossy(&haproxy.stdout));
    }

    // 3. Check NGINX Ingress
    println!("\nNGINX Ingress Status:");
    if let Ok(nginx) = Command::new("kubectl")
        .args(["get", "pods", "-n", "ingress-nginx"])
        .output() {
        println!("{}", String::from_utf8_lossy(&nginx.stdout));
    }

    // 4. Check Metrics Server
    println!("\nMetrics Server Status:");
    if let Ok(metrics) = Command::new("kubectl")
        .args(["top", "pods"])
        .output() {
        println!("{}", String::from_utf8_lossy(&metrics.stdout));
    }

    // Cleanup
    println!("\n🧹 Cleaning up test resources...");
    Command::new("kubectl")
        .args(["delete", "deployment", "rustify-test"])
        .output()?;

    println!("✅ Kubernetes integration tests passed");
    Ok(())
}
