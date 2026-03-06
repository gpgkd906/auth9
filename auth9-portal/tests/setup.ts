import "@testing-library/jest-dom";
import i18next from "i18next";
import { initReactI18next } from "react-i18next";
import { resources, SUPPORTED_LOCALES } from "~/i18n/resources";

// Note: installGlobals() from @remix-run/node is no longer needed
// Node.js 20+ has native support for Web APIs like fetch, Request, Response, etc.

// Polyfill ResizeObserver for Radix UI components in jsdom
if (typeof window !== "undefined" && !window.ResizeObserver) {
  window.ResizeObserver = class ResizeObserver {
    private callback: ResizeObserverCallback;

    constructor(callback: ResizeObserverCallback) {
      this.callback = callback;
    }

    observe() {}
    unobserve() {}
    disconnect() {}
  };
}

// Polyfill PointerEvent for Radix UI interactions
if (typeof window !== "undefined" && !window.PointerEvent) {
  class MockPointerEvent extends MouseEvent {
    public pointerId: number;
    public pointerType: string;
    public isPrimary: boolean;

    constructor(type: string, params: PointerEventInit = {}) {
      super(type, params);
      this.pointerId = params.pointerId || 0;
      this.pointerType = params.pointerType || "mouse";
      this.isPrimary = params.isPrimary || true;
    }
  }
  window.PointerEvent = MockPointerEvent as typeof PointerEvent;
}

const i18nResources = Object.fromEntries(
  SUPPORTED_LOCALES.map((locale) => [locale, { translation: resources[locale] }])
);

void i18next.use(initReactI18next).init({
  lng: "en-US",
  fallbackLng: "en-US",
  resources: i18nResources,
  defaultNS: "translation",
  interpolation: { escapeValue: false },
});
