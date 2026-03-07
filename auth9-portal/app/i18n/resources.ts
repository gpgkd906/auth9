import zhCN from "./locales/zh-CN";
import enUS from "./locales/en-US";
import ja from "./locales/ja";

export const DEFAULT_LOCALE = "zh-CN" as const;
export const SUPPORTED_LOCALES = ["zh-CN", "en-US", "ja"] as const;
export type AppLocale = (typeof SUPPORTED_LOCALES)[number];

export const resources = {
  "zh-CN": zhCN,
  "en-US": enUS,
  ja,
} as const;
