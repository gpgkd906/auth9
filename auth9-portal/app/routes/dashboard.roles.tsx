import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "@remix-run/node";
import { json } from "@remix-run/node";
import { Form, useActionData, useLoaderData, useNavigation, useSubmit } from "@remix-run/react";
import { PlusIcon, DotsHorizontalIcon, Pencil2Icon, TrashIcon } from "@radix-ui/react-icons";
import { useEffect, useState } from "react";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
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
  DialogTrigger,
} from "~/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "~/components/ui/dropdown-menu";
import { rbacApi, serviceApi } from "~/services/api";

export const meta: MetaFunction = () => {
  return [{ title: "Roles - Auth9" }];
};

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const perPage = Number(url.searchParams.get("perPage") || "50");
  const services = await serviceApi.list(undefined, page, perPage);

  const entries = await Promise.all(
    services.data.map(async (service) => {
      const [roles, permissions] = await Promise.all([
        rbacApi.listRoles(service.id),
        rbacApi.listPermissions(service.id),
      ]);
      return { service, roles: roles.data, permissions: permissions.data };
    })
  );

  return json({ entries, pagination: services.pagination });
}

export async function action({ request }: ActionFunctionArgs) {
  const formData = await request.formData();
  const intent = formData.get("intent");

  try {
    if (intent === "create_role") {
      const serviceId = formData.get("service_id") as string;
      const name = formData.get("name") as string;
      const description = formData.get("description") as string;

      await rbacApi.createRole(serviceId, { name, description: description || undefined });
      return json({ success: true });
    }

    if (intent === "update_role") {
      const serviceId = formData.get("service_id") as string;
      const roleId = formData.get("role_id") as string;
      const name = formData.get("name") as string;
      const description = formData.get("description") as string;

      await rbacApi.updateRole(serviceId, roleId, { name, description: description || undefined });
      return json({ success: true });
    }

    if (intent === "delete_role") {
      const serviceId = formData.get("service_id") as string;
      const roleId = formData.get("role_id") as string;
      await rbacApi.deleteRole(serviceId, roleId);
      return json({ success: true });
    }
  } catch (error: any) {
    return json({ error: error.message }, { status: 400 });
  }

  return json({ error: "Invalid intent" }, { status: 400 });
}

export default function RolesPage() {
  const data = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const submit = useSubmit();

  const [createServiceId, setCreateServiceId] = useState<string | null>(null);
  const [editingRole, setEditingRole] = useState<any>(null);

  const isSubmitting = navigation.state === "submitting";

  useEffect(() => {
    if (actionData && "success" in actionData && actionData.success) {
      setCreateServiceId(null);
      setEditingRole(null);
    }
  }, [actionData]);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold text-gray-900">Roles</h1>
        <p className="text-sm text-gray-500">Define roles and permissions per service</p>
      </div>
      <Card>
        <CardHeader>
          <CardTitle>Role Management</CardTitle>
          <CardDescription>
            {data.pagination.total} services • Page {data.pagination.page} of{" "}
            {data.pagination.total_pages}
          </CardDescription>
        </CardHeader>
        <div className="px-6 pb-6 space-y-6">
          {data.entries.map((entry) => (
            <div
              key={entry.service.id}
              className="rounded-apple border border-gray-100 p-4"
            >
              <div className="flex items-center justify-between mb-4">
                <div>
                  <div className="flex items-baseline gap-2">
                    <div className="text-base font-semibold text-gray-900">
                      {entry.service.name}
                    </div>
                    <div className="text-xs text-gray-500 font-mono">{entry.service.client_id}</div>
                  </div>
                </div>
                <Button size="sm" variant="outline" onClick={() => setCreateServiceId(entry.service.id)}>
                  <PlusIcon className="mr-2 h-3.5 w-3.5" /> Add Role
                </Button>
              </div>

              <div className="divide-y divide-gray-100 border-t border-gray-100">
                {entry.roles.map((role) => (
                  <div key={role.id} className="flex items-center justify-between py-2 text-sm">
                    <div>
                      <span className="font-medium text-gray-900">{role.name}</span>
                      {role.description && (
                        <span className="ml-2 text-gray-500">- {role.description}</span>
                      )}
                    </div>
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
                          className="text-red-600 focus:text-red-600"
                          onClick={() => {
                            if (confirm("Are you sure you want to delete this role?")) {
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
                ))}
                {entry.roles.length === 0 && (
                  <div className="py-4 text-center text-xs text-gray-500">
                    No roles created yet for this service
                  </div>
                )}
              </div>
            </div>
          ))}
          {data.entries.length === 0 && (
            <div className="py-8 text-center text-sm text-gray-500">No services found</div>
          )}
        </div>
      </Card>

      {/* Create Dialog */}
      <Dialog open={!!createServiceId} onOpenChange={(open) => !open && setCreateServiceId(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create Role</DialogTitle>
            <DialogDescription>Add a new role to this service.</DialogDescription>
          </DialogHeader>
          <Form method="post" className="space-y-4">
            <input type="hidden" name="intent" value="create_role" />
            <input type="hidden" name="service_id" value={createServiceId || ""} />
            <div className="space-y-2">
              <Label htmlFor="create-name">Role Name</Label>
              <Input id="create-name" name="name" placeholder="admin" required />
            </div>
            <div className="space-y-2">
              <Label htmlFor="create-description">Description</Label>
              <Input id="create-description" name="description" placeholder="Administrator role with full access" />
            </div>
            {actionData && "error" in actionData && (
              <p className="text-sm text-red-500">{actionData.error}</p>
            )}
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setCreateServiceId(null)}>
                Cancel
              </Button>
              <Button type="submit" disabled={isSubmitting}>
                {isSubmitting ? "Creating..." : "Create"}
              </Button>
            </DialogFooter>
          </Form>
        </DialogContent>
      </Dialog>

      {/* Edit Dialog */}
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
            {actionData && "error" in actionData && (
              <p className="text-sm text-red-500">{actionData.error}</p>
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
    </div>
  );
}
