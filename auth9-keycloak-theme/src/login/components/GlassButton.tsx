import type { ButtonHTMLAttributes, ReactNode } from "react";

interface GlassButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "ghost";
  children: ReactNode;
  loading?: boolean;
}

/**
 * Styled button with primary, secondary, and ghost variants.
 */
export function GlassButton({
  variant = "primary",
  children,
  loading = false,
  disabled,
  className = "",
  ...props
}: GlassButtonProps) {
  const buttonClassName = [
    "glass-button",
    `glass-button--${variant}`,
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <button
      className={buttonClassName}
      disabled={disabled || loading}
      {...props}
    >
      {loading ? <span className="loading-spinner" /> : children}
    </button>
  );
}
