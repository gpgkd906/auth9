import { createCookie } from "react-router";
import { DEFAULT_LOCALE, SUPPORTED_LOCALES, type AppLocale } from "~/i18n";

const isProduction = process.env.NODE_ENV === "production";
const LOCALE_COOKIE_NAME = "auth9_locale";

export const localeCookie = createCookie(LOCALE_COOKIE_NAME, {
  path: "/",
  sameSite: "lax",
  httpOnly: false,
  secure: isProduction,
  maxAge: 60 * 60 * 24 * 365,
});

function normalizeLocale(input: string | null | undefined): AppLocale | null {
  if (!input) return null;
  const lower = input.toLowerCase();
  if (lower.startsWith("zh")) return "zh-CN";
  if (lower.startsWith("en")) return "en-US";
  if (lower.startsWith("ja")) return "ja";
  return null;
}

function readCookieValue(cookieHeader: string | null, name: string) {
  if (!cookieHeader) return null;
  const cookies = cookieHeader.split(/;\s*/);
  for (const cookie of cookies) {
    const [rawName, ...rest] = cookie.split("=");
    if (rawName !== name) continue;
    return decodeURIComponent(rest.join("="));
  }
  return null;
}

export async function resolveLocale(request: Request): Promise<AppLocale> {
  const cookieHeader = request.headers.get("Cookie");
  const localeFromCookie = normalizeLocale(readCookieValue(cookieHeader, LOCALE_COOKIE_NAME));
  if (localeFromCookie) return localeFromCookie;

  const acceptLanguage = request.headers.get("Accept-Language") || "";
  const locales = acceptLanguage
    .split(",")
    .map((entry) => entry.split(";")[0]?.trim())
    .filter(Boolean);

  for (const locale of locales) {
    const normalized = normalizeLocale(locale);
    if (normalized && SUPPORTED_LOCALES.includes(normalized)) {
      return normalized;
    }
  }

  return DEFAULT_LOCALE;
}

export async function serializeLocaleCookie(locale: AppLocale) {
  // Use plain cookie string matching the client-side format in i18n/index.tsx.
  // React Router's localeCookie.serialize() base64-encodes the value, which the
  // manual readCookieValue() parser cannot decode, causing the SSR locale to be
  // ignored on subsequent requests.
  const secure = isProduction ? "; Secure" : "";
  return `${LOCALE_COOKIE_NAME}=${encodeURIComponent(locale)}; Path=/; Max-Age=${60 * 60 * 24 * 365}; SameSite=Lax${secure}`;
}
