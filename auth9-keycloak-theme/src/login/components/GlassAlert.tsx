import type { ReactNode } from "react";

interface GlassAlertProps {
  variant?: "error" | "success" | "warning" | "info";
  children: ReactNode;
  className?: string;
}

/**
 * Alert/message component with error, success, warning, and info variants.
 */
export function GlassAlert({
  variant = "error",
  children,
  className = "",
}: GlassAlertProps) {
  if (!children) return null;

  return (
    <div className={`glass-alert glass-alert--${variant} ${className}`.trim()}>
      {children}
    </div>
  );
}
