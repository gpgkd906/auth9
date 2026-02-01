import { forwardRef, type InputHTMLAttributes } from "react";
import { cn } from "~/lib/utils";

export type InputProps = InputHTMLAttributes<HTMLInputElement>;

const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ className, type, ...props }, ref) => {
    return (
      <input
        type={type}
        className={cn(
          "flex h-11 w-full rounded-apple bg-gray-100 px-4 py-2 text-base",
          "placeholder:text-gray-400",
          "focus:bg-white focus:outline-none focus:ring-2 focus:ring-apple-blue",
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
