import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useTheme } from "~/hooks/useTheme";
import { ThemeToggle } from "~/components/ThemeToggle";

describe("useTheme", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
  });

  it("returns light theme by default", () => {
    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe("light");
    expect(result.current.isLight).toBe(true);
    expect(result.current.isDark).toBe(false);
  });

  it("reads theme from localStorage", () => {
    localStorage.setItem("auth9-theme", "dark");
    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe("dark");
    expect(result.current.isDark).toBe(true);
  });

  it("setTheme updates theme and localStorage", () => {
    const { result } = renderHook(() => useTheme());

    act(() => {
      result.current.setTheme("dark");
    });

    expect(result.current.theme).toBe("dark");
    expect(result.current.isDark).toBe(true);
    expect(localStorage.getItem("auth9-theme")).toBe("dark");
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  });

  it("setTheme to light sets data-theme to light", () => {
    document.documentElement.setAttribute("data-theme", "dark");
    const { result } = renderHook(() => useTheme());

    act(() => {
      result.current.setTheme("light");
    });

    expect(result.current.theme).toBe("light");
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");
  });

  it("toggleTheme switches between light and dark", () => {
    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe("light");

    act(() => {
      result.current.toggleTheme();
    });

    expect(result.current.theme).toBe("dark");

    act(() => {
      result.current.toggleTheme();
    });

    expect(result.current.theme).toBe("light");
  });

  it("initializes dark theme on mount when stored", () => {
    localStorage.setItem("auth9-theme", "dark");
    renderHook(() => useTheme());
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  });

  it("ignores invalid localStorage values", () => {
    localStorage.setItem("auth9-theme", "invalid");
    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe("light");
  });

  it("responds to system preference change when no stored preference", () => {
    // Capture the listener added by the hook
    let capturedListener: ((e: MediaQueryListEvent) => void) | null = null;
    const originalMatchMedia = window.matchMedia;

    window.matchMedia = vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      addEventListener: (_event: string, listener: (e: MediaQueryListEvent) => void) => {
        capturedListener = listener;
      },
      removeEventListener: vi.fn(),
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe("light");

    // Simulate system dark mode change
    act(() => {
      capturedListener?.({ matches: true } as MediaQueryListEvent);
    });

    expect(result.current.theme).toBe("dark");

    window.matchMedia = originalMatchMedia;
  });

  it("ignores system preference change when user has stored preference", () => {
    localStorage.setItem("auth9-theme", "light");

    let capturedListener: ((e: MediaQueryListEvent) => void) | null = null;
    const originalMatchMedia = window.matchMedia;

    window.matchMedia = vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      addEventListener: (_event: string, listener: (e: MediaQueryListEvent) => void) => {
        capturedListener = listener;
      },
      removeEventListener: vi.fn(),
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe("light");

    // Simulate system dark mode change - should be ignored because stored = "light"
    act(() => {
      capturedListener?.({ matches: true } as MediaQueryListEvent);
    });

    expect(result.current.theme).toBe("light");

    window.matchMedia = originalMatchMedia;
  });
});

describe("ThemeToggle", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
  });

  it("renders light and dark mode buttons", () => {
    render(<ThemeToggle />);
    expect(screen.getByLabelText("Switch to light mode")).toBeInTheDocument();
    expect(screen.getByLabelText("Switch to dark mode")).toBeInTheDocument();
  });

  it("clicking dark mode button sets dark theme", async () => {
    const user = userEvent.setup();
    render(<ThemeToggle />);

    await user.click(screen.getByLabelText("Switch to dark mode"));
    expect(localStorage.getItem("auth9-theme")).toBe("dark");
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  });

  it("clicking light mode button sets light theme", async () => {
    localStorage.setItem("auth9-theme", "dark");
    const user = userEvent.setup();
    render(<ThemeToggle />);

    await user.click(screen.getByLabelText("Switch to light mode"));
    expect(localStorage.getItem("auth9-theme")).toBe("light");
  });
});
