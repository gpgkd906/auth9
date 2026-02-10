import type { Auth9Claims } from "@auth9/core";
import { TokenVerifier, type TokenVerifierConfig } from "./token-verifier.js";
import {
  Auth9GrpcClient,
  type GrpcClientConfig,
} from "./grpc-client.js";
import {
  ClientCredentials,
  type ClientCredentialsConfig,
} from "./client-credentials.js";

export interface Auth9Config {
  /** Auth9 Core URL (e.g., "https://auth9.example.com") */
  domain: string;
  /** Expected audience for token verification (service client_id) */
  audience?: string;
  /** Service client ID (for M2M authentication) */
  clientId?: string;
  /** Service client secret (for M2M authentication) */
  clientSecret?: string;
  /** JWKS cache TTL in seconds (default: 3600) */
  jwksCacheTtl?: number;
}

/**
 * Main Auth9 SDK entry point for Node.js applications.
 *
 * @example
 * ```ts
 * const auth9 = new Auth9({
 *   domain: "https://auth9.example.com",
 *   audience: "my-service-client-id",
 * });
 *
 * // Verify a token
 * const claims = await auth9.verifyToken(token);
 *
 * // Use gRPC for token exchange
 * const grpc = auth9.grpc({ address: "localhost:50051" });
 * const result = await grpc.exchangeToken({ identityToken, tenantId, serviceId });
 * ```
 */
export class Auth9 {
  private verifier: TokenVerifier;
  private credentials: ClientCredentials | null = null;
  private config: Auth9Config;

  constructor(config: Auth9Config) {
    this.config = config;
    this.verifier = new TokenVerifier({
      domain: config.domain,
      audience: config.audience,
      jwksCacheTtl: config.jwksCacheTtl,
    });

    if (config.clientId && config.clientSecret) {
      this.credentials = new ClientCredentials({
        domain: config.domain,
        clientId: config.clientId,
        clientSecret: config.clientSecret,
      });
    }
  }

  /** Verify a JWT token and return typed claims */
  async verifyToken(token: string): Promise<Auth9Claims> {
    const { claims } = await this.verifier.verify(token);
    return claims;
  }

  /** Create a gRPC client for Token Exchange operations */
  grpc(config: GrpcClientConfig): Auth9GrpcClient {
    return new Auth9GrpcClient(config);
  }

  /**
   * Get a service token via client_credentials grant.
   * Requires clientId and clientSecret in the Auth9 config.
   */
  async getServiceToken(): Promise<string> {
    if (!this.credentials) {
      throw new Error(
        "Client credentials not configured. Provide clientId and clientSecret in Auth9Config.",
      );
    }
    return this.credentials.getToken();
  }
}

// Re-export everything from @auth9/core
export * from "@auth9/core";

// Re-export node-specific modules
export { TokenVerifier } from "./token-verifier.js";
export type { TokenVerifierConfig, VerifyResult } from "./token-verifier.js";
export { Auth9GrpcClient } from "./grpc-client.js";
export type {
  GrpcClientConfig,
  ExchangeTokenRequest,
  ExchangeTokenResponse,
  ValidateTokenRequest,
  ValidateTokenResponse,
  GetUserRolesRequest,
  GetUserRolesResponse,
  IntrospectTokenRequest,
  IntrospectTokenResponse,
} from "./grpc-client.js";
export { ClientCredentials } from "./client-credentials.js";
export type { ClientCredentialsConfig } from "./client-credentials.js";
