import { beforeEach, describe, expect, it, vi } from "vitest";

const {
  mockLoadSync,
  mockCreateSsl,
  mockCreateInsecure,
  mockLoadPackageDefinition,
  MockMetadata,
  mockClientMethods,
  mockTokenExchangeCtor,
} = vi.hoisted(() => {
  const mockLoadSync = vi.fn();
  const mockCreateSsl = vi.fn();
  const mockCreateInsecure = vi.fn();
  const mockLoadPackageDefinition = vi.fn();

  class MockMetadata {
    private values = new Map<string, string>();

    set(key: string, value: string): void {
      this.values.set(key, value);
    }

    get(key: string): string | undefined {
      return this.values.get(key);
    }
  }

  const mockClientMethods = {
    ExchangeToken: vi.fn(),
    ValidateToken: vi.fn(),
    GetUserRoles: vi.fn(),
    IntrospectToken: vi.fn(),
    close: vi.fn(),
  };

  const mockTokenExchangeCtor = vi.fn(function mockTokenExchangeCtor(
    this: Record<string, unknown>,
  ) {
    Object.assign(this, mockClientMethods);
  });

  return {
    mockLoadSync,
    mockCreateSsl,
    mockCreateInsecure,
    mockLoadPackageDefinition,
    MockMetadata,
    mockClientMethods,
    mockTokenExchangeCtor,
  };
});

vi.mock("@grpc/proto-loader", () => ({
  default: {
    loadSync: mockLoadSync,
  },
  loadSync: mockLoadSync,
}));

vi.mock("@grpc/grpc-js", () => ({
  Metadata: MockMetadata,
  credentials: {
    createSsl: mockCreateSsl,
    createInsecure: mockCreateInsecure,
  },
  loadPackageDefinition: mockLoadPackageDefinition,
}));

import { Auth9GrpcClient } from "./grpc-client.js";

describe("Auth9GrpcClient", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockCreateSsl.mockImplementation((ca?: Buffer, key?: Buffer, cert?: Buffer) => ({
      type: "ssl",
      ca,
      key,
      cert,
    }));
    mockCreateInsecure.mockReturnValue({ type: "insecure" });
    mockLoadSync.mockReturnValue({});
    mockLoadPackageDefinition.mockReturnValue({
      auth9: {
        TokenExchange: mockTokenExchangeCtor,
      },
    });
  });

  it("uses insecure credentials by default and sets api key metadata", async () => {
    mockClientMethods.ExchangeToken.mockImplementation((_req, _md, cb) =>
      cb(null, {
        access_token: "access-1",
        token_type: "Bearer",
        expires_in: 3600,
        refresh_token: "refresh-1",
      }),
    );

    const client = new Auth9GrpcClient({
      address: "localhost:50051",
      auth: { apiKey: "test-api-key" },
    });

    const result = await client.exchangeToken({
      identityToken: "id-token",
      tenantId: "tenant-1",
      serviceId: "svc-1",
    });

    expect(mockCreateInsecure).toHaveBeenCalledTimes(1);
    expect(mockCreateSsl).not.toHaveBeenCalled();
    expect(mockLoadSync).toHaveBeenCalledWith(
      expect.stringContaining("proto/auth9.proto"),
      expect.objectContaining({ keepCase: true }),
    );
    expect(mockTokenExchangeCtor).toHaveBeenCalledWith(
      "localhost:50051",
      expect.objectContaining({ type: "insecure" }),
    );

    const [request, metadata] = mockClientMethods.ExchangeToken.mock.calls[0];
    expect(request).toEqual({
      identity_token: "id-token",
      tenant_id: "tenant-1",
      service_id: "svc-1",
    });
    expect((metadata as MockMetadata).get("x-api-key")).toBe("test-api-key");
    expect(result).toEqual({
      accessToken: "access-1",
      tokenType: "Bearer",
      expiresIn: 3600,
      refreshToken: "refresh-1",
    });
  });

  it("uses TLS credentials when tls=true", () => {
    new Auth9GrpcClient({
      address: "localhost:50052",
      tls: true,
    });

    expect(mockCreateSsl).toHaveBeenCalledWith();
    expect(mockCreateInsecure).not.toHaveBeenCalled();
  });

  it("uses mTLS credentials when cert/key/ca are provided", () => {
    const cert = Buffer.from("cert");
    const key = Buffer.from("key");
    const ca = Buffer.from("ca");

    new Auth9GrpcClient({
      address: "localhost:50053",
      auth: { mtls: { cert, key, ca } },
    });

    expect(mockCreateSsl).toHaveBeenCalledWith(ca, key, cert);
    expect(mockCreateInsecure).not.toHaveBeenCalled();
  });

  it("maps validateToken request/response with optional audience", async () => {
    mockClientMethods.ValidateToken.mockImplementation((_req, _md, cb) =>
      cb(null, {
        valid: true,
        user_id: "user-1",
        tenant_id: "tenant-1",
        error: "",
      }),
    );

    const client = new Auth9GrpcClient({ address: "localhost:50054" });
    const response = await client.validateToken({ accessToken: "at-1" });

    expect(mockClientMethods.ValidateToken).toHaveBeenCalledWith(
      { access_token: "at-1", audience: "" },
      expect.any(MockMetadata),
      expect.any(Function),
    );
    expect(response).toEqual({
      valid: true,
      userId: "user-1",
      tenantId: "tenant-1",
      error: undefined,
    });
  });

  it("maps getUserRoles request/response and defaults", async () => {
    mockClientMethods.GetUserRoles.mockImplementation((_req, _md, cb) =>
      cb(null, {
        roles: [{ id: "r1", name: "admin", service_id: "svc-1" }],
      }),
    );

    const client = new Auth9GrpcClient({ address: "localhost:50055" });
    const response = await client.getUserRoles({
      userId: "user-1",
      tenantId: "tenant-1",
    });

    expect(mockClientMethods.GetUserRoles).toHaveBeenCalledWith(
      { user_id: "user-1", tenant_id: "tenant-1", service_id: "" },
      expect.any(MockMetadata),
      expect.any(Function),
    );
    expect(response).toEqual({
      roles: [{ id: "r1", name: "admin", serviceId: "svc-1" }],
      permissions: [],
    });
  });

  it("maps introspectToken response and closes client", async () => {
    mockClientMethods.IntrospectToken.mockImplementation((_req, _md, cb) =>
      cb(null, {
        active: true,
        sub: "user-1",
        email: "u@example.com",
        tenant_id: "tenant-1",
        roles: ["admin"],
        permissions: ["user:read"],
        exp: 200,
        iat: 100,
        iss: "https://issuer",
        aud: "service-a",
      }),
    );

    const client = new Auth9GrpcClient({ address: "localhost:50056" });
    const response = await client.introspectToken({ token: "t-1" });
    client.close();

    expect(mockClientMethods.IntrospectToken).toHaveBeenCalledWith(
      { token: "t-1" },
      expect.any(MockMetadata),
      expect.any(Function),
    );
    expect(response).toEqual({
      active: true,
      sub: "user-1",
      email: "u@example.com",
      tenantId: "tenant-1",
      roles: ["admin"],
      permissions: ["user:read"],
      exp: 200,
      iat: 100,
      iss: "https://issuer",
      aud: "service-a",
    });
    expect(mockClientMethods.close).toHaveBeenCalledTimes(1);
  });

  it("rejects promise when grpc methods return errors", async () => {
    const grpcError = new Error("grpc failed");
    mockClientMethods.ExchangeToken.mockImplementation((_req, _md, cb) =>
      cb(grpcError, null),
    );

    const client = new Auth9GrpcClient({ address: "localhost:50057" });
    await expect(
      client.exchangeToken({
        identityToken: "id-token",
        tenantId: "tenant-1",
        serviceId: "svc-1",
      }),
    ).rejects.toBe(grpcError);
  });
});
