import type { ReactNode } from "react";
import { cn } from "~/lib/utils";

interface AuthMethodStackProps {
  children: ReactNode;
  className?: string;
}

export function AuthMethodStack({ children, className }: AuthMethodStackProps) {
  return (
    <div
      className={cn(
        "flex flex-col gap-3 rounded-[28px] border border-[var(--auth-surface-border)] bg-[var(--auth-surface-bg)] p-5 shadow-lg backdrop-blur-2xl",
        className,
      )}
    >
      {children}
    </div>
  );
}
