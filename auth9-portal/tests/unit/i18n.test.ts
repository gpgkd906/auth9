import { describe, it, expect } from "vitest";
import { resources, SUPPORTED_LOCALES, DEFAULT_LOCALE, type AppLocale } from "~/i18n/resources";
import { translate, getLocaleDisplayName } from "~/i18n/translate";
import zhCN from "~/i18n/locales/zh-CN";
import enUS from "~/i18n/locales/en-US";
import ja from "~/i18n/locales/ja";

const I18NEXT_PLURAL_SUFFIX = /_(?:zero|one|two|few|many|other|\d+)$/;

function collectKeys(obj: Record<string, unknown>, prefix = ""): string[] {
  const keys: string[] = [];
  for (const [key, value] of Object.entries(obj)) {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    if (value && typeof value === "object" && !Array.isArray(value)) {
      keys.push(...collectKeys(value as Record<string, unknown>, fullKey));
    } else if (!I18NEXT_PLURAL_SUFFIX.test(fullKey)) {
      keys.push(fullKey);
    }
  }
  return keys.sort();
}

describe("i18n resources module", () => {
  describe("module structure after split", () => {
    it("resources re-exports all three locales", () => {
      expect(resources).toHaveProperty("zh-CN");
      expect(resources).toHaveProperty("en-US");
      expect(resources).toHaveProperty("ja");
    });

    it("re-exported objects match direct imports", () => {
      expect(resources["zh-CN"]).toBe(zhCN);
      expect(resources["en-US"]).toBe(enUS);
      expect(resources["ja"]).toBe(ja);
    });

    it("SUPPORTED_LOCALES contains all three locale codes", () => {
      expect(SUPPORTED_LOCALES).toEqual(["zh-CN", "en-US", "ja"]);
    });

    it("DEFAULT_LOCALE is zh-CN", () => {
      expect(DEFAULT_LOCALE).toBe("zh-CN");
    });
  });

  describe("translation key parity between locales", () => {
    const zhKeys = collectKeys(resources["zh-CN"] as unknown as Record<string, unknown>);
    const enKeys = collectKeys(resources["en-US"] as unknown as Record<string, unknown>);
    const jaKeys = collectKeys(resources["ja"] as unknown as Record<string, unknown>);

    it("zh-CN and en-US have identical key sets", () => {
      const missingInEn = zhKeys.filter((k) => !enKeys.includes(k));
      const extraInEn = enKeys.filter((k) => !zhKeys.includes(k));
      expect(missingInEn).toEqual([]);
      expect(extraInEn).toEqual([]);
    });

    it("zh-CN and ja have identical key sets", () => {
      const missingInJa = zhKeys.filter((k) => !jaKeys.includes(k));
      const extraInJa = jaKeys.filter((k) => !zhKeys.includes(k));
      expect(missingInJa).toEqual([]);
      expect(extraInJa).toEqual([]);
    });
  });

  describe("each locale has non-empty top-level sections", () => {
    const topLevelSections = Object.keys(resources["zh-CN"]);

    for (const locale of SUPPORTED_LOCALES) {
      it(`${locale} has all top-level sections`, () => {
        const localeKeys = Object.keys(resources[locale]);
        expect(localeKeys).toEqual(topLevelSections);
      });
    }
  });
});

describe("translate()", () => {
  it("returns zh-CN translation for a simple key", () => {
    expect(translate("zh-CN", "common.appName")).toBe("Auth9");
    expect(translate("zh-CN", "common.buttons.signIn")).toBe("登录");
  });

  it("returns en-US translation for a simple key", () => {
    expect(translate("en-US", "common.buttons.signIn")).toBe("Sign in");
  });

  it("returns ja translation for a simple key", () => {
    expect(translate("ja", "common.buttons.signIn")).toBe("サインイン");
  });

  it("interpolates template variables", () => {
    expect(translate("en-US", "dashboard.metaTitle", { tenantName: "Acme" })).toBe(
      "Acme - Auth9"
    );
    expect(translate("zh-CN", "dashboard.metaTitle", { tenantName: "测试" })).toBe(
      "测试 - Auth9"
    );
    expect(translate("ja", "dashboard.metaTitle", { tenantName: "テスト" })).toBe(
      "テスト - Auth9"
    );
  });

  it("falls back to DEFAULT_LOCALE when key missing in target", () => {
    expect(translate("en-US", "common.appName")).toBe("Auth9");
  });

  it("returns key path when key does not exist in any locale", () => {
    expect(translate("en-US", "nonexistent.key.path")).toBe("nonexistent.key.path");
  });
});

describe("getLocaleDisplayName()", () => {
  it("returns display name for zh-CN", () => {
    expect(getLocaleDisplayName("zh-CN")).toBe("简体中文");
  });

  it("returns display name for en-US", () => {
    expect(getLocaleDisplayName("en-US")).toBe("English");
  });

  it("returns display name for ja", () => {
    expect(getLocaleDisplayName("ja")).toBe("日本語");
  });

  it("covers all supported locales", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const name = getLocaleDisplayName(locale);
      expect(name).toBeTruthy();
      expect(typeof name).toBe("string");
    }
  });
});
