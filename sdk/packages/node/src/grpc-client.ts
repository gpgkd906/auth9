import * as grpc from "@grpc/grpc-js";
import * as protoLoader from "@grpc/proto-loader";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

export interface GrpcClientConfig {
  /** gRPC server address (host:port) */
  address: string;
  /** Use TLS for the connection (server-side TLS, no client certs) */
  tls?: boolean;
  /** Authentication method */
  auth?:
    | { apiKey: string }
    | { mtls: { cert: Buffer; key: Buffer; ca: Buffer } };
}

export interface ExchangeTokenRequest {
  identityToken: string;
  tenantId: string;
  serviceId: string;
}

export interface ExchangeTokenResponse {
  accessToken: string;
  tokenType: string;
  expiresIn: number;
  refreshToken: string;
}

export interface ValidateTokenRequest {
  accessToken: string;
  audience?: string;
}

export interface ValidateTokenResponse {
  valid: boolean;
  userId: string;
  tenantId: string;
  error?: string;
}

export interface GetUserRolesRequest {
  userId: string;
  tenantId: string;
  serviceId?: string;
}

export interface GetUserRolesResponse {
  roles: Array<{ id: string; name: string; serviceId: string }>;
  permissions: string[];
}

export interface IntrospectTokenRequest {
  token: string;
}

export interface IntrospectTokenResponse {
  active: boolean;
  sub: string;
  email: string;
  tenantId: string;
  roles: string[];
  permissions: string[];
  exp: number;
  iat: number;
  iss: string;
  aud: string;
}

// Proto loader helper types
interface ProtoGrpcType {
  auth9: {
    TokenExchange: grpc.ServiceClientConstructor;
  };
}

type GrpcCallback<T> = (error: grpc.ServiceError | null, response: T) => void;

interface TokenExchangeClient extends grpc.Client {
  ExchangeToken(
    request: Record<string, unknown>,
    metadata: grpc.Metadata,
    callback: GrpcCallback<Record<string, unknown>>,
  ): void;
  ValidateToken(
    request: Record<string, unknown>,
    metadata: grpc.Metadata,
    callback: GrpcCallback<Record<string, unknown>>,
  ): void;
  GetUserRoles(
    request: Record<string, unknown>,
    metadata: grpc.Metadata,
    callback: GrpcCallback<Record<string, unknown>>,
  ): void;
  IntrospectToken(
    request: Record<string, unknown>,
    metadata: grpc.Metadata,
    callback: GrpcCallback<Record<string, unknown>>,
  ): void;
  close(): void;
}

function getProtoPath(): string {
  try {
    const currentFile = fileURLToPath(import.meta.url);
    return join(dirname(currentFile), "..", "proto", "auth9.proto");
  } catch {
    // CJS fallback
    return join(__dirname, "..", "proto", "auth9.proto");
  }
}

export class Auth9GrpcClient {
  private client: TokenExchangeClient;
  private metadata: grpc.Metadata;

  constructor(config: GrpcClientConfig) {
    const protoPath = getProtoPath();
    const packageDefinition = protoLoader.loadSync(protoPath, {
      keepCase: true,
      longs: Number,
      enums: String,
      defaults: true,
      oneofs: true,
    });

    const proto = grpc.loadPackageDefinition(
      packageDefinition,
    ) as unknown as ProtoGrpcType;

    let credentials: grpc.ChannelCredentials;
    if (config.auth && "mtls" in config.auth) {
      credentials = grpc.credentials.createSsl(
        config.auth.mtls.ca,
        config.auth.mtls.key,
        config.auth.mtls.cert,
      );
    } else if (config.tls) {
      credentials = grpc.credentials.createSsl();
    } else {
      credentials = grpc.credentials.createInsecure();
    }

    this.client = new proto.auth9.TokenExchange(
      config.address,
      credentials,
    ) as unknown as TokenExchangeClient;

    this.metadata = new grpc.Metadata();
    if (config.auth && "apiKey" in config.auth) {
      this.metadata.set("x-api-key", config.auth.apiKey);
    }
  }

  exchangeToken(req: ExchangeTokenRequest): Promise<ExchangeTokenResponse> {
    return new Promise((resolve, reject) => {
      this.client.ExchangeToken(
        {
          identity_token: req.identityToken,
          tenant_id: req.tenantId,
          service_id: req.serviceId,
        },
        this.metadata,
        (err, res) => {
          if (err) return reject(err);
          resolve({
            accessToken: res!.access_token as string,
            tokenType: res!.token_type as string,
            expiresIn: res!.expires_in as number,
            refreshToken: res!.refresh_token as string,
          });
        },
      );
    });
  }

  validateToken(req: ValidateTokenRequest): Promise<ValidateTokenResponse> {
    return new Promise((resolve, reject) => {
      this.client.ValidateToken(
        {
          access_token: req.accessToken,
          audience: req.audience ?? "",
        },
        this.metadata,
        (err, res) => {
          if (err) return reject(err);
          resolve({
            valid: res!.valid as boolean,
            userId: res!.user_id as string,
            tenantId: res!.tenant_id as string,
            error: (res!.error as string) || undefined,
          });
        },
      );
    });
  }

  getUserRoles(req: GetUserRolesRequest): Promise<GetUserRolesResponse> {
    return new Promise((resolve, reject) => {
      this.client.GetUserRoles(
        {
          user_id: req.userId,
          tenant_id: req.tenantId,
          service_id: req.serviceId ?? "",
        },
        this.metadata,
        (err, res) => {
          if (err) return reject(err);
          const roles = ((res!.roles as Array<Record<string, unknown>>) ?? []).map(
            (r) => ({
              id: r.id as string,
              name: r.name as string,
              serviceId: r.service_id as string,
            }),
          );
          resolve({
            roles,
            permissions: (res!.permissions as string[]) ?? [],
          });
        },
      );
    });
  }

  introspectToken(
    req: IntrospectTokenRequest,
  ): Promise<IntrospectTokenResponse> {
    return new Promise((resolve, reject) => {
      this.client.IntrospectToken(
        { token: req.token },
        this.metadata,
        (err, res) => {
          if (err) return reject(err);
          resolve({
            active: res!.active as boolean,
            sub: res!.sub as string,
            email: res!.email as string,
            tenantId: res!.tenant_id as string,
            roles: (res!.roles as string[]) ?? [],
            permissions: (res!.permissions as string[]) ?? [],
            exp: res!.exp as number,
            iat: res!.iat as number,
            iss: res!.iss as string,
            aud: res!.aud as string,
          });
        },
      );
    });
  }

  close(): void {
    this.client.close();
  }
}
