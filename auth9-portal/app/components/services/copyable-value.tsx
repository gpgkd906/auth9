import { CopyIcon } from "@radix-ui/react-icons";
import { useState } from "react";
import { Button } from "~/components/ui/button";
import { useI18n } from "~/i18n";

export function copyToClipboard(text: string): Promise<void> {
  return navigator.clipboard.writeText(text);
}

export function CodeBlock({ children, label }: { children: string; label?: string }) {
  const { t } = useI18n();
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    await copyToClipboard(children);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="group relative">
      {label && <div className="mb-1 text-xs text-[var(--text-tertiary)]">{label}</div>}
      <div className="overflow-x-auto whitespace-pre rounded-lg bg-[#0d1117] p-4 font-mono text-sm text-[#c9d1d9]">
        {children}
      </div>
      <Button
        variant="ghost"
        size="icon"
        className="absolute top-2 right-2 h-11 w-11 text-[#8b949e] opacity-0 transition-opacity group-hover:opacity-100 hover:bg-[#30363d] hover:text-white sm:h-7 sm:w-7"
        onClick={handleCopy}
        title={t("common.buttons.copy")}
      >
        {copied ? (
          <span className="text-xs text-[var(--accent-green)]">&#10003;</span>
        ) : (
          <CopyIcon className="h-3.5 w-3.5" />
        )}
      </Button>
    </div>
  );
}

export function CopyValue({ value, fieldId }: { value: string; fieldId: string }) {
  const { t } = useI18n();
  const [copied, setCopied] = useState(false);

  return (
    <div className="flex min-w-0 items-center gap-2">
      <code className="min-w-0 flex-1 select-all break-all whitespace-normal font-mono text-sm text-[var(--text-primary)] [word-break:break-all]">
        {value}
      </code>
      <Button
        variant="ghost"
        className="h-8 w-8 shrink-0 p-0 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)]"
        onClick={async () => {
          await copyToClipboard(value);
          setCopied(true);
          setTimeout(() => setCopied(false), 2000);
        }}
        title={t("common.buttons.copyField", { field: fieldId })}
      >
        {copied ? (
          <span className="text-xs text-[var(--accent-green)]">&#10003;</span>
        ) : (
          <CopyIcon className="h-3.5 w-3.5" />
        )}
      </Button>
    </div>
  );
}
