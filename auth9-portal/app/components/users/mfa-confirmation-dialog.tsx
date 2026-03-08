import { useEffect, useState } from "react";
import { Form } from "react-router";
import type { User } from "~/services/api";
import { useI18n } from "~/i18n";
import { Button } from "~/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "~/components/ui/dialog";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";

interface MfaActionState {
  user: User;
  action: "enable" | "disable";
}

interface MfaConfirmationDialogProps {
  error?: string | null;
  isSubmitting: boolean;
  mfaAction: MfaActionState | null;
  onOpenChange: (open: boolean) => void;
}

export function MfaConfirmationDialog({
  error,
  isSubmitting,
  mfaAction,
  onOpenChange,
}: MfaConfirmationDialogProps) {
  const { t } = useI18n();
  const [password, setPassword] = useState("");

  useEffect(() => {
    if (!mfaAction) {
      setPassword("");
    }
  }, [mfaAction]);

  return (
    <Dialog
      open={Boolean(mfaAction)}
      onOpenChange={(open) => {
        if (!open) {
          setPassword("");
        }
        onOpenChange(open);
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>
            {mfaAction?.action === "enable" ? t("usersPage.enableMfaTitle") : t("usersPage.disableMfaTitle")}
          </DialogTitle>
          <DialogDescription>
            {t("usersPage.mfaDescription", {
              action: mfaAction?.action === "enable" ? t("usersPage.enabling") : t("usersPage.disabling"),
              email: mfaAction?.user.email || "",
            })}
          </DialogDescription>
        </DialogHeader>
        <Form method="post" className="space-y-4">
          <input type="hidden" name="intent" value={mfaAction?.action === "enable" ? "enable_mfa" : "disable_mfa"} />
          <input type="hidden" name="id" value={mfaAction?.user.id || ""} />
          <div className="space-y-1.5">
            <Label htmlFor="mfa-confirm-password">{t("usersPage.yourPassword")}</Label>
            <Input
              id="mfa-confirm-password"
              name="confirm_password"
              type="password"
              required
              placeholder={t("usersPage.passwordConfirmPlaceholder")}
              value={password}
              onChange={(event) => setPassword(event.target.value)}
            />
          </div>
          {error && <p className="text-sm text-[var(--accent-red)]">{error}</p>}
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              className="bg-[var(--glass-bg)]"
              onClick={() => {
                setPassword("");
                onOpenChange(false);
              }}
            >
              {t("common.buttons.cancel")}
            </Button>
            <Button type="submit" disabled={isSubmitting || !password}>
              {mfaAction?.action === "enable" ? t("usersPage.enableMfa") : t("usersPage.disableMfa")}
            </Button>
          </DialogFooter>
        </Form>
      </DialogContent>
    </Dialog>
  );
}
