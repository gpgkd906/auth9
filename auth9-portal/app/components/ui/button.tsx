import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import { forwardRef, type ButtonHTMLAttributes } from "react";
import { cn } from "~/lib/utils";

const buttonVariants = cva(
  "inline-flex items-center justify-center whitespace-nowrap rounded-[12px] font-semibold transition-all duration-200 ease-out focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--accent-blue)] focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
  {
    variants: {
      variant: {
        default:
          "bg-[var(--accent-blue)] text-white hover:bg-[#0066D6] active:bg-[#0055B3] shadow-[0_4px_12px_rgba(0,122,255,0.3)] hover:shadow-[0_6px_20px_rgba(0,122,255,0.4)] hover:-translate-y-[1px]",
        destructive:
          "bg-[var(--accent-red)] text-white hover:bg-[#E6342B] active:bg-[#CC2E26] shadow-[0_4px_12px_rgba(255,59,48,0.3)] hover:shadow-[0_6px_20px_rgba(255,59,48,0.4)]",
        outline:
          "border border-[var(--glass-border-subtle)] bg-transparent text-[var(--text-primary)] hover:bg-[var(--sidebar-item-hover)] active:bg-[var(--glass-border-subtle)]",
        secondary:
          "bg-[var(--sidebar-item-hover)] text-[var(--text-primary)] hover:bg-[var(--glass-border-subtle)] active:opacity-80",
        ghost:
          "text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--sidebar-item-hover)] active:bg-[var(--glass-border-subtle)]",
        glass:
          "bg-[var(--glass-bg)] backdrop-blur-[16px] text-[var(--text-primary)] border border-[var(--glass-border)] shadow-[0_4px_12px_var(--glass-shadow),inset_0_1px_0_var(--glass-highlight)] hover:bg-[var(--glass-bg-hover)] hover:-translate-y-[1px]",
        link:
          "text-[var(--accent-blue)] underline-offset-4 hover:underline p-0 h-auto",
      },
      size: {
        default: "h-11 px-6 py-2 text-[14px]",
        sm: "h-9 px-4 text-[13px]",
        lg: "h-12 px-8 text-[15px]",
        icon: "h-10 w-10",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
);

export interface ButtonProps
  extends ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : "button";
    return (
      <Comp
        className={cn(buttonVariants({ variant, size, className }))}
        ref={ref}
        {...props}
      />
    );
  }
);
Button.displayName = "Button";

export { Button, buttonVariants };
