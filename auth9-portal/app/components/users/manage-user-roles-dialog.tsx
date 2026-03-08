import type { Dispatch, SetStateAction } from "react";
import type { Role, Service, User } from "~/services/api";
import { useI18n } from "~/i18n";
import { Button } from "~/components/ui/button";
import { Checkbox } from "~/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "~/components/ui/dialog";
import { Label } from "~/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "~/components/ui/select";
import type { TenantInfo } from "./types";

interface ManagingRolesState {
  user: User;
  tenant: TenantInfo;
}

interface ManageUserRolesDialogProps {
  allAssignedRoles: Role[];
  availableRoles: Role[];
  isSubmitting: boolean;
  managingRoles: ManagingRolesState | null;
  assignedRoleIds: Set<string>;
  selectedServiceId: string;
  services: Service[];
  setSelectedServiceId: Dispatch<SetStateAction<string>>;
  onAssignRoles: () => void;
  onOpenChange: (open: boolean) => void;
  onRoleCheckedChange: (roleId: string, checked: boolean, wasOriginallyAssigned: boolean) => void;
}

export function ManageUserRolesDialog({
  allAssignedRoles,
  availableRoles,
  isSubmitting,
  managingRoles,
  assignedRoleIds,
  selectedServiceId,
  services,
  setSelectedServiceId,
  onAssignRoles,
  onOpenChange,
  onRoleCheckedChange,
}: ManageUserRolesDialogProps) {
  const { t } = useI18n();

  return (
    <Dialog open={Boolean(managingRoles)} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("usersPage.assignRolesTitle")}</DialogTitle>
          <DialogDescription>
            {t("usersPage.assignRolesDescription", { tenant: managingRoles?.tenant.name || "" })}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div className="space-y-2">
            <Label id="role-service-label">{t("usersPage.service")}</Label>
            <Select value={selectedServiceId} onValueChange={setSelectedServiceId} aria-labelledby="role-service-label">
              <SelectTrigger aria-labelledby="role-service-label">
                <SelectValue placeholder={t("usersPage.selectService")} />
              </SelectTrigger>
              <SelectContent>
                {services.map((service) => (
                  <SelectItem key={service.id} value={service.id}>
                    {service.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {selectedServiceId && (
            <div className="flex max-h-64 flex-col gap-3 overflow-y-auto rounded-xl border border-[var(--glass-border-subtle)] p-2">
              {availableRoles.length === 0 ? (
                <p className="text-sm text-[var(--text-tertiary)]">{t("usersPage.noRolesDefined")}</p>
              ) : (
                availableRoles.map((role) => {
                  const isAssigned = assignedRoleIds.has(role.id);
                  const wasOriginallyAssigned = allAssignedRoles.some((assignedRole) => assignedRole.id === role.id);

                  return (
                    <div key={role.id} className="flex h-10 min-h-[40px] items-center gap-3">
                      <Checkbox
                        id={role.id}
                        checked={isAssigned}
                        onCheckedChange={(checked) => onRoleCheckedChange(role.id, checked === true, wasOriginallyAssigned)}
                      />
                      <label
                        htmlFor={role.id}
                        className="text-sm font-medium leading-none text-[var(--text-primary)] peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                      >
                        {role.name}
                        {role.description && (
                          <span className="ml-2 font-normal text-[var(--text-tertiary)]">{role.description}</span>
                        )}
                      </label>
                    </div>
                  );
                })
              )}
            </div>
          )}
        </div>

        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
            {t("usersPage.done")}
          </Button>
          <Button onClick={onAssignRoles} disabled={isSubmitting || !selectedServiceId}>
            {t("usersPage.saveRoles")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
