import type { ReactNode } from "react";
import { useTheme } from "../hooks/useTheme";
import { ThemeToggle } from "./ThemeToggle";

interface PageLayoutProps {
  children: ReactNode;
}

/**
 * Page wrapper component with animated backdrop and theme toggle.
 */
export function PageLayout({ children }: PageLayoutProps) {
  const { theme, toggleTheme } = useTheme();

  return (
    <div className="login-page">
      <div className="page-backdrop" />
      <ThemeToggle theme={theme} onToggle={toggleTheme} />
      {children}
    </div>
  );
}
