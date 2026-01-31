# Build stage: Build the Keycloakify theme
FROM node:20-alpine AS builder

# Install Maven for keycloakify build
RUN apk add --no-cache openjdk17 maven

WORKDIR /app

# Install dependencies first (for better caching)
COPY package*.json ./
RUN npm ci

# Copy source files
COPY . .

# Generate kc.gen files (required before TypeScript compilation)
RUN npx keycloakify update-kc-gen

# Build the theme JAR
RUN npm run build && npx keycloakify build

# Output stage: Extract the built JAR
FROM alpine:3.19 AS output

WORKDIR /theme

# Copy the built theme JAR from builder
COPY --from=builder /app/dist_keycloak/*.jar ./

# Default command: copy JAR to mounted volume
CMD ["sh", "-c", "cp /theme/*.jar /theme-output/ 2>/dev/null && echo 'Auth9 theme JAR copied to /theme-output/' && ls -la /theme-output/"]
