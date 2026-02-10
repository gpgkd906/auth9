import { Auth9HttpClient, UnauthorizedError } from "@auth9/core";

export interface ClientCredentialsConfig {
  /** Auth9 Core URL */
  domain: string;
  /** Service client ID */
  clientId: string;
  /** Service client secret */
  clientSecret: string;
}

interface TokenResponse {
  accessToken: string;
  tokenType: string;
  expiresIn: number;
}

export class ClientCredentials {
  private httpClient: Auth9HttpClient;
  private clientId: string;
  private clientSecret: string;
  private cachedToken: string | null = null;
  private expiresAt = 0;

  constructor(config: ClientCredentialsConfig) {
    this.httpClient = new Auth9HttpClient({
      baseUrl: config.domain,
    });
    this.clientId = config.clientId;
    this.clientSecret = config.clientSecret;
  }

  /** Get a valid service token, refreshing if necessary */
  async getToken(): Promise<string> {
    // Return cached token if still valid (with 30s buffer)
    if (this.cachedToken && Date.now() / 1000 < this.expiresAt - 30) {
      return this.cachedToken;
    }

    const response = await this.httpClient.post<TokenResponse>(
      "/api/v1/auth/token",
      {
        grantType: "client_credentials",
        clientId: this.clientId,
        clientSecret: this.clientSecret,
      },
    );

    if (!response.accessToken) {
      throw new UnauthorizedError("Failed to obtain service token");
    }

    this.cachedToken = response.accessToken;
    this.expiresAt = Date.now() / 1000 + response.expiresIn;

    return this.cachedToken;
  }

  /** Invalidate the cached token */
  clearCache(): void {
    this.cachedToken = null;
    this.expiresAt = 0;
  }
}
