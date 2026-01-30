import type { LoaderFunctionArgs, ActionFunctionArgs, MetaFunction } from "@remix-run/node";
import { json } from "@remix-run/node";
import { Form, useActionData, useLoaderData, useNavigation, useSubmit } from "@remix-run/react";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { userApi, tenantApi, rbacApi, serviceApi, type User, type Tenant, type Service, type Role } from "~/services/api";

// Type for user-tenant relationship from userApi.getTenants
interface UserTenant {
  id: string;
  tenant_id: string;
  role_in_tenant: string;
  joined_at: string;
  tenant: Tenant;
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
  DropdownMenuTrigger,
} from "~/components/ui/dropdown-menu";
import { DotsHorizontalIcon, Pencil2Icon, PersonIcon, GearIcon } from "@radix-ui/react-icons";
import { useEffect, useState } from "react";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "~/components/ui/select";
import { Checkbox } from "~/components/ui/checkbox";

export const meta: MetaFunction = () => {
  return [{ title: "Users - Auth9" }];
};

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const perPage = Number(url.searchParams.get("perPage") || "20");
  const [users, tenants, services] = await Promise.all([
    userApi.list(page, perPage),
    tenantApi.list(1, 100), // List first 100 tenants for now
    serviceApi.list(undefined, 1, 100) // List first 100 services
  ]);
  return json({ users, tenants, services });
}

export async function action({ request }: ActionFunctionArgs) {
  const formData = await request.formData();
  const intent = formData.get("intent");

  try {
    if (intent === "update_user") {
      const id = formData.get("id") as string;
      const display_name = formData.get("display_name") as string;
      await userApi.update(id, { display_name });
      return json({ success: true, intent });
    }

    if (intent === "add_to_tenant") {
      const user_id = formData.get("user_id") as string;
      const tenant_id = formData.get("tenant_id") as string;
      const role_in_tenant = formData.get("role_in_tenant") as string;
      await userApi.addToTenant(user_id, tenant_id, role_in_tenant);
      return json({ success: true, intent });
    }

    if (intent === "remove_from_tenant") {
      const user_id = formData.get("user_id") as string;
      const tenant_id = formData.get("tenant_id") as string;
      await userApi.removeFromTenant(user_id, tenant_id);
      return json({ success: true, intent });
    }

    if (intent === "assign_roles") {
      const user_id = formData.get("user_id") as string;
      const tenant_id = formData.get("tenant_id") as string;
      const roles_json = formData.get("roles") as string;
      const roles = JSON.parse(roles_json);

      await rbacApi.assignRoles({
        user_id,
        tenant_id,
        roles
      });
      return json({ success: true, intent });
    }

    if (intent === "create_user") {
      const email = formData.get("email") as string;
      const display_name = formData.get("display_name") as string;
      const password = formData.get("password") as string;
      await userApi.create({ email, display_name, password });
      return json({ success: true, intent });
    }

    if (intent === "unassign_role") {
      const user_id = formData.get("user_id") as string;
      const tenant_id = formData.get("tenant_id") as string;
      const role_id = formData.get("role_id") as string;
      await rbacApi.unassignRole(user_id, tenant_id, role_id);
      return json({ success: true, intent });
    }

  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown error";
    return json({ error: message, intent }, { status: 400 });
  }

  return json({ error: "Invalid intent", intent }, { status: 400 });
}

export default function UsersPage() {
  const { users, tenants, services } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const submit = useSubmit();

  const [editingUser, setEditingUser] = useState<User | null>(null);
  const [creatingUser, setCreatingUser] = useState(false);
  const [managingTenantsUser, setManagingTenantsUser] = useState<User | null>(null);
  const [managingRoles, setManagingRoles] = useState<{ user: User, tenant: Tenant } | null>(null);

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
      // For tenant/role management, we might want to keep the dialog open or refresh data
      // For now, let's close role dialog but keep tenant dialog unless specifically closed
      if (actionData.intent === "assign_roles") setManagingRoles(null);
      // Refresh assigned roles after unassign
      if (actionData.intent === "unassign_role" && managingRoles) {
        rbacApi.getUserAssignedRoles(managingRoles.user.id, managingRoles.tenant.id)
          .then(res => {
            setAllAssignedRoles(res.data);
            const ids = new Set(res.data.map((r: Role) => r.id));
            setAssignedRoleIds(ids);
          });
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [actionData]);

  // Fetch user tenants when opening Manage Tenants dialog
  const [userTenants, setUserTenants] = useState<UserTenant[]>([]);
  useEffect(() => {
    if (managingTenantsUser) {
      userApi.getTenants(managingTenantsUser.id).then(res => setUserTenants(res.data));
    }
  }, [managingTenantsUser, actionData]); // Refresh on actionData change (e.g. after add/remove)

  // Fetch roles when opening Manage Roles dialog
  // 1. Fetch all assigned roles for this user in this tenant
  useEffect(() => {
    if (managingRoles) {
      rbacApi.getUserAssignedRoles(managingRoles.user.id, managingRoles.tenant.id)
        .then(res => {
          setAllAssignedRoles(res.data);
          const ids = new Set(res.data.map((r: Role) => r.id));
          setAssignedRoleIds(ids);
        });
    }
  }, [managingRoles]);

  // 2. Fetch available roles when service is selected
  useEffect(() => {
    if (selectedServiceId) {
      rbacApi.listRoles(selectedServiceId).then(res => setAvailableRoles(res.data));
    } else {
      setAvailableRoles([]);
    }
  }, [selectedServiceId]);

  const handleAssignRoles = () => {
    if (!managingRoles) return;

    // We only update roles for the selected service.
    // However, the backend `assignRoles` might replace ALL roles or just add/append?
    // Looking at backend `assignRolesToUser`: it INSERT IGNOREs. It does NOT clear existing roles.
    // Wait, how do we remove roles?
    // Implementation Plan didn't specifying removing roles via `assignRoles`.
    // Existing backend `assign_roles_to_user` in `RbacRepositoryImpl` only INSERTs. 
    // It does not delete.
    // `remove_role_from_user` exists in repo but not in `assign_roles` flow.
    // And `api/role.rs` `assign_roles` endpoint calls `rbac_service.assign_roles`.

    // Oh, I see. `AssignRolesInput` is for granting.
    // To revoke, we need a separate API or `assign_roles` should be "set roles".
    // Currently `RbacService::assign_roles` adds roles.
    // There is no endpoint to remove roles exposed in `api/role.rs` except `assign_roles` (which adds).
    // Actually, I verified `api/role.rs` earlier. 
    // There is NO `remove_role_from_user` endpoint exposed for User-Tenant-Role assignment!
    // `api/role.rs` has:
    // `assign_permission` / `remove_permission` (for Role-Permission)
    // `assign_roles` (User-Tenant-Role) -> adds.

    // I missed this gap. I cannot unassign roles with current backend!
    // I should add `remove_role_from_user` endpoint or make `assign_roles` replace.
    // Making it replace is better for UI "checkboxes".
    // But `assign_roles_to_user` in repo loops and inserts.

    // I will stick to "Adding" roles for now as per current backend capabilities, 
    // OR quickly add `unassign_role` endpoint.
    // Let's add `unassign_role` endpoint to `auth9-core` as well.
    // It is critical for "User Management Interaction" to be able to remove roles.

    console.error("Backend does not support removing roles yet. Only addition is possible.");
    // I will implement Addition for now, and if I have time/tokens, add removal.
    // The user approved "Edit/Assign Tenant/Assign Roles". 
    // "Assign" usually implies adding. "Manage" implies both.
    // I will proceed with Addition.

    const rolesToAdd = Array.from(assignedRoleIds).filter(id =>
      // Only send IDs that are currently selected AND belong to the currently selected Service
      // (to differentiate from other services' roles in `assignedRoleIds`)
      availableRoles.some(r => r.id === id)
    );

    // Filter out roles that are already assigned (though backend uses INSERT IGNORE so it's fine)

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

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold text-gray-900">Users</h1>
          <p className="text-sm text-gray-500">Manage users and tenant assignments</p>
        </div>
        <Button onClick={() => setCreatingUser(true)}>+ Create User</Button>
      </div>
      <Card>
        <CardHeader>
          <CardTitle>User Directory</CardTitle>
          <CardDescription>
            {users.pagination.total} users • Page {users.pagination.page} of{" "}
            {users.pagination.total_pages}
          </CardDescription>
        </CardHeader>
        <div className="px-6 pb-6">
          <div className="overflow-hidden rounded-apple border border-gray-100">
            <table className="min-w-full divide-y divide-gray-100 text-sm">
              <thead className="bg-gray-50 text-left text-gray-500">
                <tr>
                  <th className="px-4 py-3 font-medium">Email</th>
                  <th className="px-4 py-3 font-medium">Display Name</th>
                  <th className="px-4 py-3 font-medium">MFA</th>
                  <th className="px-4 py-3 font-medium">Updated</th>
                  <th className="px-4 py-3 font-medium w-10"></th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {users.data.map((user: User) => (
                  <tr key={user.id} className="text-gray-700 hover:bg-gray-50/50">
                    <td className="px-4 py-3 font-medium text-gray-900">{user.email}</td>
                    <td className="px-4 py-3">{user.display_name || "-"}</td>
                    <td className="px-4 py-3">{user.mfa_enabled ? "Enabled" : "Disabled"}</td>
                    <td className="px-4 py-3">
                      {new Date(user.updated_at).toLocaleString()}
                    </td>
                    <td className="px-4 py-3">
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" className="h-8 w-8 p-0">
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
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </td>
                  </tr>
                ))}
                {users.data.length === 0 && (
                  <tr>
                    <td className="px-4 py-6 text-center text-gray-500" colSpan={5}>
                      No users found
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
          </DialogHeader>
          <Form method="post" className="space-y-4">
            <input type="hidden" name="intent" value="update_user" />
            <input type="hidden" name="id" value={editingUser?.id || ""} />
            <div className="space-y-2">
              <Label htmlFor="edit-name">Display Name</Label>
              <Input
                id="edit-name"
                name="display_name"
                defaultValue={editingUser?.display_name || ""}
              />
            </div>
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setEditingUser(null)}>
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
      <Dialog open={creatingUser} onOpenChange={(open) => !open && setCreatingUser(false)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create User</DialogTitle>
            <DialogDescription>
              Create a new user account.
            </DialogDescription>
          </DialogHeader>
          <Form method="post" className="space-y-4">
            <input type="hidden" name="intent" value="create_user" />
            <div className="space-y-2">
              <Label htmlFor="create-email">Email *</Label>
              <Input
                id="create-email"
                name="email"
                type="email"
                required
                placeholder="user@example.com"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="create-name">Display Name</Label>
              <Input
                id="create-name"
                name="display_name"
                placeholder="John Doe"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="create-password">Password *</Label>
              <Input
                id="create-password"
                name="password"
                type="password"
                required
                placeholder="Enter a strong password"
              />
            </div>
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setCreatingUser(false)}>
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
            <div className="rounded-md border p-4">
              <h4 className="mb-4 text-sm font-medium">Joined Tenants</h4>
              <div className="space-y-2">
                {userTenants.map((ut: UserTenant) => (
                  <div key={ut.tenant_id} className="flex items-center justify-between rounded-md bg-gray-50 p-2 text-sm">
                    <div className="flex items-center gap-2">
                      {ut.tenant.logo_url && <img src={ut.tenant.logo_url} alt="" className="h-5 w-5 rounded" />}
                      <span className="font-medium">{ut.tenant.name}</span>
                      <span className="text-gray-500">({ut.role_in_tenant})</span>
                    </div>
                    <div className="flex gap-2">
                      <Button size="sm" variant="outline" onClick={() => managingTenantsUser && setManagingRoles({ user: managingTenantsUser, tenant: ut.tenant })}>
                        <GearIcon className="mr-2 h-3.5 w-3.5" /> Roles
                      </Button>
                      <Form method="post" className="inline">
                        <input type="hidden" name="intent" value="remove_from_tenant" />
                        <input type="hidden" name="user_id" value={managingTenantsUser?.id} />
                        <input type="hidden" name="tenant_id" value={ut.tenant_id} />
                        <Button size="sm" variant="ghost" className="text-red-500 hover:text-red-600">
                          Remove
                        </Button>
                      </Form>
                    </div>
                  </div>
                ))}
                {userTenants.length === 0 && <p className="text-sm text-gray-500">Not a member of any tenant.</p>}
              </div>
            </div>

            <div className="rounded-md border p-4 bg-gray-50/50">
              <h4 className="mb-4 text-sm font-medium">Add to Tenant</h4>
              <Form method="post" className="flex gap-4 items-end">
                <input type="hidden" name="intent" value="add_to_tenant" />
                <input type="hidden" name="user_id" value={managingTenantsUser?.id} />
                <div className="flex-1 space-y-2">
                  <Label>Tenant</Label>
                  <Select name="tenant_id">
                    <SelectTrigger>
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
                  <Label>Role</Label>
                  <Select name="role_in_tenant" defaultValue="member">
                    <SelectTrigger>
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
              <Label>Service</Label>
              <Select onValueChange={setSelectedServiceId}>
                <SelectTrigger>
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
              <div className="space-y-2 max-h-64 overflow-y-auto border p-2 rounded">
                {availableRoles.length === 0 ? (
                  <p className="text-sm text-gray-500">No roles defined for this service.</p>
                ) : (
                  availableRoles.map((role: Role) => {
                    const isAssigned = assignedRoleIds.has(role.id);
                    const wasOriginallyAssigned = allAssignedRoles.some((r: Role) => r.id === role.id);
                    return (
                      <div key={role.id} className="flex items-center space-x-2">
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
                          className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                        >
                          {role.name}
                          {role.description && <span className="ml-2 text-gray-400 font-normal">{role.description}</span>}
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
