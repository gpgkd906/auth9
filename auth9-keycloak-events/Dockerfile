# Build stage: Build the keycloak-events SPI from p2-inc/keycloak-events
FROM maven:3.9-eclipse-temurin-17-alpine AS builder

WORKDIR /build

# Clone and build the keycloak-events project
# Using v0.47 which is compatible with Keycloak 23.x
RUN apk add --no-cache git && \
    git clone --depth 1 --branch v0.47 https://github.com/p2-inc/keycloak-events.git . && \
    mvn clean package -DskipTests -pl jar -am

# Output stage: Extract the built JAR
FROM alpine:3.19 AS output

WORKDIR /events

# Copy the built events JAR from builder
COPY --from=builder /build/jar/target/*.jar ./

# Default command: copy JAR to mounted volume
CMD ["sh", "-c", "cp /events/*.jar /events-output/ 2>/dev/null && echo 'Keycloak Events SPI JAR copied to /events-output/' && ls -la /events-output/"]
