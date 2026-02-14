import type { MetaFunction, LoaderFunctionArgs, ActionFunctionArgs } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation, useSubmit, useRevalidator } from "react-router";
import { PlusIcon, DotsHorizontalIcon, Pencil2Icon, TrashIcon, CheckIcon, GearIcon } from "@radix-ui/react-icons";
import { useEffect, useState } from "react";
import { useConfirm } from "~/hooks/useConfirm";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "~/components/ui/tabs";
import { Checkbox } from "~/components/ui/checkbox";
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
import { rbacApi, serviceApi, type Role, type Permission, type RoleWithPermissions } from "~/services/api";
import { getAccessToken } from "~/services/session.server";

// Extended role type with service_id for editing
interface EditableRole extends Role {
  service_id: string;
}

export const meta: MetaFunction = () => {
  return [{ title: "Roles & Permissions - Auth9" }];
};

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const perPage = Number(url.searchParams.get("perPage") || "50");
  const accessToken = await getAccessToken(request);
  const services = await serviceApi.list(undefined, page, perPage, accessToken || undefined);

  const entries = await Promise.all(
    services.data.map(async (service) => {
      const [roles, permissions] = await Promise.all([
        rbacApi.listRoles(service.id, accessToken || undefined),
        rbacApi.listPermissions(service.id, accessToken || undefined),
      ]);

      // Fetch permission counts per role for hierarchy view
      const rolePermissionCounts: Record<string, number> = {};
      await Promise.all(
        roles.data.map(async (role) => {
          try {
            const roleDetail = await rbacApi.getRole(role.id, accessToken || undefined);
            rolePermissionCounts[role.id] = roleDetail.data.permissions.length;
          } catch {
            rolePermissionCounts[role.id] = 0;
          }
        })
      );

      return { service, roles: roles.data, permissions: permissions.data, rolePermissionCounts };
    })
  );

  return { entries, pagination: services.pagination };
}

export async function action({ request }: ActionFunctionArgs) {
  const accessToken = await getAccessToken(request);
  const formData = await request.formData();
  const intent = formData.get("intent");

  try {
    // Role actions
    if (intent === "create_role") {
      const serviceId = formData.get("service_id") as string;
      const name = formData.get("name") as string;
      const description = formData.get("description") as string;
      const parentRoleId = formData.get("parent_role_id") as string;

      await rbacApi.createRole(serviceId, {
        name,
        description: description || undefined,
        parent_role_id: parentRoleId || undefined,
      }, accessToken || undefined);
      return { success: true };
    }

    if (intent === "update_role") {
      const serviceId = formData.get("service_id") as string;
      const roleId = formData.get("role_id") as string;
      const name = formData.get("name") as string;
      const description = formData.get("description") as string;
      const parentRoleId = formData.get("parent_role_id") as string;

      await rbacApi.updateRole(serviceId, roleId, {
        name,
        description: description || undefined,
        parent_role_id: parentRoleId || undefined,
      }, accessToken || undefined);
      return { success: true };
    }

    if (intent === "delete_role") {
      const serviceId = formData.get("service_id") as string;
      const roleId = formData.get("role_id") as string;
      await rbacApi.deleteRole(serviceId, roleId, accessToken || undefined);
      return { success: true };
    }

    // Permission actions
    if (intent === "create_permission") {
      const serviceId = formData.get("service_id") as string;
      const code = formData.get("code") as string;
      const name = formData.get("name") as string;
      const description = formData.get("description") as string;

      await rbacApi.createPermission({
        service_id: serviceId,
        code,
        name,
        description: description || undefined,
      }, accessToken || undefined);
      return { success: true };
    }

    if (intent === "delete_permission") {
      const permissionId = formData.get("permission_id") as string;
      await rbacApi.deletePermission(permissionId, accessToken || undefined);
      return { success: true };
    }

    // Role-Permission assignment actions
    if (intent === "assign_permission") {
      const roleId = formData.get("role_id") as string;
      const permissionId = formData.get("permission_id") as string;
      await rbacApi.assignPermissionToRole(roleId, permissionId, accessToken || undefined);
      return { success: true };
    }

    if (intent === "remove_permission") {
      const roleId = formData.get("role_id") as string;
      const permissionId = formData.get("permission_id") as string;
      await rbacApi.removePermissionFromRole(roleId, permissionId, accessToken || undefined);
      return { success: true };
    }

    // Get role with permissions
    if (intent === "get_role_permissions") {
      const roleId = formData.get("role_id") as string;
      const result = await rbacApi.getRole(roleId, accessToken || undefined);
      return { success: true, role: result.data };
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown error";
    return Response.json({ error: message }, { status: 400 });
  }

  return Response.json({ error: "Invalid intent" }, { status: 400 });
}

export default function RolesPage() {
  const data = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const submit = useSubmit();
  const confirm = useConfirm();
  const revalidator = useRevalidator();

  // Role state
  const [createRoleServiceId, setCreateRoleServiceId] = useState<string | null>(null);
  const [editingRole, setEditingRole] = useState<EditableRole | null>(null);
  const [managingPermissionsRole, setManagingPermissionsRole] = useState<{
    role: EditableRole;
    permissions: Permission[];
    rolePermissions: Permission[];
  } | null>(null);

  // Permission state
  const [createPermissionServiceId, setCreatePermissionServiceId] = useState<string | null>(null);

  const isSubmitting = navigation.state === "submitting";

  useEffect(() => {
    if (actionData && "success" in actionData && actionData.success) {
      setCreateRoleServiceId(null);
      setEditingRole(null);
      setCreatePermissionServiceId(null);

      // If we got role permissions back, update the managing state
      if ("role" in actionData && actionData.role && managingPermissionsRole) {
        const roleData = actionData.role as RoleWithPermissions;
        setManagingPermissionsRole({
          ...managingPermissionsRole,
          rolePermissions: roleData.permissions || [],
        });
      }

      // Revalidate loader to refresh the permissions list
      revalidator.revalidate();
    }
  }, [actionData, managingPermissionsRole, revalidator]);

  const openManagePermissions = async (role: EditableRole, servicePermissions: Permission[]) => {
    // Fetch current role permissions
    const formData = new FormData();
    formData.append("intent", "get_role_permissions");
    formData.append("role_id", role.id);

    try {
      const response = await fetch("", {
        method: "POST",
        body: formData,
      });
      const result = await response.json();

      if (result.success && result.role) {
        setManagingPermissionsRole({
          role,
          permissions: servicePermissions,
          rolePermissions: result.role.permissions || [],
        });
      } else {
        // Fallback: open with empty permissions
        setManagingPermissionsRole({
          role,
          permissions: servicePermissions,
          rolePermissions: [],
        });
      }
    } catch {
      // Fallback: open with empty permissions
      setManagingPermissionsRole({
        role,
        permissions: servicePermissions,
        rolePermissions: [],
      });
    }
  };

  const togglePermission = (permissionId: string, isAssigned: boolean) => {
    if (!managingPermissionsRole) return;

    const formData = new FormData();
    formData.append("intent", isAssigned ? "remove_permission" : "assign_permission");
    formData.append("role_id", managingPermissionsRole.role.id);
    formData.append("permission_id", permissionId);

    submit(formData, { method: "post" });

    // Optimistic update
    if (isAssigned) {
      setManagingPermissionsRole({
        ...managingPermissionsRole,
        rolePermissions: managingPermissionsRole.rolePermissions.filter(p => p.id !== permissionId),
      });
    } else {
      const permission = managingPermissionsRole.permissions.find(p => p.id === permissionId);
      if (permission) {
        setManagingPermissionsRole({
          ...managingPermissionsRole,
          rolePermissions: [...managingPermissionsRole.rolePermissions, permission],
        });
      }
    }
  };

  // Find all parent role options for a service
  const getParentRoleOptions = (serviceId: string, excludeRoleId?: string) => {
    const entry = data.entries.find(e => e.service.id === serviceId);
    if (!entry) return [];
    return entry.roles.filter(r => r.id !== excludeRoleId);
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-[24px] font-semibold text-[var(--text-primary)] tracking-tight">Roles & Permissions</h1>
        <p className="text-sm text-[var(--text-secondary)]">Manage roles, permissions, and role hierarchy per service</p>
      </div>

      <Tabs defaultValue="roles" className="w-full">
        <TabsList>
          <TabsTrigger value="roles">Roles</TabsTrigger>
          <TabsTrigger value="permissions">Permissions</TabsTrigger>
          <TabsTrigger value="hierarchy">Hierarchy</TabsTrigger>
        </TabsList>

        {/* Roles Tab */}
        <TabsContent value="roles">
          <Card>
            <CardHeader>
              <CardTitle>Role Management</CardTitle>
              <CardDescription>
                {data.pagination.total} services • Create and manage roles for each service
              </CardDescription>
            </CardHeader>
            <div className="px-6 pb-6 space-y-6">
              {data.entries.map((entry) => (
                <div
                  key={entry.service.id}
                  className="rounded-xl border border-[var(--glass-border-subtle)] p-4"
                >
                  <div className="flex items-center justify-between mb-4">
                    <div className="text-base font-semibold text-[var(--text-primary)]">
                      {entry.service.name}
                    </div>
                    <Button size="sm" variant="outline" onClick={() => setCreateRoleServiceId(entry.service.id)}>
                      <PlusIcon className="mr-2 h-3.5 w-3.5" /> Add Role
                    </Button>
                  </div>

                  <div className="divide-y divide-[var(--glass-border-subtle)] border-t border-[var(--glass-border-subtle)]">
                    {entry.roles.map((role) => (
                      <div key={role.id} className="flex items-center justify-between py-2 text-sm">
                        <div className="flex-1">
                          <span className="font-medium text-[var(--text-primary)]">{role.name}</span>
                          {role.description && (
                            <span className="ml-2 text-[var(--text-secondary)]">- {role.description}</span>
                          )}
                          {role.parent_role_id && (
                            <span className="ml-2 text-xs text-[var(--accent-blue)]">
                              (inherits from {entry.roles.find(r => r.id === role.parent_role_id)?.name || "parent"})
                            </span>
                          )}
                        </div>
                        <div className="flex items-center gap-2">
                          <Button
                            size="sm"
                            variant="ghost"
                            onClick={() => openManagePermissions({ ...role, service_id: entry.service.id }, entry.permissions)}
                          >
                            <GearIcon className="mr-1 h-3.5 w-3.5" />
                            Permissions
                          </Button>
                          <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                              <Button variant="ghost" className="h-6 w-6 p-0">
                                <span className="sr-only">Open menu</span>
                                <DotsHorizontalIcon className="h-3.5 w-3.5" />
                              </Button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end">
                              <DropdownMenuLabel>Actions</DropdownMenuLabel>
                              <DropdownMenuItem onClick={() => setEditingRole({ ...role, service_id: entry.service.id })}>
                                <Pencil2Icon className="mr-2 h-3.5 w-3.5" /> Edit
                              </DropdownMenuItem>
                              <DropdownMenuSeparator />
                              <DropdownMenuItem
                                className="text-[var(--accent-red)] focus:text-[var(--accent-red)]"
                                onClick={async () => {
                                  const ok = await confirm({
                                    title: "Delete Role",
                                    description: "Are you sure you want to delete this role?",
                                    variant: "destructive",
                                  });
                                  if (ok) {
                                    submit({
                                      intent: "delete_role",
                                      service_id: entry.service.id,
                                      role_id: role.id
                                    }, { method: "post" });
                                  }
                                }}
                              >
                                <TrashIcon className="mr-2 h-3.5 w-3.5" /> Delete
                              </DropdownMenuItem>
                            </DropdownMenuContent>
                          </DropdownMenu>
                        </div>
                      </div>
                    ))}
                    {entry.roles.length === 0 && (
                      <div className="py-4 text-center text-xs text-[var(--text-secondary)]">
                        No roles created yet for this service
                      </div>
                    )}
                  </div>
                </div>
              ))}
              {data.entries.length === 0 && (
                <div className="py-8 text-center text-sm text-[var(--text-secondary)]">No services found</div>
              )}
            </div>
          </Card>
        </TabsContent>

        {/* Permissions Tab */}
        <TabsContent value="permissions">
          <Card>
            <CardHeader>
              <CardTitle>Permission Management</CardTitle>
              <CardDescription>
                Create and manage permissions for each service
              </CardDescription>
            </CardHeader>
            <div className="px-6 pb-6 space-y-6">
              {data.entries.map((entry) => (
                <div
                  key={entry.service.id}
                  className="rounded-xl border border-[var(--glass-border-subtle)] p-4"
                >
                  <div className="flex items-center justify-between mb-4">
                    <div className="text-base font-semibold text-[var(--text-primary)]">
                      {entry.service.name}
                    </div>
                    <Button size="sm" variant="outline" onClick={() => setCreatePermissionServiceId(entry.service.id)}>
                      <PlusIcon className="mr-2 h-3.5 w-3.5" /> Add Permission
                    </Button>
                  </div>

                  <div className="border rounded-md overflow-hidden">
                    <table className="w-full text-sm">
                      <thead className="bg-[var(--sidebar-item-hover)]">
                        <tr>
                          <th className="px-4 py-2 text-left font-medium text-[var(--text-secondary)]">Code</th>
                          <th className="px-4 py-2 text-left font-medium text-[var(--text-secondary)]">Name</th>
                          <th className="px-4 py-2 text-left font-medium text-[var(--text-secondary)]">Description</th>
                          <th className="px-4 py-2 text-right font-medium text-[var(--text-secondary)]">Actions</th>
                        </tr>
                      </thead>
                      <tbody className="divide-y divide-[var(--glass-border-subtle)]">
                        {entry.permissions.map((permission) => (
                          <tr key={permission.id} className="hover:bg-[var(--sidebar-item-hover)]">
                            <td className="px-4 py-2 font-mono text-xs text-[var(--text-secondary)]">{permission.code}</td>
                            <td className="px-4 py-2 text-[var(--text-primary)]">{permission.name}</td>
                            <td className="px-4 py-2 text-[var(--text-secondary)]">{permission.description || "-"}</td>
                            <td className="px-4 py-2 text-right">
                              <Button
                                variant="ghost"
                                size="sm"
                                className="text-[var(--accent-red)] hover:text-[var(--accent-red)] h-7 px-2"
                                onClick={async () => {
                                  const ok = await confirm({
                                    title: "Delete Permission",
                                    description: "Are you sure you want to delete this permission?",
                                    variant: "destructive",
                                  });
                                  if (ok) {
                                    submit({
                                      intent: "delete_permission",
                                      permission_id: permission.id
                                    }, { method: "post" });
                                  }
                                }}
                              >
                                <TrashIcon className="h-3.5 w-3.5" />
                              </Button>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                    {entry.permissions.length === 0 && (
                      <div className="py-4 text-center text-xs text-[var(--text-secondary)]">
                        No permissions created yet for this service
                      </div>
                    )}
                  </div>
                </div>
              ))}
            </div>
          </Card>
        </TabsContent>

        {/* Hierarchy Tab */}
        <TabsContent value="hierarchy">
          <Card>
            <CardHeader>
              <CardTitle>Role Hierarchy</CardTitle>
              <CardDescription>
                View role inheritance structure for each service
              </CardDescription>
            </CardHeader>
            <div className="px-6 pb-6 space-y-6">
              {data.entries.map((entry) => {
                // Build hierarchy tree
                const rootRoles = entry.roles.filter(r => !r.parent_role_id);
                const childRoles = entry.roles.filter(r => r.parent_role_id);

                const renderRoleTree = (role: Role, level: number = 0): React.ReactNode => {
                  const children = childRoles.filter(r => r.parent_role_id === role.id);
                  return (
                    <div key={role.id} className="relative">
                      <div
                        className="flex items-center py-2 text-sm"
                        style={{ paddingLeft: `${level * 24 + 16}px` }}
                      >
                        {level > 0 && (
                          <span className="absolute left-0 h-full border-l-2 border-[var(--glass-border-subtle)]" style={{ left: `${(level - 1) * 24 + 24}px` }} />
                        )}
                        <div className="flex items-center gap-2">
                          <span className={`w-2 h-2 rounded-full ${level === 0 ? 'bg-[var(--accent-blue)]' : 'bg-gray-400'}`} />
                          <span className="font-medium text-[var(--text-primary)]">{role.name}</span>
                          {role.description && (
                            <span className="text-[var(--text-secondary)] text-xs">({role.description})</span>
                          )}
                          <span className="text-[var(--text-secondary)] text-xs px-1.5 py-0.5 rounded-full bg-[var(--glass-bg-subtle)]">
                            {entry.rolePermissionCounts[role.id] ?? 0} permission{(entry.rolePermissionCounts[role.id] ?? 0) !== 1 ? 's' : ''}
                          </span>
                        </div>
                      </div>
                      {children.map(child => renderRoleTree(child, level + 1))}
                    </div>
                  );
                };

                return (
                  <div
                    key={entry.service.id}
                    className="rounded-xl border border-[var(--glass-border-subtle)] p-4"
                  >
                    <div className="text-base font-semibold text-[var(--text-primary)] mb-4">
                      {entry.service.name}
                    </div>

                    <div className="border-t border-[var(--glass-border-subtle)]">
                      {rootRoles.length > 0 ? (
                        rootRoles.map(role => renderRoleTree(role))
                      ) : (
                        <div className="py-4 text-center text-xs text-[var(--text-secondary)]">
                          No roles defined for this service
                        </div>
                      )}
                    </div>

                    {childRoles.filter(r => !entry.roles.some(p => p.id === r.parent_role_id)).length > 0 && (
                      <div className="mt-4 pt-4 border-t border-[var(--glass-border-subtle)]">
                        <div className="text-xs text-[var(--accent-orange)] mb-2">Orphaned Roles (invalid parent):</div>
                        {childRoles
                          .filter(r => !entry.roles.some(p => p.id === r.parent_role_id))
                          .map(role => (
                            <div key={role.id} className="text-sm text-[var(--text-secondary)] pl-4">
                              {role.name}
                            </div>
                          ))
                        }
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </Card>
        </TabsContent>
      </Tabs>

      {/* Create Role Dialog */}
      <Dialog open={!!createRoleServiceId} onOpenChange={(open) => !open && setCreateRoleServiceId(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create Role</DialogTitle>
            <DialogDescription>Add a new role to this service.</DialogDescription>
          </DialogHeader>
          <Form method="post" className="space-y-4">
            <input type="hidden" name="intent" value="create_role" />
            <input type="hidden" name="service_id" value={createRoleServiceId || ""} />
            <div className="space-y-2">
              <Label htmlFor="create-name">Role Name</Label>
              <Input id="create-name" name="name" placeholder="admin" required />
            </div>
            <div className="space-y-2">
              <Label htmlFor="create-description">Description</Label>
              <Input id="create-description" name="description" placeholder="Administrator role with full access" />
            </div>
            <div className="space-y-2">
              <Label htmlFor="create-parent">Parent Role (Optional)</Label>
              <select
                id="create-parent"
                name="parent_role_id"
                className="w-full px-3 py-2 border border-[var(--glass-border-subtle)] rounded-md text-sm"
              >
                <option value="">No parent (root role)</option>
                {createRoleServiceId && getParentRoleOptions(createRoleServiceId).map(role => (
                  <option key={role.id} value={role.id}>{role.name}</option>
                ))}
              </select>
            </div>
            {actionData && "error" in actionData && (
              <p className="text-sm text-[var(--accent-red)]">{String(actionData.error)}</p>
            )}
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setCreateRoleServiceId(null)}>
                Cancel
              </Button>
              <Button type="submit" disabled={isSubmitting}>
                {isSubmitting ? "Creating..." : "Create"}
              </Button>
            </DialogFooter>
          </Form>
        </DialogContent>
      </Dialog>

      {/* Edit Role Dialog */}
      <Dialog open={!!editingRole} onOpenChange={(open) => !open && setEditingRole(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit Role</DialogTitle>
            <DialogDescription>Update role details.</DialogDescription>
          </DialogHeader>
          <Form method="post" className="space-y-4">
            <input type="hidden" name="intent" value="update_role" />
            <input type="hidden" name="service_id" value={editingRole?.service_id || ""} />
            <input type="hidden" name="role_id" value={editingRole?.id || ""} />
            <div className="space-y-2">
              <Label htmlFor="edit-name">Role Name</Label>
              <Input id="edit-name" name="name" defaultValue={editingRole?.name} required />
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-description">Description</Label>
              <Input id="edit-description" name="description" defaultValue={editingRole?.description} />
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-parent">Parent Role (Optional)</Label>
              <select
                id="edit-parent"
                name="parent_role_id"
                defaultValue={editingRole?.parent_role_id || ""}
                className="w-full px-3 py-2 border border-[var(--glass-border-subtle)] rounded-md text-sm"
              >
                <option value="">No parent (root role)</option>
                {editingRole && getParentRoleOptions(editingRole.service_id, editingRole.id).map(role => (
                  <option key={role.id} value={role.id}>{role.name}</option>
                ))}
              </select>
            </div>
            {actionData && "error" in actionData && (
              <p className="text-sm text-[var(--accent-red)]">{String(actionData.error)}</p>
            )}
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setEditingRole(null)}>
                Cancel
              </Button>
              <Button type="submit" disabled={isSubmitting}>
                {isSubmitting ? "Saving..." : "Save Changes"}
              </Button>
            </DialogFooter>
          </Form>
        </DialogContent>
      </Dialog>

      {/* Create Permission Dialog */}
      <Dialog open={!!createPermissionServiceId} onOpenChange={(open) => !open && setCreatePermissionServiceId(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create Permission</DialogTitle>
            <DialogDescription>Add a new permission to this service.</DialogDescription>
          </DialogHeader>
          <Form method="post" className="space-y-4">
            <input type="hidden" name="intent" value="create_permission" />
            <input type="hidden" name="service_id" value={createPermissionServiceId || ""} />
            <div className="space-y-2">
              <Label htmlFor="create-perm-code">Permission Code</Label>
              <Input id="create-perm-code" name="code" placeholder="user:read" required />
              <p className="text-xs text-[var(--text-secondary)]">Use format like &quot;resource:action&quot; (e.g., user:read, post:write)</p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="create-perm-name">Display Name</Label>
              <Input id="create-perm-name" name="name" placeholder="Read Users" required />
            </div>
            <div className="space-y-2">
              <Label htmlFor="create-perm-description">Description</Label>
              <Input id="create-perm-description" name="description" placeholder="Allows reading user information" />
            </div>
            {actionData && "error" in actionData && (
              <p className="text-sm text-[var(--accent-red)]">{String(actionData.error)}</p>
            )}
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setCreatePermissionServiceId(null)}>
                Cancel
              </Button>
              <Button type="submit" disabled={isSubmitting}>
                {isSubmitting ? "Creating..." : "Create"}
              </Button>
            </DialogFooter>
          </Form>
        </DialogContent>
      </Dialog>

      {/* Manage Role Permissions Dialog */}
      <Dialog
        open={!!managingPermissionsRole}
        onOpenChange={(open) => !open && setManagingPermissionsRole(null)}
      >
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Manage Permissions</DialogTitle>
            <DialogDescription>
              Assign permissions to role: <strong>{managingPermissionsRole?.role.name}</strong>
            </DialogDescription>
          </DialogHeader>
          <div className="max-h-[400px] overflow-y-auto">
            {managingPermissionsRole?.permissions.length === 0 ? (
              <div className="py-8 text-center text-sm text-[var(--text-secondary)]">
                No permissions defined for this service.
                <br />
                Create permissions in the Permissions tab first.
              </div>
            ) : (
              <div className="space-y-2">
                {managingPermissionsRole?.permissions.map((permission) => {
                  const isAssigned = managingPermissionsRole.rolePermissions.some(p => p.id === permission.id);
                  return (
                    <label
                      key={permission.id}
                      className="flex items-start gap-3 p-3 rounded-lg border border-[var(--glass-border-subtle)] hover:bg-[var(--sidebar-item-hover)] cursor-pointer"
                    >
                      <Checkbox
                        checked={isAssigned}
                        onCheckedChange={() => togglePermission(permission.id, isAssigned)}
                        className="mt-0.5"
                      />
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <span className="font-mono text-xs bg-[var(--sidebar-item-hover)] px-1.5 py-0.5 rounded">
                            {permission.code}
                          </span>
                          <span className="font-medium text-sm text-[var(--text-primary)]">{permission.name}</span>
                        </div>
                        {permission.description && (
                          <p className="text-xs text-[var(--text-secondary)] mt-1">{permission.description}</p>
                        )}
                      </div>
                      {isAssigned && (
                        <CheckIcon className="h-4 w-4 text-[var(--accent-green)] mt-0.5" />
                      )}
                    </label>
                  );
                })}
              </div>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setManagingPermissionsRole(null)}>
              Done
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
