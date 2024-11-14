# # 🚀 Rustify CLI - Comprehensive User Guide

## Table of Contents
- [Installation](#installation)
- [Getting Started](#getting-started)
- [Framework Support](#framework-support)
- [Deployment Guide](#deployment-guide)
- [Production Features](#production-features)
- [Auto-scaling Guide](#auto-scaling-guide)
- [Troubleshooting](#troubleshooting)

## Installation

### macOS
```bash
# Using curl (recommended)
curl -fsSL https://raw.githubusercontent.com/duggal1/rustify/main/install.sh | bash
# Verify installation
rustify --version
```

### Linux
```bash
# Using curl
curl -fsSL https://raw.githubusercontent.com/duggal1/rustify/main/install.sh | bash
```

### Windows
```powershell
# Run PowerShell as Administrator
iwr -useb https://raw.githubusercontent.com/duggal1/rustify/main/install.ps1 | iex
```

## Getting Started

1. **Initialize a New Project**
   ```bash
   # Create a new project with default framework (Bun)
   rustify init
   # Create a project with specific framework
   rustify init --type react
   ```

2. **Basic Deployment**
   ```bash
   # Development deployment
   rustify deploy
   # Production deployment
   rustify deploy --prod
   ```

## Framework Support

### React Projects
```bash
# Initialize
rustify init --type react
# Deploy with optimizations
rustify deploy --prod
```
Features:
- Webpack optimization
- Code splitting
- Tree shaking
- Bundle analysis

### Vue Projects
```bash
# Initialize
rustify init --type vue
# Deploy with modern mode
rustify deploy --prod
```
Features:
- Modern mode building
- Auto compression
- Asset optimization

### MERN Stack
```bash
# Initialize
rustify init --type mern
# Deploy full stack
rustify deploy --prod
```
Features:
- Full-stack optimization
- MongoDB configuration
- Express server setup
- React optimization

### Other Supported Frameworks
```bash
# Svelte
rustify init --type svelte
# Angular
rustify init --type angular
# Astro
rustify init --type astro
# Remix
rustify init --type remix
```

## Deployment Guide

### Development Deployment
```bash
# Basic deployment
rustify deploy
# Custom port
rustify deploy --port 3000
# With cleanup
rustify deploy --cleanup
```

### Production Deployment
```bash
# Full production setup
rustify deploy --prod
# Production with custom port
rustify deploy --prod --port 3000
# Production with auto-scaling
rustify deploy --prod --rpl
```

### Advanced Deployment Options
```bash
# Full production deployment with all features
rustify deploy --prod --rpl --port 3000 --cleanup
```

## Production Features

1. **Docker Integration**
   - Automatic Docker setup
   - Optimized Dockerfile generation
   - Multi-stage builds
   - Layer caching

2. **Kubernetes Setup**
   ```bash
   # Deploy to Kubernetes
   rustify deploy --prod
   ```
   Features:
   - Automatic namespace creation
   - Resource management
   - Health checks
   - Load balancing

3. **Monitoring**
   - CPU usage tracking
   - Memory monitoring
   - Request tracking
   - Error rate monitoring

## Auto-scaling Guide

### Enable Auto-scaling
```bash
# Enable auto-scaling in production
rustify deploy --prod --rpl
```

### Auto-scaling Features
- CPU-based scaling (70% threshold)
- Memory-based scaling (80% threshold)
- Automatic replica management
- Scale up to 10 pods
- Intelligent scaling policies

### Scaling Configuration
```yaml
# Default scaling configuration
minReplicas: 1
maxReplicas: 10
metrics:
  cpu: 70%
  memory: 80%
```

## Troubleshooting

### Common Issues

1. **Docker Issues**
   ```bash
   # If Docker isn't running
   Error: Docker daemon not responding
   Solution: Start Docker Desktop manually
   ```

2. **Port Conflicts**
   ```bash
   # If default port is in use
   rustify deploy --port 3001
   ```

3. **Deployment Failures**
   ```bash
   # Clean up and retry
   rustify deploy --cleanup
   ```

### Best Practices

- **Development**
  ```bash
  # Use development mode for testing
  rustify deploy
  ```
- **Production**
  ```bash
  # Always use production mode with cleanup
  rustify deploy --prod --cleanup
  ```
- **High Traffic Apps**
  ```bash
  # Enable auto-scaling for production
  rustify deploy --prod --rpl
  ```

### Performance Tips

- **Resource Optimization**
  - Use `--prod` flag for production optimizations
  - Enable auto-scaling for high-traffic periods
  - Use `--cleanup` flag for fresh deployments
- **Monitoring**
  - Check logs regularly
  - Monitor resource usage
  - Track scaling events
- **Maintenance**
  ```bash
  # Regular cleanup
  rustify deploy --cleanup
  
  # Update deployments
  rustify deploy --prod --rpl --cleanup
  ```

### Environment Variables

```bash
# Production environment
NODE_ENV=production
PORT=3000
# Development environment
NODE_ENV=development
PORT=8000
```

### Security Best Practices

- **Production Deployments**
  - Always use `--prod` flag
  - Enable security features
  - Use proper namespaces
- **Docker Security**
  - Non-root user
  - Limited permissions
  - Secure defaults
- **Kubernetes Security**
  - Network policies
  - Resource limits
  - Security contexts

### CI/CD Integration

The CLI automatically generates GitHub Actions workflows:

```yaml
name: CI/CD Pipeline
on:
  push:
    branches: [main, develop]
Features:
- Automated testing
- Docker image building
- Kubernetes deployment
- Security scanning
```

This guide covers all the major features and capabilities of the Rustify CLI. For additional help or specific use cases, refer to the documentation or raise an issue on GitHub.