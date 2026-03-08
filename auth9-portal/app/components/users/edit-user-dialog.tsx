import { Form } from "react-router";
import type { User } from "~/services/api";
import { useI18n } from "~/i18n";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "~/components/ui/dialog";

interface EditUserDialogProps {
  isSubmitting: boolean;
  user: User | null;
  onOpenChange: (open: boolean) => void;
}

export function EditUserDialog({ isSubmitting, user, onOpenChange }: EditUserDialogProps) {
  const { t } = useI18n();

  return (
    <Dialog open={Boolean(user)} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("usersPage.editUserTitle")}</DialogTitle>
          <DialogDescription>{t("usersPage.editUserDescription")}</DialogDescription>
        </DialogHeader>
        <Form method="post" className="space-y-4">
          <input type="hidden" name="intent" value="update_user" />
          <input type="hidden" name="id" value={user?.id || ""} />
          <div className="space-y-1.5">
            <Label htmlFor="edit-name">{t("usersPage.displayName")}</Label>
            <Input id="edit-name" name="display_name" defaultValue={user?.display_name || ""} />
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" className="bg-[var(--glass-bg)]" onClick={() => onOpenChange(false)}>
              {t("common.buttons.cancel")}
            </Button>
            <Button type="submit" disabled={isSubmitting}>
              {t("usersPage.saveChanges")}
            </Button>
          </DialogFooter>
        </Form>
      </DialogContent>
    </Dialog>
  );
}
