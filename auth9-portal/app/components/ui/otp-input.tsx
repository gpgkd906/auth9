import { useRef, useCallback, type KeyboardEvent, type ClipboardEvent } from "react";
import { cn } from "~/lib/utils";

interface OtpInputProps {
  length?: number;
  onComplete: (code: string) => void;
  disabled?: boolean;
  error?: boolean;
}

export function OtpInput({ length = 6, onComplete, disabled, error }: OtpInputProps) {
  const inputsRef = useRef<(HTMLInputElement | null)[]>([]);

  const focusInput = useCallback((index: number) => {
    inputsRef.current[index]?.focus();
  }, []);

  const getCode = useCallback(() => {
    return inputsRef.current.map((input) => input?.value || "").join("");
  }, []);

  const handleInput = useCallback(
    (index: number, value: string) => {
      const digit = value.replace(/\D/g, "").slice(-1);
      const input = inputsRef.current[index];
      if (input) input.value = digit;

      if (digit && index < length - 1) {
        focusInput(index + 1);
      }

      const code = getCode();
      if (code.length === length) {
        onComplete(code);
      }
    },
    [length, onComplete, focusInput, getCode]
  );

  const handleKeyDown = useCallback(
    (index: number, e: KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "Backspace") {
        const input = inputsRef.current[index];
        if (input && !input.value && index > 0) {
          const prev = inputsRef.current[index - 1];
          if (prev) {
            prev.value = "";
            prev.focus();
          }
        }
      }
      if (e.key === "ArrowLeft" && index > 0) {
        focusInput(index - 1);
      }
      if (e.key === "ArrowRight" && index < length - 1) {
        focusInput(index + 1);
      }
    },
    [length, focusInput]
  );

  const handlePaste = useCallback(
    (e: ClipboardEvent<HTMLInputElement>) => {
      e.preventDefault();
      const pasted = e.clipboardData.getData("text").replace(/\D/g, "").slice(0, length);
      for (let i = 0; i < length; i++) {
        const input = inputsRef.current[i];
        if (input) input.value = pasted[i] || "";
      }
      if (pasted.length === length) {
        onComplete(pasted);
      } else {
        focusInput(Math.min(pasted.length, length - 1));
      }
    },
    [length, onComplete, focusInput]
  );

  return (
    <div className="flex justify-center gap-2">
      {Array.from({ length }, (_, i) => (
        <input
          key={i}
          ref={(el) => { inputsRef.current[i] = el; }}
          type="text"
          inputMode="numeric"
          autoComplete={i === 0 ? "one-time-code" : "off"}
          maxLength={1}
          disabled={disabled}
          className={cn(
            "h-12 w-10 rounded-xl border bg-[var(--glass-bg)] text-center text-lg font-mono text-[var(--text-primary)]",
            "outline-none transition-all duration-200",
            "focus:border-[var(--accent-blue)] focus:ring-2 focus:ring-[var(--accent-blue)]/25",
            error
              ? "border-[var(--accent-red)] focus:border-[var(--accent-red)] focus:ring-[var(--accent-red)]/25"
              : "border-[var(--glass-border-subtle)]",
            disabled && "opacity-50 cursor-not-allowed"
          )}
          onInput={(e) => handleInput(i, (e.target as HTMLInputElement).value)}
          onKeyDown={(e) => handleKeyDown(i, e)}
          onPaste={handlePaste}
          onFocus={(e) => e.target.select()}
        />
      ))}
    </div>
  );
}
