import type { ReactNode } from "react";
import { buildAuthBrandStyle } from "~/components/auth/AuthBrandPanel";
import { LanguageSwitcher } from "~/components/LanguageSwitcher";
import { ThemeToggle } from "~/components/ThemeToggle";
import { cn } from "~/lib/utils";
import type { BrandingConfig } from "~/services/api";

interface AuthPageShellProps {
  children: ReactNode;
  className?: string;
  branding?: BrandingConfig;
  panelTitle?: string;
  panelDescription?: string;
  panelEyebrow?: string;
}

export function AuthPageShell({
  children,
  className,
  branding,
}: AuthPageShellProps) {
  const style = branding ? buildAuthBrandStyle(branding) : undefined;

  return (
    <>
      <div className="fixed top-6 right-6 z-20 flex items-center gap-3">
        <LanguageSwitcher />
        <ThemeToggle />
      </div>

      <div
        style={style}
        className={cn("auth-page-shell min-h-screen flex items-center justify-center px-4 py-8 sm:px-6 relative", className)}
      >
        <div className="page-backdrop" />
        <div className="relative z-10 flex w-full justify-center">
          {children}
        </div>
      </div>
    </>
  );
}
