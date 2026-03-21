import { useState, useCallback } from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "~/components/ui/dialog";
import { Button } from "~/components/ui/button";
import { useI18n } from "~/i18n";

interface RecoveryCodesDialogProps {
  codes: string[];
  open: boolean;
  onClose: () => void;
}

export function RecoveryCodesDialog({ codes, open, onClose }: RecoveryCodesDialogProps) {
  const { t } = useI18n();
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(codes.join("\n"));
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Clipboard API may not be available
    }
  }, [codes]);

  return (
    <Dialog open={open} onOpenChange={(isOpen) => { if (!isOpen) onClose(); }}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{t("accountMfa.recovery.generated")}</DialogTitle>
          <DialogDescription>
            {t("accountMfa.recovery.generatedHint")}
          </DialogDescription>
        </DialogHeader>

        <div className="rounded-2xl border border-[var(--glass-border-subtle)] bg-[var(--glass-bg)] p-4">
          <div className="grid grid-cols-2 gap-2">
            {codes.map((code) => (
              <code
                key={code}
                className="text-sm font-mono text-[var(--text-primary)] select-all text-center py-1"
              >
                {code}
              </code>
            ))}
          </div>
        </div>

        <DialogFooter className="flex-row gap-2 sm:justify-between">
          <Button variant="outline" onClick={handleCopy}>
            {copied ? t("accountMfa.recovery.copied") : t("accountMfa.recovery.copyAll")}
          </Button>
          <Button onClick={onClose}>
            {t("accountMfa.recovery.close")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
