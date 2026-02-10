import { describe, it, expect, vi, beforeEach } from "vitest";
import { ClientCredentials } from "./client-credentials.js";

const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

beforeEach(() => {
  vi.clearAllMocks();
});

function createCreds() {
  return new ClientCredentials({
    domain: "https://auth9.example.com",
    clientId: "my-client",
    clientSecret: "my-secret",
  });
}

describe("ClientCredentials", () => {
  it("fetches a new token on first call", async () => {
    const creds = createCreds();
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      json: () =>
        Promise.resolve({
          access_token: "new-token",
          token_type: "Bearer",
          expires_in: 3600,
        }),
    });

    const token = await creds.getToken();
    expect(token).toBe("new-token");
    expect(mockFetch).toHaveBeenCalledTimes(1);

    // Verify request body was sent with snake_case
    const [, init] = mockFetch.mock.calls[0];
    const body = JSON.parse(init.body);
    expect(body.grant_type).toBe("client_credentials");
    expect(body.client_id).toBe("my-client");
    expect(body.client_secret).toBe("my-secret");
  });

  it("returns cached token on subsequent calls", async () => {
    const creds = createCreds();
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      json: () =>
        Promise.resolve({
          access_token: "cached-token",
          token_type: "Bearer",
          expires_in: 3600,
        }),
    });

    await creds.getToken();
    const token2 = await creds.getToken();

    // Should only make one network call
    expect(mockFetch).toHaveBeenCalledTimes(1);
    expect(token2).toBe("cached-token");
  });

  it("clears cache and re-fetches", async () => {
    const creds = createCreds();
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: () =>
          Promise.resolve({
            access_token: "first-token",
            token_type: "Bearer",
            expires_in: 3600,
          }),
      })
      .mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: () =>
          Promise.resolve({
            access_token: "second-token",
            token_type: "Bearer",
            expires_in: 3600,
          }),
      });

    const token1 = await creds.getToken();
    expect(token1).toBe("first-token");

    creds.clearCache();

    const token2 = await creds.getToken();
    expect(token2).toBe("second-token");
    expect(mockFetch).toHaveBeenCalledTimes(2);
  });
});
