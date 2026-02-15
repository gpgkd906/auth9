---
name: auth9-grpc-regression
description: Run Auth9 gRPC regression tests by issuing grpcurl requests from inside the Docker Compose network (required when host-to-container ports are blocked), especially for the auth9-grpc-tls TLS endpoint.
---

# Auth9 gRPC Regression Skill

When `auth9-grpc-tls` is introduced (TLS terminator for gRPC), host -> container gRPC access may be blocked or flaky.
Always run `grpcurl` from an ephemeral Docker container on the same compose network.

## Quick Start

```bash
# Smoke checks for auth9-grpc-tls:50051
.claude/skills/tools/grpc-smoke.sh
```

## One-off grpcurl (via Docker network)

```bash
# List services is expected to FAIL (reflection disabled)
.claude/skills/tools/grpcurl-docker.sh \
  -cacert /certs/ca.crt -cert /certs/client.crt -key /certs/client.key \
  auth9-grpc-tls:50051 list

# Call ExchangeToken without API key is expected to FAIL (Missing API key)
.claude/skills/tools/grpcurl-docker.sh \
  -cacert /certs/ca.crt -cert /certs/client.crt -key /certs/client.key \
  -import-path /proto -proto auth9.proto \
  -d '{"identity_token":"dummy","tenant_id":"dummy","service_id":"dummy"}' \
  auth9-grpc-tls:50051 auth9.TokenExchange/ExchangeToken

# Call with API key should get past "Missing API key" (may still fail due to invalid token)
.claude/skills/tools/grpcurl-docker.sh \
  -cacert /certs/ca.crt -cert /certs/client.crt -key /certs/client.key \
  -H "x-api-key: dev-grpc-api-key" \
  -import-path /proto -proto auth9.proto \
  -d '{"identity_token":"dummy","tenant_id":"dummy","service_id":"dummy"}' \
  auth9-grpc-tls:50051 auth9.TokenExchange/ExchangeToken
```

## Environment Variables

- `GRPC_NETWORK`: Docker network name. If unset, auto-detect `*_auth9-network`, fallback to `auth9_auth9-network`.
- `GRPC_TARGET`: default `auth9-grpc-tls:50051`
- `GRPC_API_KEY`: default `dev-grpc-api-key`
- `GRPC_IMPORT_PATH_HOST`: default `auth9-core/proto` (mounted to `/proto`)
- `GRPC_PROTO`: default `auth9.proto`
