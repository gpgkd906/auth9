import { useEffect, useState } from "react";
import { Form } from "react-router";
import type { Tenant } from "~/services/api";
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
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "~/components/ui/select";

interface CreateUserDialogProps {
  activeTenantId?: string;
  error?: string | null;
  isSubmitting: boolean;
  open: boolean;
  tenants: Tenant[];
  onOpenChange: (open: boolean) => void;
}

export function CreateUserDialog({
  activeTenantId,
  error,
  isSubmitting,
  open,
  tenants,
  onOpenChange,
}: CreateUserDialogProps) {
  const { t } = useI18n();
  const [emailError, setEmailError] = useState<string | null>(null);
  const [emailValue, setEmailValue] = useState("");

  const resetFormState = () => {
    setEmailError(null);
    setEmailValue("");
  };

  useEffect(() => {
    if (!open) {
      resetFormState();
    }
  }, [open]);

  const validateEmail = (email: string): boolean => {
    if (!email) {
      setEmailError(t("usersPage.emailRequired"));
      return false;
    }
    if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
      setEmailError(t("usersPage.emailInvalid"));
      return false;
    }
    setEmailError(null);
    return true;
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(nextOpen) => {
        if (!nextOpen) {
          resetFormState();
        }
        onOpenChange(nextOpen);
      }}
    >
      <DialogContent aria-modal="true">
        <DialogHeader>
          <DialogTitle>{t("usersPage.createUserTitle")}</DialogTitle>
          <DialogDescription>{t("usersPage.createUserDescription")}</DialogDescription>
        </DialogHeader>
        <Form
          method="post"
          className="space-y-4"
          onSubmit={(event) => {
            if (!validateEmail(emailValue)) {
              event.preventDefault();
            }
          }}
        >
          <input type="hidden" name="intent" value="create_user" />
          <div className="space-y-1.5">
            <Label htmlFor="create-email">{t("usersPage.emailRequiredLabel")}</Label>
            <Input
              id="create-email"
              name="email"
              type="email"
              required
              aria-required="true"
              placeholder={t("usersPage.emailPlaceholder")}
              value={emailValue}
              onChange={(event) => {
                setEmailValue(event.target.value);
                if (emailError) {
                  validateEmail(event.target.value);
                }
              }}
              onBlur={(event) => validateEmail(event.target.value)}
              className={emailError ? "border-[var(--accent-red)]" : ""}
              aria-invalid={emailError ? true : undefined}
              aria-errormessage={emailError ? "create-email-error" : undefined}
            />
            {emailError && (
              <p id="create-email-error" role="alert" className="text-sm text-[var(--accent-red)]">
                {emailError}
              </p>
            )}
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="create-name">{t("usersPage.displayName")}</Label>
            <Input id="create-name" name="display_name" placeholder={t("usersPage.displayNamePlaceholder")} />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="create-password">{t("usersPage.passwordRequiredLabel")}</Label>
            <Input
              id="create-password"
              name="password"
              type="password"
              required
              aria-required="true"
              placeholder={t("usersPage.passwordPlaceholder")}
            />
          </div>
          <div className="space-y-1.5">
            <Label id="create-tenant-label">{t("usersPage.tenantOptional")}</Label>
            <Select name="tenant_id" defaultValue={activeTenantId} aria-labelledby="create-tenant-label">
              <SelectTrigger aria-labelledby="create-tenant-label">
                <SelectValue placeholder={t("usersPage.noTenant")} />
              </SelectTrigger>
              <SelectContent>
                {tenants.map((tenant) => (
                  <SelectItem key={tenant.id} value={tenant.id}>
                    {tenant.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          {error && (
            <p role="alert" className="text-sm text-[var(--accent-red)]">
              {error}
            </p>
          )}
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              className="bg-[var(--glass-bg)]"
              onClick={() => {
                resetFormState();
                onOpenChange(false);
              }}
            >
              {t("common.buttons.cancel")}
            </Button>
            <Button type="submit" disabled={isSubmitting}>
              {t("usersPage.createUserSubmit")}
            </Button>
          </DialogFooter>
        </Form>
      </DialogContent>
    </Dialog>
  );
}
