import { Form } from "react-router";
import type { Tenant, User } from "~/services/api";
import { useI18n } from "~/i18n";
import { Button } from "~/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "~/components/ui/dialog";
import { Label } from "~/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "~/components/ui/select";
import type { TenantInfo, UserTenant } from "./types";
import { GearIcon } from "@radix-ui/react-icons";

interface ManageUserTenantsDialogProps {
  addToTenantError?: string | null;
  loadingTenants: boolean;
  onManageRoles: (tenant: TenantInfo) => void;
  onOpenChange: (open: boolean) => void;
  onUpdateRoleInTenant: (tenantId: string, roleInTenant: string) => void;
  tenants: Tenant[];
  tenantsError?: string | null;
  user: User | null;
  userTenants: UserTenant[];
}

export function ManageUserTenantsDialog({
  addToTenantError,
  loadingTenants,
  onManageRoles,
  onOpenChange,
  onUpdateRoleInTenant,
  tenants,
  tenantsError,
  user,
  userTenants,
}: ManageUserTenantsDialogProps) {
  const { t } = useI18n();

  return (
    <Dialog open={Boolean(user)} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t("usersPage.manageTenantsTitle", { email: user?.email || "" })}</DialogTitle>
          <DialogDescription>{t("usersPage.manageTenantsDescription")}</DialogDescription>
        </DialogHeader>

        <div className="space-y-6">
          <div className="rounded-xl border border-[var(--glass-border-subtle)] p-4">
            <h4 className="mb-4 text-sm font-medium text-[var(--text-primary)]">{t("usersPage.joinedTenants")}</h4>
            <div className="space-y-2">
              {loadingTenants && <p className="text-sm text-[var(--text-tertiary)]">{t("usersPage.loadingTenants")}</p>}
              {tenantsError && (
                <p className="text-sm text-[var(--accent-red)]">
                  {t("usersPage.loadingTenantsError", { error: tenantsError })}
                </p>
              )}
              {!loadingTenants &&
                userTenants.map((userTenant) => (
                  <div
                    key={userTenant.tenant_id}
                    className="flex items-center justify-between rounded-lg bg-[var(--sidebar-item-hover)] p-2 text-sm"
                  >
                    <div className="flex items-center gap-2">
                      {userTenant.tenant?.logo_url && (
                        <img src={userTenant.tenant.logo_url} alt="" className="h-5 w-5 rounded" />
                      )}
                      <span className="font-medium text-[var(--text-primary)]">
                        {userTenant.tenant?.name ?? t("usersPage.unknownTenant")}
                      </span>
                      {userTenant.joined_at && (
                        <span className="text-xs text-[var(--text-tertiary)]">
                          {new Date(userTenant.joined_at).toLocaleDateString()}
                        </span>
                      )}
                      <Select
                        key={`${userTenant.tenant_id}-${userTenant.role_in_tenant}`}
                        defaultValue={userTenant.role_in_tenant}
                        onValueChange={(value) => {
                          if (value !== userTenant.role_in_tenant) {
                            onUpdateRoleInTenant(userTenant.tenant_id, value);
                          }
                        }}
                      >
                        <SelectTrigger className="h-7 w-24 text-xs">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="member">{t("usersPage.member")}</SelectItem>
                          <SelectItem value="admin">{t("usersPage.admin")}</SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                    <div className="flex gap-2">
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => userTenant.tenant && onManageRoles(userTenant.tenant)}
                        disabled={!userTenant.tenant}
                      >
                        <GearIcon className="mr-2 h-3.5 w-3.5" /> {t("usersPage.roles")}
                      </Button>
                      <Form method="post" className="inline">
                        <input type="hidden" name="intent" value="remove_from_tenant" />
                        <input type="hidden" name="user_id" value={user?.id ?? ""} />
                        <input type="hidden" name="tenant_id" value={userTenant.tenant_id} />
                        <Button size="sm" variant="ghost" className="text-[var(--accent-red)] hover:text-[var(--accent-red)]">
                          {t("usersPage.remove")}
                        </Button>
                      </Form>
                    </div>
                  </div>
                ))}
              {!loadingTenants && !tenantsError && userTenants.length === 0 && (
                <p className="text-sm text-[var(--text-tertiary)]">{t("usersPage.notMemberOfAnyTenant")}</p>
              )}
            </div>
          </div>

          <div className="rounded-xl border border-[var(--glass-border-subtle)] bg-[var(--sidebar-item-hover)]/50 p-4">
            <h4 className="mb-4 text-sm font-medium text-[var(--text-primary)]">{t("usersPage.addToTenant")}</h4>
            <Form method="post" className="flex items-end gap-4">
              <input type="hidden" name="intent" value="add_to_tenant" />
              <input type="hidden" name="user_id" value={user?.id ?? ""} />
              <div className="flex-1 space-y-2">
                <Label id="add-tenant-label">{t("usersPage.tenant")}</Label>
                <Select name="tenant_id" aria-labelledby="add-tenant-label">
                  <SelectTrigger aria-labelledby="add-tenant-label">
                    <SelectValue placeholder={t("usersPage.selectTenant")} />
                  </SelectTrigger>
                  <SelectContent>
                    {tenants
                      .filter((tenant) => !userTenants.some((userTenant) => userTenant.tenant_id === tenant.id))
                      .map((tenant) => (
                        <SelectItem key={tenant.id} value={tenant.id}>
                          {tenant.name}
                        </SelectItem>
                      ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="w-32 space-y-2">
                <Label id="add-role-label">{t("usersPage.role")}</Label>
                <Select name="role_in_tenant" defaultValue="member" aria-labelledby="add-role-label">
                  <SelectTrigger aria-labelledby="add-role-label">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="member">{t("usersPage.member")}</SelectItem>
                    <SelectItem value="admin">{t("usersPage.admin")}</SelectItem>
                    <SelectItem value="viewer">{t("usersPage.viewer")}</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <Button type="submit">{t("usersPage.add")}</Button>
            </Form>
            {addToTenantError && <p className="text-sm text-[var(--accent-red)]">{addToTenantError}</p>}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
