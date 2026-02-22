import { forwardRef, type InputHTMLAttributes } from "react";
import { cn } from "~/lib/utils";

export type InputProps = InputHTMLAttributes<HTMLInputElement>;

const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ className, type, ...props }, ref) => {
    return (
      <input
        type={type}
        className={cn(
          "flex h-10 w-full rounded-[12px] px-4 py-2.5 text-[14px]",
          "bg-[var(--sidebar-item-hover)] text-[var(--text-primary)]",
          "border border-[var(--glass-border-subtle)]",
          "placeholder:text-[var(--text-tertiary)]",
          "focus-visible:outline-2 focus-visible:outline-[var(--accent-blue)] focus-visible:outline-offset-2",
          "focus:border-[var(--accent-blue)] focus:ring-2 focus:ring-[var(--accent-blue-light)]",
          "disabled:cursor-not-allowed disabled:opacity-50",
          "transition-all duration-200",
          className
        )}
        ref={ref}
        {...props}
      />
    );
  }
);
Input.displayName = "Input";

export { Input };
