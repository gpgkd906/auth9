/**
 * Keycloak Admin API Helper
 * Used to prepare test data before E2E tests
 */

import { TEST_CONFIG } from "./test-config";
import { execFileSync } from "node:child_process";

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
  private useKcadm = false;

  constructor() {
    this.baseUrl = TEST_CONFIG.keycloakUrl;
    this.realm = TEST_CONFIG.keycloakRealm;
  }

  /**
   * Get admin access token from master realm
   */
  async authenticate(): Promise<void> {
    if (this.useKcadm) {
      this.ensureKcadmSession();
      return;
    }

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
      if (text.includes("HTTPS required")) {
        this.useKcadm = true;
        this.ensureKcadmSession();
        return;
      }
      throw new Error(`Failed to authenticate with Keycloak: ${response.status} ${text}`);
    }

    const token: KeycloakToken = await response.json();
    this.accessToken = token.access_token;
  }

  private ensureKcadmSession(): void {
    execFileSync(
      "docker",
      [
        "exec",
        "auth9-keycloak",
        "/opt/keycloak/bin/kcadm.sh",
        "config",
        "credentials",
        "--server",
        "http://localhost:8080",
        "--realm",
        "master",
        "--user",
        TEST_CONFIG.keycloakAdmin.username,
        "--password",
        TEST_CONFIG.keycloakAdmin.password,
      ],
      { stdio: "pipe" }
    );
  }

  private runKcadm(args: string[]): string {
    this.ensureKcadmSession();
    return execFileSync(
      "docker",
      ["exec", "auth9-keycloak", "/opt/keycloak/bin/kcadm.sh", ...args],
      { encoding: "utf8", stdio: "pipe" }
    );
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
    if (this.useKcadm) {
      const output = this.runKcadm([
        "get",
        "users",
        "-r",
        this.realm,
        "-q",
        `username=${username}`,
        "--fields",
        "id,username,email,firstName,lastName,enabled,emailVerified",
      ]);
      const users = JSON.parse(output) as Array<Partial<KeycloakUser>>;
      if (!users.length) return null;
      const user = users[0];
      return {
        id: user.id,
        username: user.username || username,
        email: user.email || "",
        firstName: user.firstName || "",
        lastName: user.lastName || "",
        enabled: user.enabled ?? true,
        emailVerified: user.emailVerified ?? true,
      };
    }

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

    if (this.useKcadm) {
      this.runKcadm([
        "create",
        "users",
        "-r",
        this.realm,
        "-s",
        `username=${user.username}`,
        "-s",
        `email=${user.email}`,
        "-s",
        `firstName=${user.firstName}`,
        "-s",
        `lastName=${user.lastName}`,
        "-s",
        "enabled=true",
        "-s",
        "emailVerified=true",
      ]);
      this.runKcadm([
        "set-password",
        "-r",
        this.realm,
        "--username",
        user.username,
        "--new-password",
        user.password,
      ]);
    } else {
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
    }
    console.log(`Created user: ${user.username}`);
  }

  /**
   * Delete a user by username
   */
  async deleteUserByUsername(username: string): Promise<void> {
    const user = await this.getUserByUsername(username);
    if (user?.id) {
      if (this.useKcadm) {
        this.runKcadm(["delete", `users/${user.id}`, "-r", this.realm]);
      } else {
        await this.request("DELETE", `/users/${user.id}`);
      }
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
