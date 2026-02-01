import { type HTMLAttributes } from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "~/lib/utils";

const badgeVariants = cva(
  "inline-flex items-center rounded-full px-2.5 py-0.5 text-[11px] font-semibold transition-colors",
  {
    variants: {
      variant: {
        default:
          "bg-[var(--accent-blue-light)] text-[var(--accent-blue)]",
        secondary:
          "bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)] border border-[var(--glass-border-subtle)]",
        success:
          "bg-[var(--accent-green-light)] text-[var(--accent-green)]",
        warning:
          "bg-[var(--accent-orange-light)] text-[var(--accent-orange)]",
        destructive:
          "bg-[var(--accent-red-light)] text-[var(--accent-red)]",
        outline:
          "text-[var(--text-primary)] border border-[var(--glass-border-subtle)]",
      },
    },
    defaultVariants: {
      variant: "default",
    },
  }
);

export interface BadgeProps
  extends HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof badgeVariants> {}

function Badge({ className, variant, ...props }: BadgeProps) {
  return (
    <div className={cn(badgeVariants({ variant }), className)} {...props} />
  );
}

export { Badge, badgeVariants };
