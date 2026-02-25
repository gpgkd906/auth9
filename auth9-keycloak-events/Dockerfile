# Build stage: Build the keycloak-events SPI from p2-inc/keycloak-events
FROM maven:3.9-eclipse-temurin-21-alpine AS builder

WORKDIR /build

# Clone and build the keycloak-events project
# Using v0.51 which is compatible with Keycloak 26.x (targets 26.5.0)
RUN apk add --no-cache git && \
    git clone --depth 1 --branch v0.51 https://github.com/p2-inc/keycloak-events.git .
RUN --mount=type=cache,target=/root/.m2/repository \
    mvn clean package -DskipTests

# Output stage: Extract the built JAR
FROM alpine:3.21 AS output

WORKDIR /events

# Copy the shaded events JAR from builder (exclude original-*.jar to avoid duplicate classes)
COPY --from=builder /build/target/*.jar ./
RUN rm -f /events/original-*.jar

# Default command: remove old events JARs, then copy new one to mounted volume
CMD ["sh", "-c", "rm -f /events-output/keycloak-events-*.jar && cp /events/*.jar /events-output/ && echo 'Keycloak Events SPI JAR copied to /events-output/' && ls -la /events-output/"]
