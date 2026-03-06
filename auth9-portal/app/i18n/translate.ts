import { DEFAULT_LOCALE, resources, type AppLocale } from "./resources";

type Primitive = string | number | boolean;
type Values = Record<string, Primitive | undefined>;

function getByPath(obj: unknown, path: string): unknown {
  return path.split(".").reduce<unknown>((acc, segment) => {
    if (acc && typeof acc === "object" && segment in acc) {
      return (acc as Record<string, unknown>)[segment];
    }
    return undefined;
  }, obj);
}

function interpolate(template: string, values?: Values): string {
  if (!values) return template;
  return template.replace(/\{\{\s*(\w+)\s*\}\}/g, (_, key: string) => {
    const value = values[key];
    return value === undefined ? "" : String(value);
  });
}

export function translate(locale: AppLocale, key: string, values?: Values): string {
  const message = getByPath(resources[locale], key) ?? getByPath(resources[DEFAULT_LOCALE], key);
  if (typeof message !== "string") {
    return key;
  }
  return interpolate(message, values);
}

export function getLocaleDisplayName(locale: AppLocale): string {
  return locale === "zh-CN" ? "简体中文" : "English";
}
