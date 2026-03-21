import { Form, useNavigation } from "react-router";
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogCancel,
} from "~/components/ui/alert-dialog";
import { Button } from "~/components/ui/button";
import { useI18n } from "~/i18n";

interface RemoveTotpDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function RemoveTotpDialog({ open, onOpenChange }: RemoveTotpDialogProps) {
  const { t } = useI18n();
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{t("accountMfa.totp.removeTitle")}</AlertDialogTitle>
          <AlertDialogDescription>
            {t("accountMfa.totp.removeConfirm")}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>{t("common.buttons.cancel")}</AlertDialogCancel>
          <Form method="post">
            <input type="hidden" name="intent" value="remove_totp" />
            <Button type="submit" variant="destructive" disabled={isSubmitting}>
              {t("accountMfa.totp.remove")}
            </Button>
          </Form>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
