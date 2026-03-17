import type { ReactNode } from "react";
import { AuthBrandPanel, buildAuthBrandStyle } from "~/components/auth/AuthBrandPanel";
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
  panelTitle,
  panelDescription,
  panelEyebrow,
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
        <div className="relative z-10 grid w-full max-w-6xl items-center gap-6 lg:grid-cols-[minmax(0,1fr)_minmax(0,480px)]">
          {branding && panelTitle && panelDescription ? (
            <AuthBrandPanel
              branding={branding}
              eyebrow={panelEyebrow}
              title={panelTitle}
              description={panelDescription}
            />
          ) : null}
          {children}
        </div>
      </div>
    </>
  );
}
