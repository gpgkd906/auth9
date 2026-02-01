/**
 * Keycloak Admin API Helper
 * Used to prepare test data before E2E tests
 */

import { TEST_CONFIG } from "./test-config";

interface KeycloakToken {
  access_token: string;
  expires_in: number;
  token_type: string;
}

interface KeycloakUser {
  id?: string;
  username: string;
  email: string;
  firstName: string;
  lastName: string;
  enabled: boolean;
  emailVerified: boolean;
  credentials?: Array<{
    type: string;
    value: string;
    temporary: boolean;
  }>;
}

export class KeycloakAdminClient {
  private baseUrl: string;
  private realm: string;
  private accessToken: string | null = null;

  constructor() {
    this.baseUrl = TEST_CONFIG.keycloakUrl;
    this.realm = TEST_CONFIG.keycloakRealm;
  }

  /**
   * Get admin access token from master realm
   */
  async authenticate(): Promise<void> {
    const response = await fetch(
      `${this.baseUrl}/realms/master/protocol/openid-connect/token`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/x-www-form-urlencoded",
        },
        body: new URLSearchParams({
          grant_type: "password",
          client_id: "admin-cli",
          username: TEST_CONFIG.keycloakAdmin.username,
          password: TEST_CONFIG.keycloakAdmin.password,
        }),
      }
    );

    if (!response.ok) {
      const text = await response.text();
      throw new Error(`Failed to authenticate with Keycloak: ${response.status} ${text}`);
    }

    const token: KeycloakToken = await response.json();
    this.accessToken = token.access_token;
  }

  /**
   * Make authenticated request to Keycloak Admin API
   */
  private async request<T>(
    method: string,
    path: string,
    body?: unknown
  ): Promise<T | null> {
    if (!this.accessToken) {
      await this.authenticate();
    }

    const response = await fetch(`${this.baseUrl}/admin/realms/${this.realm}${path}`, {
      method,
      headers: {
        Authorization: `Bearer ${this.accessToken}`,
        "Content-Type": "application/json",
      },
      body: body ? JSON.stringify(body) : undefined,
    });

    if (response.status === 404) {
      return null;
    }

    if (response.status === 409) {
      // Conflict - resource already exists
      return null;
    }

    if (!response.ok) {
      const text = await response.text();
      throw new Error(`Keycloak API error: ${method} ${path} - ${response.status} ${text}`);
    }

    if (response.status === 204 || response.status === 201) {
      return null;
    }

    return response.json();
  }

  /**
   * Get user by username
   */
  async getUserByUsername(username: string): Promise<KeycloakUser | null> {
    const users = await this.request<KeycloakUser[]>(
      "GET",
      `/users?username=${encodeURIComponent(username)}&exact=true`
    );
    return users && users.length > 0 ? users[0] : null;
  }

  /**
   * Create a new user
   */
  async createUser(user: {
    username: string;
    email: string;
    password: string;
    firstName: string;
    lastName: string;
  }): Promise<void> {
    // Check if user already exists
    const existingUser = await this.getUserByUsername(user.username);
    if (existingUser) {
      console.log(`User ${user.username} already exists, skipping creation`);
      return;
    }

    const keycloakUser: KeycloakUser = {
      username: user.username,
      email: user.email,
      firstName: user.firstName,
      lastName: user.lastName,
      enabled: true,
      emailVerified: true,
      credentials: [
        {
          type: "password",
          value: user.password,
          temporary: false,
        },
      ],
    };

    await this.request("POST", "/users", keycloakUser);
    console.log(`Created user: ${user.username}`);
  }

  /**
   * Delete a user by username
   */
  async deleteUserByUsername(username: string): Promise<void> {
    const user = await this.getUserByUsername(username);
    if (user?.id) {
      await this.request("DELETE", `/users/${user.id}`);
      console.log(`Deleted user: ${username}`);
    }
  }

  /**
   * Setup all test users
   */
  async setupTestUsers(): Promise<void> {
    console.log("Setting up test users in Keycloak...");

    for (const [, user] of Object.entries(TEST_CONFIG.testUsers)) {
      await this.createUser(user);
    }

    console.log("Test users setup complete");
  }

  /**
   * Cleanup all test users
   */
  async cleanupTestUsers(): Promise<void> {
    console.log("Cleaning up test users from Keycloak...");

    for (const [, user] of Object.entries(TEST_CONFIG.testUsers)) {
      await this.deleteUserByUsername(user.username);
    }

    console.log("Test users cleanup complete");
  }
}
