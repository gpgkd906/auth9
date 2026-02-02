import type { ReactNode, CSSProperties } from "react";

interface GlassCardProps {
  children: ReactNode;
  className?: string;
  style?: CSSProperties;
}

/**
 * Glass morphism card container with blur effect and shadow.
 */
export function GlassCard({ children, className = "", style }: GlassCardProps) {
  return (
    <div className={`liquid-glass login-card ${className}`.trim()} style={style}>
      {children}
    </div>
  );
}
