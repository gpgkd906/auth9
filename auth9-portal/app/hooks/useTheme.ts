import { useCallback, useEffect, useSyncExternalStore } from "react";

export type Theme = "light" | "dark";

const STORAGE_KEY = "auth9-theme";

// Read the current theme from the DOM (set by theme-init.js before React hydrates)
function getSnapshot(): Theme {
  const attr = document.documentElement.getAttribute("data-theme");
  return attr === "dark" ? "dark" : "light";
}

function getServerSnapshot(): Theme {
  return "light";
}

// Notify subscribers when theme changes
let listeners: Array<() => void> = [];

function subscribe(listener: () => void) {
  listeners = [...listeners, listener];
  return () => {
    listeners = listeners.filter((l) => l !== listener);
  };
}

function emitChange() {
  for (const listener of listeners) {
    listener();
  }
}

function applyTheme(newTheme: Theme) {
  document.documentElement.setAttribute("data-theme", newTheme);
}

export function useTheme() {
  const theme = useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);

  const setTheme = useCallback((newTheme: Theme) => {
    if (typeof window === "undefined") return;

    localStorage.setItem(STORAGE_KEY, newTheme);
    applyTheme(newTheme);
    emitChange();
  }, []);

  const toggleTheme = useCallback(() => {
    const current = getSnapshot();
    setTheme(current === "light" ? "dark" : "light");
  }, [setTheme]);

  // On mount, sync localStorage → DOM (in production theme-init.js does this,
  // but this handles cases where theme-init.js hasn't run, e.g. tests)
  useEffect(() => {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "dark" || stored === "light") {
      const current = getSnapshot();
      if (current !== stored) {
        applyTheme(stored);
        emitChange();
      }
    }
  }, []);

  // Listen for system preference changes
  useEffect(() => {
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");

    const handleChange = (e: MediaQueryListEvent) => {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (!stored) {
        setTheme(e.matches ? "dark" : "light");
      }
    };

    mediaQuery.addEventListener("change", handleChange);
    return () => mediaQuery.removeEventListener("change", handleChange);
  }, [setTheme]);

  return {
    theme,
    setTheme,
    toggleTheme,
    isDark: theme === "dark",
    isLight: theme === "light",
  };
}
