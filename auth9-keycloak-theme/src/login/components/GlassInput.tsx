import type { InputHTMLAttributes } from "react";

interface GlassInputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
  variant?: "default" | "otp";
}

/**
 * Styled form input with glass effect focus states.
 */
export function GlassInput({
  label,
  error,
  variant = "default",
  id,
  className = "",
  ...props
}: GlassInputProps) {
  const inputClassName = [
    "glass-input",
    variant === "otp" && "glass-input--otp",
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div className="form-group">
      {label && (
        <label htmlFor={id} className="form-label">
          {label}
        </label>
      )}
      <input id={id} className={inputClassName} {...props} />
      {error && <span className="glass-alert glass-alert--error">{error}</span>}
    </div>
  );
}
