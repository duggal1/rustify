
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
CMD ["bun", "run", "start"]