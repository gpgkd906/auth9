import { useEffect, useMemo, useSyncExternalStore } from "react";
import i18next, { createInstance, type i18n as I18nInstance } from "i18next";
import { initReactI18next, I18nextProvider, useTranslation } from "react-i18next";
import { DEFAULT_LOCALE, resources, SUPPORTED_LOCALES, type AppLocale } from "./resources";

const LANGUAGE_COOKIE = "auth9_locale";
const i18nResources = Object.fromEntries(
  SUPPORTED_LOCALES.map((locale) => [locale, { translation: resources[locale] }])
);

function buildOptions(locale: AppLocale) {
  return {
    lng: locale,
    fallbackLng: DEFAULT_LOCALE,
    supportedLngs: [...SUPPORTED_LOCALES],
    resources: i18nResources,
    defaultNS: "translation",
    interpolation: { escapeValue: false },
    initImmediate: false,
  };
}

let clientInstance: I18nInstance | null = null;

function getClientInstance(locale: AppLocale) {
  if (!clientInstance) {
    clientInstance = createInstance();
    clientInstance.use(initReactI18next);
    void clientInstance.init(buildOptions(locale));
  }
  return clientInstance;
}

function createScopedInstance(locale: AppLocale) {
  const instance = createInstance();
  instance.use(initReactI18next);
  void instance.init(buildOptions(locale));
  return instance;
}

let localeListeners: Array<() => void> = [];

function writeLocaleCookie(locale: AppLocale) {
  if (typeof document === "undefined") return;
  document.cookie = `${LANGUAGE_COOKIE}=${encodeURIComponent(locale)}; path=/; max-age=31536000; samesite=lax`;
}

function notifyLocaleListeners() {
  for (const listener of localeListeners) {
    listener();
  }
}

function subscribeLocale(listener: () => void) {
  localeListeners = [...localeListeners, listener];
  return () => {
    localeListeners = localeListeners.filter((item) => item !== listener);
  };
}

function getStoredLocale(): AppLocale {
  if (typeof document === "undefined") return DEFAULT_LOCALE;
  const cookie = document.cookie
    .split("; ")
    .find((item) => item.startsWith(`${LANGUAGE_COOKIE}=`));
  const raw = cookie?.split("=")[1];
  if (!raw) return DEFAULT_LOCALE;
  const decoded = decodeURIComponent(raw) as AppLocale;
  return SUPPORTED_LOCALES.includes(decoded) ? decoded : DEFAULT_LOCALE;
}

export function I18nProvider({
  locale,
  children,
}: {
  locale: AppLocale;
  children: React.ReactNode;
}) {
  const instance = useMemo(() => {
    if (typeof document === "undefined") {
      return createScopedInstance(locale);
    }
    return getClientInstance(locale);
  }, [locale]);

  useEffect(() => {
    void instance.changeLanguage(locale);
    writeLocaleCookie(locale);
    notifyLocaleListeners();
  }, [instance, locale]);

  return <I18nextProvider i18n={instance}>{children}</I18nextProvider>;
}

export function useI18n() {
  return useTranslation();
}

export function useLocale() {
  const { i18n } = useTranslation();
  const locale = (i18n.resolvedLanguage || i18n.language || DEFAULT_LOCALE) as AppLocale;

  return {
    locale,
    setLocale: async (nextLocale: AppLocale) => {
      writeLocaleCookie(nextLocale);
      await i18n.changeLanguage(nextLocale);
      notifyLocaleListeners();
    },
  };
}

export function useLocaleStore() {
  return useSyncExternalStore(subscribeLocale, getStoredLocale, () => DEFAULT_LOCALE);
}

export function getI18n() {
  return i18next;
}

export { DEFAULT_LOCALE, SUPPORTED_LOCALES } from "./resources";
export type { AppLocale } from "./resources";
