# Auth9 Portal - Remix Frontend
# Multi-stage build for minimal image size

# ==================== Builder Stage ====================
FROM node:20-alpine AS builder

WORKDIR /app

# Copy package files
COPY package.json package-lock.json ./

# Install dependencies
RUN npm ci

# Copy source code
COPY . .

# Build the application
RUN npm run build

# ==================== Runtime Stage ====================
FROM node:20-alpine

# Create non-root user
RUN addgroup -g 1001 -S nodejs && \
    adduser -S remix -u 1001

WORKDIR /app

# Copy built assets from builder
COPY --from=builder --chown=remix:nodejs /app/build ./build
COPY --from=builder --chown=remix:nodejs /app/public ./public
COPY --from=builder --chown=remix:nodejs /app/package.json ./package.json
COPY --from=builder --chown=remix:nodejs /app/package-lock.json ./package-lock.json

# Install production dependencies only
RUN npm ci --omit=dev

# Switch to non-root user
USER remix

# Environment
ENV NODE_ENV=production
ENV PORT=3000

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:3000/ || exit 1

# Run
CMD ["npm", "start"]
