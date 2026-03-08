import { useTheme } from "~/hooks/useTheme";
import { useI18n } from "~/i18n";
import { cn } from "~/lib/utils";

export function ThemeToggle() {
  const { theme, setTheme } = useTheme();
  const { t } = useI18n();

  return (
    <div className="theme-toggle" role="group" aria-label={t("common.theme.label")} data-testid="theme-toggle">
      <button
        className={cn("theme-btn", theme === "light" && "active")}
        onClick={() => setTheme("light")}
        title={t("common.theme.light")}
        aria-label={t("common.theme.switchToLight")}
        aria-pressed={theme === "light"}
        tabIndex={0}
        data-testid="theme-light"
      >
        <SunIcon />
      </button>
      <button
        className={cn("theme-btn", theme === "dark" && "active")}
        onClick={() => setTheme("dark")}
        title={t("common.theme.dark")}
        aria-label={t("common.theme.switchToDark")}
        aria-pressed={theme === "dark"}
        tabIndex={0}
        data-testid="theme-dark"
      >
        <MoonIcon />
      </button>
    </div>
  );
}

function SunIcon() {
  return (
    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z"
      />
    </svg>
  );
}

function MoonIcon() {
  return (
    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z"
      />
    </svg>
  );
}
