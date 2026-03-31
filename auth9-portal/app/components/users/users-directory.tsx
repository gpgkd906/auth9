import {
  DotsHorizontalIcon,
  ExitIcon,
  Pencil2Icon,
  PersonIcon,
  PlusIcon,
  TrashIcon,
} from "@radix-ui/react-icons";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "~/components/ui/dropdown-menu";
import { Avatar, AvatarFallback } from "~/components/ui/avatar";
import { Badge } from "~/components/ui/badge";
import { Button } from "~/components/ui/button";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { FormattedDate } from "~/components/ui/formatted-date";
import { useI18n } from "~/i18n";
import type { User } from "~/services/api";

interface UsersDirectoryProps {
  currentSearch: string;
  pagination: {
    page: number;
    total: number;
    total_pages: number;
  };
  users: User[];
  onClearFilter: () => void;
  onCreateUser: () => void;
  onDeleteUser: (user: User) => void | Promise<void>;
  onEditUser: (user: User) => void;
  onForceLogout: (user: User) => void | Promise<void>;
  onManageTenants: (user: User) => void;
  onToggleMfa: (payload: { action: "enable" | "disable"; user: User }) => void;
}

export function UsersDirectory({
  currentSearch,
  pagination,
  users,
  onClearFilter,
  onCreateUser,
  onDeleteUser,
  onEditUser,
  onForceLogout,
  onManageTenants,
  onToggleMfa,
}: UsersDirectoryProps) {
  const { t } = useI18n();

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("usersPage.userDirectory")}</CardTitle>
        <CardDescription>
          {t("usersPage.pagination", {
            count: pagination.total,
            page: pagination.page,
            totalPages: pagination.total_pages,
          })}
        </CardDescription>
      </CardHeader>
      <div className="px-6 pb-6">
        <div className="mt-2 overflow-hidden rounded-xl border border-[var(--glass-border-subtle)] md:hidden">
          {users.length > 0 ? (
            <div className="space-y-3 p-3">
              {users.map((user) => (
                <div
                  key={user.id}
                  className="rounded-lg border border-[var(--glass-border-subtle)] bg-[var(--sidebar-item-hover)]/20 p-3"
                >
                  <div className="space-y-1 text-sm">
                    <p className="break-all font-semibold text-[var(--text-primary)]">{user.email}</p>
                    <p className="text-[var(--text-secondary)]">
                      {t("usersPage.name")}: {user.display_name || "-"}
                    </p>
                    <p className="text-[var(--text-secondary)]">
                      {t("usersPage.mfa")}: {user.mfa_enabled ? t("usersPage.enabled") : t("usersPage.disabled")}
                    </p>
                    <p className="text-xs text-[var(--text-tertiary)]">
                      {t("usersPage.updated")}: <FormattedDate date={user.updated_at} />
                    </p>
                  </div>
                  <div className="mt-3 grid grid-cols-2 gap-2">
                    <Button variant="outline" className="min-h-11 w-full text-[13px]" onClick={() => onManageTenants(user)}>
                      {t("usersPage.manageTenants")}
                    </Button>
                    <Button variant="secondary" className="min-h-11 w-full text-[13px]" onClick={() => onEditUser(user)}>
                      {t("usersPage.edit")}
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="flex flex-col items-center px-4 py-10 text-center">
              <PersonIcon className="h-10 w-10 text-[var(--text-tertiary)]" />
              <h3 className="mt-3 text-base font-semibold text-[var(--text-primary)]">
                {currentSearch ? t("usersPage.noUsersFound") : t("usersPage.emptyTitle")}
              </h3>
              {!currentSearch && (
                <p className="mt-1 text-sm text-[var(--text-secondary)]">{t("usersPage.emptyDescription")}</p>
              )}
              {currentSearch ? (
                <Button type="button" variant="ghost" className="mt-4 min-h-11" onClick={onClearFilter}>
                  {t("usersPage.clearFilter")}
                </Button>
              ) : (
                <Button type="button" className="mt-4" onClick={onCreateUser}>
                  <PlusIcon className="mr-2 h-4 w-4" />
                  {t("usersPage.createUser")}
                </Button>
              )}
            </div>
          )}
        </div>
        <div className="mt-2 hidden overflow-x-auto rounded-xl border border-[var(--glass-border-subtle)] md:block">
          <table className="min-w-[600px] w-full divide-y divide-[var(--glass-border-subtle)] text-sm">
            <thead className="border-b border-[var(--glass-border-subtle)] bg-[var(--sidebar-item-hover)] text-left text-[11px] uppercase tracking-[0.04em] text-[var(--text-tertiary)]">
              <tr>
                <th scope="col" className="px-4 py-3 font-semibold">{t("usersPage.email")}</th>
                <th scope="col" className="px-4 py-3 font-semibold">{t("usersPage.displayName")}</th>
                <th scope="col" className="px-4 py-3 font-semibold">{t("usersPage.mfa")}</th>
                <th scope="col" className="px-4 py-3 font-semibold">{t("usersPage.updated")}</th>
                <th scope="col" className="w-10 px-4 py-3 font-semibold"></th>
              </tr>
            </thead>
            <tbody className="divide-y divide-[var(--glass-border-subtle)]">
              {users.map((user) => (
                <tr key={user.id} className="text-[var(--text-secondary)] hover:bg-[var(--sidebar-item-hover)]/50">
                  <td className="px-4 py-3">
                    <div className="flex items-center gap-3">
                      <Avatar className="h-8 w-8 text-xs">
                        <AvatarFallback>{user.email.charAt(0).toUpperCase()}</AvatarFallback>
                      </Avatar>
                      <span className="font-medium text-[var(--text-primary)]">{user.email}</span>
                    </div>
                  </td>
                  <td className="px-4 py-3">{user.display_name || "-"}</td>
                  <td className="px-4 py-3">
                    <Badge variant={user.mfa_enabled ? "success" : "secondary"}>
                      {user.mfa_enabled ? t("usersPage.enabled") : t("usersPage.disabled")}
                    </Badge>
                  </td>
                  <td className="px-4 py-3">
                    <FormattedDate date={user.updated_at} />
                  </td>
                  <td className="px-4 py-3">
                      <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button
                          variant="ghost"
                          className="h-11 w-11 p-0 active:scale-95 sm:h-8 sm:w-8"
                          aria-label={t("usersPage.openMenu")}
                        >
                          <span className="sr-only">{t("usersPage.openMenu")}</span>
                          <DotsHorizontalIcon className="h-4 w-4" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuLabel>{t("usersPage.actions")}</DropdownMenuLabel>
                        <DropdownMenuItem onClick={() => onEditUser(user)}>
                          <Pencil2Icon className="mr-2 h-3.5 w-3.5" /> {t("usersPage.editUser")}
                        </DropdownMenuItem>
                        <DropdownMenuItem onClick={() => onManageTenants(user)}>
                          <PersonIcon className="mr-2 h-3.5 w-3.5" /> {t("usersPage.manageTenants")}
                        </DropdownMenuItem>
                        {user.mfa_enabled ? (
                          <DropdownMenuItem onClick={() => onToggleMfa({ action: "disable", user })}>
                            {t("usersPage.disableMfa")}
                          </DropdownMenuItem>
                        ) : (
                          <DropdownMenuItem onClick={() => onToggleMfa({ action: "enable", user })}>
                            {t("usersPage.enableMfa")}
                          </DropdownMenuItem>
                        )}
                        <DropdownMenuItem onClick={() => onForceLogout(user)}>
                          <ExitIcon className="mr-2 h-3.5 w-3.5" /> {t("usersPage.forceLogout")}
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem
                          className="text-[var(--accent-red)] focus:text-[var(--accent-red)]"
                          onClick={() => onDeleteUser(user)}
                        >
                          <TrashIcon className="mr-2 h-3.5 w-3.5" /> {t("common.buttons.delete")}
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </td>
                </tr>
              ))}
              {users.length === 0 && (
                <tr>
                  <td className="px-4 py-12 text-center" colSpan={5}>
                    <div className="flex flex-col items-center">
                      <PersonIcon className="h-10 w-10 text-[var(--text-tertiary)]" />
                      <h3 className="mt-3 text-base font-semibold text-[var(--text-primary)]">
                        {currentSearch ? t("usersPage.noUsersFound") : t("usersPage.emptyTitle")}
                      </h3>
                      {!currentSearch && (
                        <p className="mt-1 text-sm text-[var(--text-secondary)]">{t("usersPage.emptyDescription")}</p>
                      )}
                      {currentSearch ? (
                        <Button type="button" variant="ghost" className="mt-4 min-h-10" onClick={onClearFilter}>
                          {t("usersPage.clearFilter")}
                        </Button>
                      ) : (
                        <Button type="button" className="mt-4" onClick={onCreateUser}>
                          <PlusIcon className="mr-2 h-4 w-4" />
                          {t("usersPage.createUser")}
                        </Button>
                      )}
                    </div>
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>
    </Card>
  );
}
