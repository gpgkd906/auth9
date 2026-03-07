import { describe, it, expect, vi } from "vitest";

vi.mock("react-router", () => ({
  createCookie: vi.fn(() => ({
    parse: vi.fn(),
    serialize: vi.fn(),
  })),
}));

import { resolveLocale } from "~/services/locale.server";

function makeRequest(headers: Record<string, string> = {}): Request {
  const req = new Request("http://localhost/test");
  const headerMap = new Map(
    Object.entries(headers).map(([k, v]) => [k.toLowerCase(), v])
  );
  vi.spyOn(req.headers, "get").mockImplementation(
    (name) => headerMap.get(name.toLowerCase()) ?? null
  );
  return req;
}

describe("resolveLocale()", () => {
  describe("cookie-based resolution", () => {
    it("returns zh-CN from cookie", async () => {
      const req = makeRequest({ Cookie: "auth9_locale=zh-CN" });
      expect(await resolveLocale(req)).toBe("zh-CN");
    });

    it("returns en-US from cookie", async () => {
      const req = makeRequest({ Cookie: "auth9_locale=en-US" });
      expect(await resolveLocale(req)).toBe("en-US");
    });

    it("returns ja from cookie", async () => {
      const req = makeRequest({ Cookie: "auth9_locale=ja" });
      expect(await resolveLocale(req)).toBe("ja");
    });

    it("normalizes zh variants to zh-CN", async () => {
      const req = makeRequest({ Cookie: "auth9_locale=zh-TW" });
      expect(await resolveLocale(req)).toBe("zh-CN");
    });

    it("normalizes en variants to en-US", async () => {
      const req = makeRequest({ Cookie: "auth9_locale=en-GB" });
      expect(await resolveLocale(req)).toBe("en-US");
    });
  });

  describe("Accept-Language header resolution", () => {
    it("resolves ja from Accept-Language", async () => {
      const req = makeRequest({ "Accept-Language": "ja,en;q=0.9" });
      expect(await resolveLocale(req)).toBe("ja");
    });

    it("resolves ja-JP to ja", async () => {
      const req = makeRequest({ "Accept-Language": "ja-JP,en;q=0.9" });
      expect(await resolveLocale(req)).toBe("ja");
    });

    it("resolves zh-CN from Accept-Language", async () => {
      const req = makeRequest({ "Accept-Language": "zh-CN,en;q=0.8" });
      expect(await resolveLocale(req)).toBe("zh-CN");
    });

    it("resolves en-US from Accept-Language", async () => {
      const req = makeRequest({ "Accept-Language": "en-US,ja;q=0.7" });
      expect(await resolveLocale(req)).toBe("en-US");
    });

    it("picks first supported locale when multiple present", async () => {
      const req = makeRequest({ "Accept-Language": "fr,ja;q=0.9,en;q=0.8" });
      expect(await resolveLocale(req)).toBe("ja");
    });
  });

  describe("fallback to default locale", () => {
    it("returns default when no cookie and unsupported Accept-Language", async () => {
      const req = makeRequest({ "Accept-Language": "fr,de" });
      expect(await resolveLocale(req)).toBe("zh-CN");
    });

    it("returns default when no headers at all", async () => {
      const req = makeRequest();
      expect(await resolveLocale(req)).toBe("zh-CN");
    });
  });

  describe("cookie takes priority over Accept-Language", () => {
    it("prefers cookie over Accept-Language header", async () => {
      const req = makeRequest({
        Cookie: "auth9_locale=ja",
        "Accept-Language": "en-US",
      });
      expect(await resolveLocale(req)).toBe("ja");
    });
  });
});
