import { describe, it, expect } from "vitest";
import { toSnakeCase, toCamelCase } from "./utils.js";

describe("toSnakeCase", () => {
  it("converts simple object keys", () => {
    const input = { firstName: "John", lastName: "Doe" };
    expect(toSnakeCase(input)).toEqual({
      first_name: "John",
      last_name: "Doe",
    });
  });

  it("converts nested objects", () => {
    const input = {
      tenantId: "123",
      userInfo: { displayName: "John", avatarUrl: "http://example.com" },
    };
    expect(toSnakeCase(input)).toEqual({
      tenant_id: "123",
      user_info: {
        display_name: "John",
        avatar_url: "http://example.com",
      },
    });
  });

  it("converts arrays of objects", () => {
    const input = [
      { roleId: "1", roleName: "admin" },
      { roleId: "2", roleName: "user" },
    ];
    expect(toSnakeCase(input)).toEqual([
      { role_id: "1", role_name: "admin" },
      { role_id: "2", role_name: "user" },
    ]);
  });

  it("handles null and undefined", () => {
    expect(toSnakeCase(null)).toBeNull();
    expect(toSnakeCase(undefined)).toBeUndefined();
  });

  it("passes through primitives", () => {
    expect(toSnakeCase("hello")).toBe("hello");
    expect(toSnakeCase(42)).toBe(42);
    expect(toSnakeCase(true)).toBe(true);
  });

  it("handles empty objects and arrays", () => {
    expect(toSnakeCase({})).toEqual({});
    expect(toSnakeCase([])).toEqual([]);
  });
});

describe("toCamelCase", () => {
  it("converts simple object keys", () => {
    const input = { first_name: "John", last_name: "Doe" };
    expect(toCamelCase(input)).toEqual({
      firstName: "John",
      lastName: "Doe",
    });
  });

  it("converts nested objects", () => {
    const input = {
      tenant_id: "123",
      user_info: { display_name: "John", avatar_url: "http://example.com" },
    };
    expect(toCamelCase(input)).toEqual({
      tenantId: "123",
      userInfo: {
        displayName: "John",
        avatarUrl: "http://example.com",
      },
    });
  });

  it("converts arrays of objects", () => {
    const input = [
      { role_id: "1", role_name: "admin" },
      { role_id: "2", role_name: "user" },
    ];
    expect(toCamelCase(input)).toEqual([
      { roleId: "1", roleName: "admin" },
      { roleId: "2", roleName: "user" },
    ]);
  });

  it("handles null and undefined", () => {
    expect(toCamelCase(null)).toBeNull();
    expect(toCamelCase(undefined)).toBeUndefined();
  });

  it("passes through primitives", () => {
    expect(toCamelCase("hello")).toBe("hello");
    expect(toCamelCase(42)).toBe(42);
  });

  it("roundtrips with toSnakeCase", () => {
    const original = {
      tenantId: "123",
      mfaEnabled: true,
      createdAt: "2024-01-01",
    };
    expect(toCamelCase(toSnakeCase(original))).toEqual(original);
  });
});
