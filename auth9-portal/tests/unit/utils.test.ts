import { describe, it, expect } from "vitest";
import { cn, formatDate, formatDateTime, getInitials } from "~/lib/utils";

describe("cn utility", () => {
  it("should merge class names", () => {
    expect(cn("foo", "bar")).toBe("foo bar");
  });

  it("should handle conditional classes", () => {
    const condition = false;
    expect(cn("foo", condition && "bar", "baz")).toBe("foo baz");
  });

  it("should merge tailwind classes correctly", () => {
    expect(cn("px-4", "px-6")).toBe("px-6");
    expect(cn("text-red-500", "text-blue-500")).toBe("text-blue-500");
  });
});

describe("formatDate utility", () => {
  it("should format date string", () => {
    const result = formatDate("2024-01-15T10:30:00Z");
    expect(result).toMatch(/Jan/);
    expect(result).toMatch(/15/);
    expect(result).toMatch(/2024/);
  });

  it("should format Date object", () => {
    const date = new Date("2024-06-20");
    const result = formatDate(date);
    expect(result).toMatch(/Jun/);
    expect(result).toMatch(/20/);
  });
});

describe("formatDateTime utility", () => {
  it("should format date string with time", () => {
    const result = formatDateTime("2024-01-15T10:30:00Z");
    expect(result).toMatch(/Jan/);
    expect(result).toMatch(/15/);
    expect(result).toMatch(/2024/);
    // Should include time component
    expect(result).toMatch(/\d{1,2}:\d{2}/);
  });

  it("should format Date object with time", () => {
    const date = new Date("2024-06-20T14:45:00Z");
    const result = formatDateTime(date);
    expect(result).toMatch(/Jun/);
    expect(result).toMatch(/20/);
    // Should include time component
    expect(result).toMatch(/\d{1,2}:\d{2}/);
  });
});

describe("getInitials utility", () => {
  it("should return initials from full name", () => {
    expect(getInitials("John Doe")).toBe("JD");
  });

  it("should handle single name", () => {
    expect(getInitials("John")).toBe("J");
  });

  it("should handle multiple names", () => {
    expect(getInitials("John Michael Doe")).toBe("JM");
  });

  it("should handle lowercase", () => {
    expect(getInitials("john doe")).toBe("JD");
  });
});

