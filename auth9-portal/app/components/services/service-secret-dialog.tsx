import { CopyIcon } from "@radix-ui/react-icons";
import { useState } from "react";
import { Button } from "~/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "~/components/ui/dialog";
import { Label } from "~/components/ui/label";
import { useI18n } from "~/i18n";
import { copyToClipboard } from "./copyable-value";
import type { ServiceSecretDialogState } from "./types";

interface ServiceSecretDialogProps {
  secretDialog: ServiceSecretDialogState | null;
  onOpenChange: (open: boolean) => void;
}

export function ServiceSecretDialog({ secretDialog, onOpenChange }: ServiceSecretDialogProps) {
  const { t } = useI18n();
  const [copiedField, setCopiedField] = useState<string | null>(null);

  const handleCopy = async (text: string, fieldName: string) => {
    await copyToClipboard(text);
    setCopiedField(fieldName);
    setTimeout(() => setCopiedField(null), 2000);
  };

  return (
    <Dialog open={Boolean(secretDialog)} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>
            {secretDialog?.isNew
              ? t("services.detail.secretDialogNewTitle")
              : t("services.detail.secretDialogRegeneratedTitle")}
          </DialogTitle>
          <DialogDescription>{t("services.detail.secretDialogDescription")}</DialogDescription>
        </DialogHeader>
        <div className="space-y-4">
          <div>
            <Label className="text-xs text-[var(--text-secondary)]">{t("services.detail.clientCreated")}</Label>
            <div className="mt-1 flex items-center gap-2">
              <div className="flex-1 select-all break-all rounded border bg-[var(--sidebar-item-hover)] p-2 font-mono text-sm">
                {secretDialog?.clientId}
              </div>
              <Button
                variant="outline"
                size="icon"
                className="h-8 w-8 shrink-0"
                onClick={() => secretDialog && handleCopy(secretDialog.clientId, "dialog-id")}
              >
                {copiedField === "dialog-id" ? (
                  <span className="text-xs text-[var(--accent-green)]">&#10003;</span>
                ) : (
                  <CopyIcon className="h-4 w-4" />
                )}
              </Button>
            </div>
          </div>
          <div>
            <Label className="text-xs text-[var(--text-secondary)]">{t("services.detail.clientSecret")}</Label>
            <div className="mt-1 flex items-center gap-2">
              <div className="flex-1 select-all break-all rounded border border-[var(--accent-green)]/20 bg-[var(--accent-green)]/10 p-3 text-center font-mono font-bold text-[var(--accent-green)] [word-break:break-all]">
                {secretDialog?.secret}
              </div>
              <Button
                variant="outline"
                size="icon"
                className="h-8 w-8 shrink-0"
                onClick={() => secretDialog && handleCopy(secretDialog.secret, "dialog-secret")}
              >
                {copiedField === "dialog-secret" ? (
                  <span className="text-xs text-[var(--accent-green)]">&#10003;</span>
                ) : (
                  <CopyIcon className="h-4 w-4" />
                )}
              </Button>
            </div>
          </div>
        </div>
        <DialogFooter>
          <Button onClick={() => onOpenChange(false)}>{t("services.detail.close")}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
