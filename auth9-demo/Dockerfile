# Auth9 Demo - SDK Integration Example
# Multi-stage build for minimal image size
# Build context: repo root (for file: dependency on SDK packages)

# ==================== SDK Core Build Stage ====================
FROM node:20-alpine AS sdk-core-builder

WORKDIR /app/sdk

# Copy base tsconfig (needed by all SDK packages)
COPY sdk/tsconfig.base.json ./

WORKDIR /app/sdk/packages/core

# Install SDK Core dependencies and build
COPY sdk/packages/core/package.json ./
RUN --mount=type=cache,target=/root/.npm \
    npm install
COPY sdk/packages/core/ ./
RUN npm run build

# ==================== SDK Node Build Stage ====================
FROM node:20-alpine AS sdk-node-builder

WORKDIR /app/sdk

# Copy base tsconfig (needed for extends "../../tsconfig.base.json")
COPY sdk/tsconfig.base.json ./

# Copy built @auth9/core (needed as dependency for @auth9/node)
COPY --from=sdk-core-builder /app/sdk/packages/core/ ./packages/core/

# Install SDK Node dependencies and build
WORKDIR /app/sdk/packages/node
COPY sdk/packages/node/package.json ./
# Replace workspace:* with file: reference for Docker build
RUN sed -i 's|"@auth9/core": "workspace:\*"|"@auth9/core": "file:../core"|' package.json
RUN --mount=type=cache,target=/root/.npm \
    npm install
COPY sdk/packages/node/src/ ./src/
COPY sdk/packages/node/proto/ ./proto/
COPY sdk/packages/node/tsconfig.json sdk/packages/node/tsup.config.ts ./
RUN npm run build

# ==================== Demo Builder Stage ====================
FROM node:20-alpine AS builder

WORKDIR /app

# Copy built SDK packages
COPY --from=sdk-core-builder /app/sdk/packages/core/package.json sdk/packages/core/
COPY --from=sdk-core-builder /app/sdk/packages/core/dist sdk/packages/core/dist

COPY --from=sdk-node-builder /app/sdk/packages/node/package.json sdk/packages/node/
COPY --from=sdk-node-builder /app/sdk/packages/node/dist sdk/packages/node/dist
COPY --from=sdk-node-builder /app/sdk/packages/node/proto sdk/packages/node/proto

# Copy demo package files and install
COPY auth9-demo/package.json auth9-demo/tsconfig.json auth9-demo/
WORKDIR /app/auth9-demo
RUN --mount=type=cache,target=/root/.npm \
    npm install

# Copy demo source and build
COPY auth9-demo/src/ ./src/
RUN npm run build

# ==================== Runtime Stage ====================
FROM node:20-alpine

WORKDIR /app

# Copy built SDK packages with their runtime dependencies
COPY --from=sdk-core-builder /app/sdk/packages/core/package.json sdk/packages/core/
COPY --from=sdk-core-builder /app/sdk/packages/core/dist sdk/packages/core/dist
COPY --from=sdk-core-builder /app/sdk/packages/core/node_modules sdk/packages/core/node_modules

COPY --from=sdk-node-builder /app/sdk/packages/node/package.json sdk/packages/node/
COPY --from=sdk-node-builder /app/sdk/packages/node/dist sdk/packages/node/dist
COPY --from=sdk-node-builder /app/sdk/packages/node/proto sdk/packages/node/proto
COPY --from=sdk-node-builder /app/sdk/packages/node/node_modules sdk/packages/node/node_modules

# Copy demo app
COPY --from=builder /app/auth9-demo/package.json ./auth9-demo/
COPY --from=builder /app/auth9-demo/dist/ ./auth9-demo/dist/
COPY --from=builder /app/auth9-demo/node_modules/ ./auth9-demo/node_modules/
COPY auth9-demo/src/views/ ./auth9-demo/src/views/

WORKDIR /app/auth9-demo

ENV NODE_ENV=production
ENV PORT=3002

EXPOSE 3002

CMD ["node", "dist/server.js"]
