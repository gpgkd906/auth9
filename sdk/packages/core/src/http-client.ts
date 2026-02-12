import { createErrorFromStatus } from "./errors.js";
import { toCamelCase, toSnakeCase } from "./utils.js";

export interface HttpClientConfig {
  /** Auth9 Core base URL (e.g., "https://auth9.example.com") */
  baseUrl: string;
  /** Access token or async function returning an access token */
  accessToken?: string | (() => string | Promise<string>);
  /** Request timeout in milliseconds (default: 10000) */
  timeout?: number;
  /** Number of retries on 5xx errors (default: 0) */
  retries?: number;
}

export class Auth9HttpClient {
  private baseUrl: string;
  private accessToken?: string | (() => string | Promise<string>);
  private timeout: number;
  private retries: number;

  constructor(config: HttpClientConfig) {
    this.baseUrl = config.baseUrl.replace(/\/+$/, "");
    this.accessToken = config.accessToken;
    this.timeout = config.timeout ?? 10_000;
    this.retries = config.retries ?? 0;
  }

  private async getToken(): Promise<string | undefined> {
    if (!this.accessToken) return undefined;
    if (typeof this.accessToken === "function") {
      return this.accessToken();
    }
    return this.accessToken;
  }

  private async request<T>(
    method: string,
    path: string,
    body?: unknown,
    params?: Record<string, string>,
  ): Promise<T> {
    let url = `${this.baseUrl}${path}`;
    if (params) {
      const searchParams = new URLSearchParams(params);
      url += `?${searchParams.toString()}`;
    }

    const token = await this.getToken();
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };
    if (token) {
      headers["Authorization"] = `Bearer ${token}`;
    }

    const init: RequestInit = { method, headers };
    if (body !== undefined) {
      init.body = JSON.stringify(toSnakeCase(body));
    }

    let lastError: Error | undefined;
    for (let attempt = 0; attempt <= this.retries; attempt++) {
      try {
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), this.timeout);
        init.signal = controller.signal;

        const response = await fetch(url, init);
        clearTimeout(timeoutId);

        if (!response.ok) {
          const errorBody = await response.json().catch(() => ({
            error: "unknown",
            message: response.statusText,
          }));
          const error = createErrorFromStatus(response.status, errorBody);

          // Only retry on 5xx errors
          if (response.status >= 500 && attempt < this.retries) {
            lastError = error;
            continue;
          }
          throw error;
        }

        // Handle 204 No Content
        if (response.status === 204) {
          return undefined as T;
        }

        const json = await response.json();
        return toCamelCase(json) as T;
      } catch (err) {
        if (err instanceof Error && err.name === "AbortError") {
          lastError = new Error(`Request timeout after ${this.timeout}ms`);
          if (attempt < this.retries) continue;
        }
        if (lastError && attempt === this.retries) throw lastError;
        throw err;
      }
    }

    throw lastError ?? new Error("Request failed");
  }

  async get<T>(path: string, params?: Record<string, string>): Promise<T> {
    return this.request<T>("GET", path, undefined, params);
  }

  async post<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>("POST", path, body);
  }

  async put<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>("PUT", path, body);
  }

  async patch<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>("PATCH", path, body);
  }

  async delete(path: string): Promise<void> {
    await this.request<void>("DELETE", path);
  }
}
