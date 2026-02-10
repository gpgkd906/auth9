import { describe, it, expect, vi, beforeEach } from "vitest";
import { Auth9HttpClient } from "./http-client.js";
import { NotFoundError, UnauthorizedError } from "./errors.js";

// Mock global fetch
const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("Auth9HttpClient", () => {
  const client = new Auth9HttpClient({
    baseUrl: "https://auth9.example.com",
    accessToken: "test-token",
  });

  it("makes GET requests with auth header", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      json: () =>
        Promise.resolve({ data: { id: "1", tenant_id: "t1", display_name: "Test" } }),
    });

    const result = await client.get<{ data: { id: string; tenantId: string; displayName: string } }>(
      "/api/v1/tenants/1",
    );

    expect(mockFetch).toHaveBeenCalledWith(
      "https://auth9.example.com/api/v1/tenants/1",
      expect.objectContaining({
        method: "GET",
        headers: expect.objectContaining({
          Authorization: "Bearer test-token",
        }),
      }),
    );
    // Response keys should be converted to camelCase
    expect(result.data.tenantId).toBe("t1");
    expect(result.data.displayName).toBe("Test");
  });

  it("makes POST requests with body converted to snake_case", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve({ data: { id: "1" } }),
    });

    await client.post("/api/v1/tenants", {
      tenantName: "My Tenant",
      logoUrl: "http://example.com",
    });

    const [, init] = mockFetch.mock.calls[0];
    const body = JSON.parse(init.body);
    expect(body.tenant_name).toBe("My Tenant");
    expect(body.logo_url).toBe("http://example.com");
  });

  it("throws NotFoundError on 404", async () => {
    mockFetch.mockResolvedValue({
      ok: false,
      status: 404,
      json: () =>
        Promise.resolve({ error: "not_found", message: "Tenant not found" }),
    });

    await expect(client.get("/api/v1/tenants/xxx")).rejects.toThrow(
      NotFoundError,
    );
  });

  it("throws UnauthorizedError on 401", async () => {
    mockFetch.mockResolvedValue({
      ok: false,
      status: 401,
      json: () =>
        Promise.resolve({ error: "unauthorized", message: "Invalid token" }),
    });

    await expect(client.get("/api/v1/tenants")).rejects.toThrow(
      UnauthorizedError,
    );
  });

  it("handles 204 No Content", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      status: 204,
    });

    const result = await client.delete("/api/v1/tenants/1");
    expect(result).toBeUndefined();
  });

  it("supports async token provider", async () => {
    const tokenFn = vi.fn().mockResolvedValue("dynamic-token");
    const dynamicClient = new Auth9HttpClient({
      baseUrl: "https://auth9.example.com",
      accessToken: tokenFn,
    });

    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve({ data: {} }),
    });

    await dynamicClient.get("/api/v1/tenants");

    expect(tokenFn).toHaveBeenCalled();
    const [, init] = mockFetch.mock.calls[0];
    expect(init.headers.Authorization).toBe("Bearer dynamic-token");
  });

  it("includes query params in GET requests", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve({ data: [] }),
    });

    await client.get("/api/v1/tenants", { page: "1", per_page: "20" });

    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining("page=1"),
      expect.anything(),
    );
  });

  it("strips trailing slashes from baseUrl", async () => {
    const slashClient = new Auth9HttpClient({
      baseUrl: "https://auth9.example.com///",
    });

    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve({}),
    });

    await slashClient.get("/api/v1/health");

    expect(mockFetch).toHaveBeenCalledWith(
      "https://auth9.example.com/api/v1/health",
      expect.anything(),
    );
  });
});
