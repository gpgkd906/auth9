import "@testing-library/jest-dom";
import { installGlobals } from "@remix-run/node";

installGlobals();

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
