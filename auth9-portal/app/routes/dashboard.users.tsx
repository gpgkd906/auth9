import type { MetaFunction, LoaderFunctionArgs, ActionFunctionArgs } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation, useSubmit, useNavigate, useSearchParams, useOutletContext } from "react-router";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { userApi, tenantApi, rbacApi, serviceApi, sessionApi, type User, type Tenant, type Service, type Role, type TenantUserWithTenant } from "~/services/api";
import { getAccessToken } from "~/services/session.server";
import { formatErrorMessage } from "~/lib/error-messages";
import { FormattedDate } from "~/components/ui/formatted-date";

// Type for tenant info embedded in user-tenant response
interface TenantInfo {
  id: string;
  name: string;
  slug: string;
  logo_url?: string;
  status: string;
}

// Type for user-tenant relationship from userApi.getTenants
interface UserTenant {
  id: string;
  tenant_id: string;
  user_id: string;
  role_in_tenant: string;
  joined_at: string;
  tenant: TenantInfo;
}
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
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "~/components/ui/dropdown-menu";
import { DotsHorizontalIcon, Pencil2Icon, PersonIcon, GearIcon, TrashIcon, ExitIcon } from "@radix-ui/react-icons";
import { useEffect, useState } from "react";
import { useConfirm } from "~/hooks/useConfirm";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "~/components/ui/select";
import { Checkbox } from "~/components/ui/checkbox";

export const meta: MetaFunction = () => {
  return [{ title: "Users - Auth9" }];
};

export function HydrateFallback() {
  return null;
}

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const perPage = Number(url.searchParams.get("perPage") || "20");
  const search = url.searchParams.get("search") || undefined;
  const accessToken = await getAccessToken(request);
  const [users, tenants, services] = await Promise.all([
    userApi.list(page, perPage, search, accessToken || undefined),
    tenantApi.list(1, 100, undefined, accessToken || undefined), // List first 100 tenants for now
    serviceApi.list(undefined, 1, 100, accessToken || undefined) // List first 100 services
  ]);
  return { users, tenants, services };
}

export async function action({ request }: ActionFunctionArgs) {
  const formData = await request.formData();
  const intent = formData.get("intent");
  const accessToken = await getAccessToken(request);

  try {
    if (intent === "update_user") {
      const id = formData.get("id") as string;
      const display_name = formData.get("display_name") as string;
      await userApi.update(id, { display_name }, accessToken || undefined);
      return { success: true, intent };
    }

    if (intent === "add_to_tenant") {
      const user_id = formData.get("user_id") as string;
      const tenant_id = formData.get("tenant_id") as string;
      const role_in_tenant = formData.get("role_in_tenant") as string;
      await userApi.addToTenant(user_id, tenant_id, role_in_tenant, accessToken || undefined);
      return { success: true, intent };
    }

    if (intent === "remove_from_tenant") {
      const user_id = formData.get("user_id") as string;
      const tenant_id = formData.get("tenant_id") as string;
      await userApi.removeFromTenant(user_id, tenant_id, accessToken || undefined);
      return { success: true, intent };
    }

    if (intent === "assign_roles") {
      const user_id = formData.get("user_id") as string;
      const tenant_id = formData.get("tenant_id") as string;
      const roles_json = formData.get("roles") as string;
      const roles = JSON.parse(roles_json);

      await rbacApi.assignRoles({
        user_id,
        tenant_id,
        role_ids: roles
      }, accessToken || undefined);
      return { success: true, intent };
    }

    if (intent === "create_user") {
      const email = formData.get("email") as string;
      const display_name = formData.get("display_name") as string;
      const password = formData.get("password") as string;
      const tenant_id = formData.get("tenant_id") as string | null;
      await userApi.create(
        { email, display_name, password, ...(tenant_id ? { tenant_id } : {}) },
        accessToken || undefined
      );
      return { success: true, intent };
    }

    if (intent === "unassign_role") {
      const user_id = formData.get("user_id") as string;
      const tenant_id = formData.get("tenant_id") as string;
      const role_id = formData.get("role_id") as string;
      await rbacApi.unassignRole(user_id, tenant_id, role_id, accessToken || undefined);
      return { success: true, intent };
    }

    if (intent === "delete_user") {
      const id = formData.get("id") as string;
      await userApi.delete(id, accessToken || undefined);
      return { success: true, intent };
    }

    if (intent === "force_logout") {
      const id = formData.get("id") as string;
      await sessionApi.forceLogoutUser(id, accessToken || undefined);
      return { success: true, intent };
    }

    if (intent === "get_user_tenants") {
      const user_id = formData.get("user_id") as string;
      const tenants = await userApi.getTenants(user_id, accessToken || undefined);
      return { success: true, data: tenants.data, intent };
    }

    if (intent === "get_user_assigned_roles") {
      const user_id = formData.get("user_id") as string;
      const tenant_id = formData.get("tenant_id") as string;
      const roles = await rbacApi.getUserAssignedRoles(user_id, tenant_id, accessToken || undefined);
      return { success: true, data: roles.data, intent };
    }

    if (intent === "get_service_roles") {
      const service_id = formData.get("service_id") as string;
      const roles = await rbacApi.listRoles(service_id, accessToken || undefined);
      return { success: true, data: roles.data, intent };
    }

    if (intent === "enable_mfa") {
      const id = formData.get("id") as string;
      const confirm_password = formData.get("confirm_password") as string;
      await userApi.enableMfa(id, confirm_password, accessToken || undefined);
      return { success: true, intent };
    }

    if (intent === "disable_mfa") {
      const id = formData.get("id") as string;
      const confirm_password = formData.get("confirm_password") as string;
      await userApi.disableMfa(id, confirm_password, accessToken || undefined);
      return { success: true, intent };
    }

    if (intent === "update_role_in_tenant") {
      const user_id = formData.get("user_id") as string;
      const tenant_id = formData.get("tenant_id") as string;
      const role_in_tenant = formData.get("role_in_tenant") as string;
      await userApi.updateRoleInTenant(user_id, tenant_id, role_in_tenant, accessToken || undefined);
      return { success: true, intent };
    }

  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown error";
    return { error: message, intent };
  }

  return { error: "Invalid intent", intent };
}

export default function UsersPage() {
  const { users, tenants, services } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const submit = useSubmit();
  const confirm = useConfirm();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const { activeTenant } = useOutletContext<{ activeTenant?: TenantUserWithTenant }>();
  const activeTenantId = activeTenant?.tenant_id;

  const currentSearch = searchParams.get("search") || "";
  const [searchInput, setSearchInput] = useState(currentSearch);
  const [editingUser, setEditingUser] = useState<User | null>(null);
  const [creatingUser, setCreatingUser] = useState(false);
  const [managingTenantsUser, setManagingTenantsUser] = useState<User | null>(null);
  const [managingRoles, setManagingRoles] = useState<{ user: User, tenant: TenantInfo } | null>(null);

  // Email validation state for Create User dialog
  const [createEmailError, setCreateEmailError] = useState<string | null>(null);
  const [createEmailValue, setCreateEmailValue] = useState("");

  const validateEmail = (email: string): boolean => {
    if (!email) {
      setCreateEmailError("Email is required");
      return false;
    }
    if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
      setCreateEmailError("Please enter a valid email address");
      return false;
    }
    setCreateEmailError(null);
    return true;
  };

  // State for MFA password confirmation dialog
  const [mfaAction, setMfaAction] = useState<{ user: User; action: "enable" | "disable" } | null>(null);
  const [mfaPassword, setMfaPassword] = useState("");
  const [mfaError, setMfaError] = useState<string | null>(null);

  // State for Manage Roles
  const [selectedServiceId, setSelectedServiceId] = useState<string>("");
  const [availableRoles, setAvailableRoles] = useState<Role[]>([]);
  const [assignedRoleIds, setAssignedRoleIds] = useState<Set<string>>(new Set());
  const [allAssignedRoles, setAllAssignedRoles] = useState<Role[]>([]);

  const isSubmitting = navigation.state === "submitting";

  // Close dialogs on success
  useEffect(() => {
    if (actionData && "success" in actionData && actionData.success) {
      if (actionData.intent === "update_user") setEditingUser(null);
      if (actionData.intent === "create_user") setCreatingUser(false);
      if (actionData.intent === "assign_roles") setManagingRoles(null);
      if (actionData.intent === "enable_mfa" || actionData.intent === "disable_mfa") {
        setMfaAction(null);
        setMfaPassword("");
        setMfaError(null);
      }
      // Refresh tenant list after role update
      if (actionData.intent === "update_role_in_tenant" && managingTenantsUser) {
        const formData = new FormData();
        formData.append("intent", "get_user_tenants");
        formData.append("user_id", managingTenantsUser.id);
        submit(formData, { method: "post" });
      }
      // Refresh assigned roles after unassign via server-side action
      if (actionData.intent === "unassign_role" && managingRoles) {
        const formData = new FormData();
        formData.append("intent", "get_user_assigned_roles");
        formData.append("user_id", managingRoles.user.id);
        formData.append("tenant_id", managingRoles.tenant.id);
        submit(formData, { method: "post" });
      }
      // Handle server-side role data responses
      if (actionData.intent === "get_user_assigned_roles" && actionData.data) {
        const roles = actionData.data as Role[];
        setAllAssignedRoles(roles);
        setAssignedRoleIds(new Set(roles.map((r: Role) => r.id)));
      }
      if (actionData.intent === "get_service_roles" && actionData.data) {
        setAvailableRoles(actionData.data as Role[]);
      }
    }
    // Show MFA errors in the confirmation dialog
    if (actionData && "error" in actionData && (actionData.intent === "enable_mfa" || actionData.intent === "disable_mfa")) {
      setMfaError(formatErrorMessage(String(actionData.error)));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [actionData]);

  // Fetch user tenants when opening Manage Tenants dialog
  const [userTenants, setUserTenants] = useState<UserTenant[]>([]);
  const [loadingTenants, setLoadingTenants] = useState(false);
  const [tenantsError, setTenantsError] = useState<string | null>(null);

  // Load user tenants when dialog opens
  useEffect(() => {
    if (managingTenantsUser) {
      setLoadingTenants(true);
      setTenantsError(null);
      const formData = new FormData();
      formData.append("intent", "get_user_tenants");
      formData.append("user_id", managingTenantsUser.id);
      submit(formData, { method: "post" });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [managingTenantsUser]);

  // Update userTenants when action returns data
  useEffect(() => {
    if (actionData && actionData.intent === "get_user_tenants") {
      setLoadingTenants(false);
      if ("success" in actionData && actionData.success) {
        setUserTenants((actionData.data as UserTenant[]) || []);
      } else if ("error" in actionData) {
        setTenantsError(String(actionData.error));
      }
    }
  }, [actionData]);

  // Fetch assigned roles when opening Manage Roles dialog (server-side)
  useEffect(() => {
    if (managingRoles) {
      const formData = new FormData();
      formData.append("intent", "get_user_assigned_roles");
      formData.append("user_id", managingRoles.user.id);
      formData.append("tenant_id", managingRoles.tenant.id);
      submit(formData, { method: "post" });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [managingRoles]);

  // Fetch available roles when service is selected (server-side)
  useEffect(() => {
    if (selectedServiceId) {
      const formData = new FormData();
      formData.append("intent", "get_service_roles");
      formData.append("service_id", selectedServiceId);
      submit(formData, { method: "post" });
    } else {
      setAvailableRoles([]);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedServiceId]);

  const handleAssignRoles = () => {
    if (!managingRoles) return;

    const rolesToAdd = Array.from(assignedRoleIds).filter(id =>
      availableRoles.some(r => r.id === id)
    );

    submit(
      {
        intent: "assign_roles",
        user_id: managingRoles.user.id,
        tenant_id: managingRoles.tenant.id,
        roles: JSON.stringify(rolesToAdd)
      },
      { method: "post" }
    );
  };

  const handleSearchSubmit = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const params = new URLSearchParams();
    if (searchInput.trim()) {
      params.set("search", searchInput);
    }
    params.set("page", "1");
    navigate(`/dashboard/users?${params.toString()}`);
  };

  return (
    <div className="space-y-6">

      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h1 className="mb-2 text-[24px] font-semibold text-[var(--text-primary)] tracking-tight">Users</h1>
          <p className="text-sm text-[var(--text-secondary)]">Manage users and tenant assignments</p>
        </div>
        <Button onClick={() => setCreatingUser(true)} className="w-full min-h-11 sm:w-auto sm:min-h-10">+ Create User</Button>
      </div>

      <Form onSubmit={handleSearchSubmit} className="flex gap-2">
        <Input
          type="text"
          placeholder="Search by email or name..."
          aria-label="Search users"
          value={searchInput}
          onChange={(e) => setSearchInput(e.target.value)}
          className="flex-1"
        />
        <Button type="submit" variant="outline" className="min-h-11 sm:min-h-10">Search</Button>
      </Form>

      <Card>
        <CardHeader>
          <CardTitle>User Directory</CardTitle>
          <CardDescription>
            {users.pagination.total} users • Page {users.pagination.page} of{" "}
            {users.pagination.total_pages}
          </CardDescription>
        </CardHeader>
        <div className="px-6 pb-6">
          <div className="mt-2 overflow-hidden rounded-xl border border-[var(--glass-border-subtle)] md:hidden">
            {users.data.length > 0 ? (
              <div className="space-y-3 p-3">
                {users.data.map((user: User) => (
                  <div key={user.id} className="rounded-lg border border-[var(--glass-border-subtle)] bg-[var(--sidebar-item-hover)]/20 p-3">
                    <div className="space-y-1 text-sm">
                      <p className="font-semibold text-[var(--text-primary)] break-all">{user.email}</p>
                      <p className="text-[var(--text-secondary)]">Name: {user.display_name || "-"}</p>
                      <p className="text-[var(--text-secondary)]">MFA: {user.mfa_enabled ? "Enabled" : "Disabled"}</p>
                      <p className="text-[var(--text-tertiary)] text-xs">
                        Updated: <FormattedDate date={user.updated_at} />
                      </p>
                    </div>
                    <div className="mt-3 grid grid-cols-2 gap-2">
                      <Button
                        variant="outline"
                        className="w-full min-h-11 text-[13px]"
                        onClick={() => setManagingTenantsUser(user)}
                      >
                        Manage Tenants
                      </Button>
                      <Button
                        variant="secondary"
                        className="w-full min-h-11 text-[13px]"
                        onClick={() => setEditingUser(user)}
                      >
                        Edit
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <div className="flex flex-col items-center px-4 py-6 text-center text-[var(--text-tertiary)]">
                <p>No users found</p>
                {currentSearch && (
                  <Button
                    type="button"
                    variant="ghost"
                    className="mt-4 min-h-11"
                    onClick={() => {
                      setSearchInput("");
                      navigate("/dashboard/users?page=1");
                    }}
                  >
                    Clear Filter
                  </Button>
                )}
              </div>
            )}
          </div>
          <div className="mt-2 hidden overflow-x-auto rounded-xl border border-[var(--glass-border-subtle)] md:block">
            <table className="min-w-[600px] w-full divide-y divide-[var(--glass-border-subtle)] text-sm">
              <thead className="bg-[var(--sidebar-item-hover)] text-left text-[var(--text-tertiary)] uppercase tracking-[0.04em] text-[11px] border-b border-[var(--glass-border-subtle)]">
                <tr>
                  <th className="px-4 py-3 font-semibold">Email</th>
                  <th className="px-4 py-3 font-semibold">Display Name</th>
                  <th className="px-4 py-3 font-semibold">MFA</th>
                  <th className="px-4 py-3 font-semibold">Updated</th>
                  <th className="px-4 py-3 font-semibold w-10"></th>
                </tr>
              </thead>
              <tbody className="divide-y divide-[var(--glass-border-subtle)]">
                {users.data.map((user: User) => (
                  <tr key={user.id} className="text-[var(--text-secondary)] hover:bg-[var(--sidebar-item-hover)]/50">
                    <td className="px-4 py-3 font-medium text-[var(--text-primary)]">{user.email}</td>
                    <td className="px-4 py-3">{user.display_name || "-"}</td>
                    <td className="px-4 py-3">{user.mfa_enabled ? "Enabled" : "Disabled"}</td>
                    <td className="px-4 py-3">
                      <FormattedDate date={user.updated_at} />
                    </td>
                    <td className="px-4 py-3">
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" className="h-11 w-11 p-0 sm:h-8 sm:w-8 active:scale-95">
                            <span className="sr-only">Open menu</span>
                            <DotsHorizontalIcon className="h-4 w-4" />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuLabel>Actions</DropdownMenuLabel>
                          <DropdownMenuItem onClick={() => setEditingUser(user)}>
                            <Pencil2Icon className="mr-2 h-3.5 w-3.5" /> Edit User
                          </DropdownMenuItem>
                          <DropdownMenuItem onClick={() => setManagingTenantsUser(user)}>
                            <PersonIcon className="mr-2 h-3.5 w-3.5" /> Manage Tenants
                          </DropdownMenuItem>
                          {user.mfa_enabled ? (
                            <DropdownMenuItem
                              onClick={() => {
                                setMfaAction({ user, action: "disable" });
                                setMfaPassword("");
                                setMfaError(null);
                              }}
                            >
                              Disable MFA
                            </DropdownMenuItem>
                          ) : (
                            <DropdownMenuItem
                              onClick={() => {
                                setMfaAction({ user, action: "enable" });
                                setMfaPassword("");
                                setMfaError(null);
                              }}
                            >
                              Enable MFA
                            </DropdownMenuItem>
                          )}
                          <DropdownMenuItem
                            onClick={async () => {
                              const ok = await confirm({
                                title: "Force Logout",
                                description: "Force logout this user from all active sessions?",
                                confirmLabel: "Force Logout",
                              });
                              if (ok) {
                                submit({ intent: "force_logout", id: user.id }, { method: "post" });
                              }
                            }}
                          >
                            <ExitIcon className="mr-2 h-3.5 w-3.5" /> Force Logout
                          </DropdownMenuItem>
                          <DropdownMenuSeparator />
                          <DropdownMenuItem
                            className="text-[var(--accent-red)] focus:text-[var(--accent-red)]"
                            onClick={async () => {
                              const ok = await confirm({
                                title: "Delete User",
                                description: "Are you sure you want to delete this user? This action cannot be undone.",
                                variant: "destructive",
                              });
                              if (ok) {
                                submit({ intent: "delete_user", id: user.id }, { method: "post" });
                              }
                            }}
                          >
                            <TrashIcon className="mr-2 h-3.5 w-3.5" /> Delete
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </td>
                  </tr>
                ))}
                {users.data.length === 0 && (
                  <tr>
                    <td className="px-4 py-6 text-center text-[var(--text-tertiary)]" colSpan={5}>
                      <div className="flex flex-col items-center">
                        <p>No users found</p>
                        {currentSearch && (
                          <Button
                            type="button"
                            variant="ghost"
                            className="mt-4 min-h-10"
                            onClick={() => {
                              setSearchInput("");
                              navigate("/dashboard/users?page=1");
                            }}
                          >
                            Clear Filter
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

      {/* Edit User Dialog */}
      <Dialog open={!!editingUser} onOpenChange={(open) => !open && setEditingUser(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit User</DialogTitle>
            <DialogDescription>
              Update the user&apos;s profile details.
            </DialogDescription>
          </DialogHeader>
          <Form method="post" className="space-y-4">
            <input type="hidden" name="intent" value="update_user" />
            <input type="hidden" name="id" value={editingUser?.id || ""} />
            <div className="space-y-1.5">
              <Label htmlFor="edit-name">Display Name</Label>
              <Input
                id="edit-name"
                name="display_name"
                defaultValue={editingUser?.display_name || ""}
              />
            </div>
            <DialogFooter>
              <Button type="button" variant="outline" className="bg-[var(--glass-bg)]" onClick={() => setEditingUser(null)}>
                Cancel
              </Button>
              <Button type="submit" disabled={isSubmitting}>
                Save Changes
              </Button>
            </DialogFooter>
          </Form>
        </DialogContent>
      </Dialog>

      {/* Create User Dialog */}
      <Dialog open={creatingUser} onOpenChange={(open) => {
        if (!open) {
          setCreatingUser(false);
          setCreateEmailError(null);
          setCreateEmailValue("");
        }
      }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create User</DialogTitle>
            <DialogDescription>
              Create a new user account.
            </DialogDescription>
          </DialogHeader>
          <Form method="post" className="space-y-4" onSubmit={(e) => {
            if (!validateEmail(createEmailValue)) {
              e.preventDefault();
            }
          }}>
            <input type="hidden" name="intent" value="create_user" />
            <div className="space-y-1.5">
              <Label htmlFor="create-email">Email *</Label>
              <Input
                id="create-email"
                name="email"
                type="email"
                required
                placeholder="user@example.com"
                value={createEmailValue}
                onChange={(e) => {
                  setCreateEmailValue(e.target.value);
                  if (createEmailError) validateEmail(e.target.value);
                }}
                onBlur={(e) => validateEmail(e.target.value)}
                className={createEmailError ? "border-[var(--accent-red)]" : ""}
              />
              {createEmailError && (
                <p className="text-sm text-[var(--accent-red)]">{createEmailError}</p>
              )}
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="create-name">Display Name</Label>
              <Input
                id="create-name"
                name="display_name"
                placeholder="John Doe"
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="create-password">Password *</Label>
              <Input
                id="create-password"
                name="password"
                type="password"
                required
                placeholder="Enter a strong password"
              />
            </div>
            <div className="space-y-1.5">
              <Label id="create-tenant-label">Tenant (optional)</Label>
              <Select name="tenant_id" defaultValue={activeTenantId} aria-labelledby="create-tenant-label">
                <SelectTrigger aria-labelledby="create-tenant-label">
                  <SelectValue placeholder="No tenant (platform user)" />
                </SelectTrigger>
                <SelectContent>
                  {tenants.data.map((t: Tenant) => (
                    <SelectItem key={t.id} value={t.id}>{t.name}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            {actionData && "error" in actionData && actionData.intent === "create_user" && (
              <p className="text-sm text-[var(--accent-red)]">{formatErrorMessage(String(actionData.error))}</p>
            )}
            <DialogFooter>
              <Button type="button" variant="outline" className="bg-[var(--glass-bg)]" onClick={() => {
                setCreatingUser(false);
                setCreateEmailError(null);
                setCreateEmailValue("");
              }}>
                Cancel
              </Button>
              <Button type="submit" disabled={isSubmitting}>
                Create User
              </Button>
            </DialogFooter>
          </Form>
        </DialogContent>
      </Dialog>

      {/* Manage Tenants Dialog */}
      <Dialog open={!!managingTenantsUser} onOpenChange={(open) => !open && setManagingTenantsUser(null)}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Manage Tenants for {managingTenantsUser?.email}</DialogTitle>
            <DialogDescription>
              Assign user to tenants and manage roles.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-6">
            <div className="rounded-xl border border-[var(--glass-border-subtle)] p-4">
              <h4 className="mb-4 text-sm font-medium text-[var(--text-primary)]">Joined Tenants</h4>
              <div className="space-y-2">
                {loadingTenants && (
                  <p className="text-sm text-[var(--text-tertiary)]">Loading tenant information...</p>
                )}
                {tenantsError && (
                  <p className="text-sm text-[var(--accent-red)]">Error loading tenants: {tenantsError}</p>
                )}
                {!loadingTenants && userTenants.map((ut: UserTenant) => (
                  <div key={ut.tenant_id} className="flex items-center justify-between rounded-lg bg-[var(--sidebar-item-hover)] p-2 text-sm">
                    <div className="flex items-center gap-2">
                      {ut.tenant?.logo_url && <img src={ut.tenant.logo_url} alt="" className="h-5 w-5 rounded" />}
                      <span className="font-medium text-[var(--text-primary)]">{ut.tenant?.name ?? "Unknown Tenant"}</span>
                      <Select
                        defaultValue={ut.role_in_tenant}
                        onValueChange={(value) => {
                          if (value !== ut.role_in_tenant && managingTenantsUser) {
                            submit(
                              {
                                intent: "update_role_in_tenant",
                                user_id: managingTenantsUser.id,
                                tenant_id: ut.tenant_id,
                                role_in_tenant: value,
                              },
                              { method: "post" }
                            );
                          }
                        }}
                      >
                        <SelectTrigger className="h-7 w-24 text-xs">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="member">Member</SelectItem>
                          <SelectItem value="admin">Admin</SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                    <div className="flex gap-2">
                      <Button size="sm" variant="outline" onClick={() => managingTenantsUser && ut.tenant && setManagingRoles({ user: managingTenantsUser, tenant: ut.tenant })} disabled={!ut.tenant}>
                        <GearIcon className="mr-2 h-3.5 w-3.5" /> Roles
                      </Button>
                      <Form method="post" className="inline">
                        <input type="hidden" name="intent" value="remove_from_tenant" />
                        <input type="hidden" name="user_id" value={managingTenantsUser?.id ?? ""} />
                        <input type="hidden" name="tenant_id" value={ut.tenant_id} />
                        <Button size="sm" variant="ghost" className="text-[var(--accent-red)] hover:text-[var(--accent-red)]">
                          Remove
                        </Button>
                      </Form>
                    </div>
                  </div>
                ))}
                {!loadingTenants && !tenantsError && userTenants.length === 0 && <p className="text-sm text-[var(--text-tertiary)]">Not a member of any tenant.</p>}
              </div>
            </div>

            <div className="rounded-xl border border-[var(--glass-border-subtle)] p-4 bg-[var(--sidebar-item-hover)]/50">
              <h4 className="mb-4 text-sm font-medium text-[var(--text-primary)]">Add to Tenant</h4>
              <Form method="post" className="flex gap-4 items-end">
                <input type="hidden" name="intent" value="add_to_tenant" />
                <input type="hidden" name="user_id" value={managingTenantsUser?.id ?? ""} />
                <div className="flex-1 space-y-2">
                  <Label id="add-tenant-label">Tenant</Label>
                  <Select name="tenant_id" aria-labelledby="add-tenant-label">
                    <SelectTrigger aria-labelledby="add-tenant-label">
                      <SelectValue placeholder="Select tenant" />
                    </SelectTrigger>
                    <SelectContent>
                      {tenants.data
                        .filter((t: Tenant) => !userTenants.some((ut: UserTenant) => ut.tenant_id === t.id))
                        .map((t: Tenant) => (
                          <SelectItem key={t.id} value={t.id}>{t.name}</SelectItem>
                        ))}
                    </SelectContent>
                  </Select>
                </div>
                <div className="w-32 space-y-2">
                  <Label id="add-role-label">Role</Label>
                  <Select name="role_in_tenant" defaultValue="member" aria-labelledby="add-role-label">
                    <SelectTrigger aria-labelledby="add-role-label">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="member">Member</SelectItem>
                      <SelectItem value="admin">Admin</SelectItem>
                      <SelectItem value="viewer">Viewer</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <Button type="submit">Add</Button>
              </Form>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      {/* MFA Password Confirmation Dialog */}
      <Dialog open={!!mfaAction} onOpenChange={(open) => {
        if (!open) {
          setMfaAction(null);
          setMfaPassword("");
          setMfaError(null);
        }
      }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{mfaAction?.action === "enable" ? "Enable" : "Disable"} MFA</DialogTitle>
            <DialogDescription>
              Enter your password to confirm {mfaAction?.action === "enable" ? "enabling" : "disabling"} MFA for {mfaAction?.user.email}.
            </DialogDescription>
          </DialogHeader>
          <Form method="post" className="space-y-4" onSubmit={() => {
            setMfaError(null);
          }}>
            <input type="hidden" name="intent" value={mfaAction?.action === "enable" ? "enable_mfa" : "disable_mfa"} />
            <input type="hidden" name="id" value={mfaAction?.user.id || ""} />
            <div className="space-y-1.5">
              <Label htmlFor="mfa-confirm-password">Your Password</Label>
              <Input
                id="mfa-confirm-password"
                name="confirm_password"
                type="password"
                required
                placeholder="Enter your password to confirm"
                value={mfaPassword}
                onChange={(e) => setMfaPassword(e.target.value)}
              />
            </div>
            {mfaError && (
              <p className="text-sm text-[var(--accent-red)]">{mfaError}</p>
            )}
            <DialogFooter>
              <Button type="button" variant="outline" className="bg-[var(--glass-bg)]" onClick={() => {
                setMfaAction(null);
                setMfaPassword("");
                setMfaError(null);
              }}>
                Cancel
              </Button>
              <Button type="submit" disabled={isSubmitting || !mfaPassword}>
                {mfaAction?.action === "enable" ? "Enable MFA" : "Disable MFA"}
              </Button>
            </DialogFooter>
          </Form>
        </DialogContent>
      </Dialog>

      {/* Manage Roles Dialog */}
      <Dialog open={!!managingRoles} onOpenChange={(open) => !open && setManagingRoles(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Assign Roles</DialogTitle>
            <DialogDescription>
              Assign roles in {managingRoles?.tenant.name}.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4">
            <div className="space-y-2">
              <Label id="role-service-label">Service</Label>
              <Select onValueChange={setSelectedServiceId} aria-labelledby="role-service-label">
                <SelectTrigger aria-labelledby="role-service-label">
                  <SelectValue placeholder="Select Service" />
                </SelectTrigger>
                <SelectContent>
                  {services.data.map((s: Service) => (
                    <SelectItem key={s.id} value={s.id}>{s.name}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {selectedServiceId && (
              <div className="flex flex-col gap-3 max-h-64 overflow-y-auto border border-[var(--glass-border-subtle)] p-2 rounded-xl">
                {availableRoles.length === 0 ? (
                  <p className="text-sm text-[var(--text-tertiary)]">No roles defined for this service.</p>
                ) : (
                  availableRoles.map((role: Role) => {
                    const isAssigned = assignedRoleIds.has(role.id);
                    const wasOriginallyAssigned = allAssignedRoles.some((r: Role) => r.id === role.id);
                    return (
                      <div key={role.id} className="flex h-10 min-h-[40px] items-center gap-3">
                        <Checkbox
                          id={role.id}
                          checked={isAssigned}
                          onCheckedChange={(checked: boolean | 'indeterminate') => {
                            const newSet = new Set(assignedRoleIds);
                            if (checked === true) {
                              newSet.add(role.id);
                            } else {
                              newSet.delete(role.id);
                              // If role was originally assigned, unassign it from backend
                              if (wasOriginallyAssigned && managingRoles) {
                                submit(
                                  {
                                    intent: "unassign_role",
                                    user_id: managingRoles.user.id,
                                    tenant_id: managingRoles.tenant.id,
                                    role_id: role.id
                                  },
                                  { method: "post" }
                                );
                              }
                            }
                            setAssignedRoleIds(newSet);
                          }}
                        />
                        <label
                          htmlFor={role.id}
                          className="text-sm font-medium leading-none text-[var(--text-primary)] peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                        >
                          {role.name}
                          {role.description && <span className="ml-2 text-[var(--text-tertiary)] font-normal">{role.description}</span>}
                        </label>
                      </div>
                    );
                  })
                )}
              </div>
            )}
          </div>

          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => setManagingRoles(null)}>
              Done
            </Button>
            <Button onClick={handleAssignRoles} disabled={isSubmitting || !selectedServiceId}>
              Save Roles
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
