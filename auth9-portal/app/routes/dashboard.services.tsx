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
import { serviceApi } from "~/services/api";

export const meta: MetaFunction = () => {
  return [{ title: "Services - Auth9" }];
};

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const perPage = Number(url.searchParams.get("perPage") || "20");
  const services = await serviceApi.list(undefined, page, perPage);
  return json(services);
}

export async function action({ request }: ActionFunctionArgs) {
  const formData = await request.formData();
  const intent = formData.get("intent");

  try {
    if (intent === "create") {
      const name = formData.get("name") as string;
      const client_id = formData.get("client_id") as string;
      const base_url = formData.get("base_url") as string;
      const redirect_uris = (formData.get("redirect_uris") as string)?.split(",").map(s => s.trim()).filter(Boolean);
      const logout_uris = (formData.get("logout_uris") as string)?.split(",").map(s => s.trim()).filter(Boolean);

      await serviceApi.create({
        name,
        client_id: client_id || undefined,
        base_url: base_url || undefined,
        redirect_uris,
        logout_uris
      });
      return json({ success: true });
    }

    if (intent === "update") {
      const id = formData.get("id") as string;
      const name = formData.get("name") as string;
      const base_url = formData.get("base_url") as string;
      const redirect_uris = (formData.get("redirect_uris") as string)?.split(",").map(s => s.trim()).filter(Boolean);
      const logout_uris = (formData.get("logout_uris") as string)?.split(",").map(s => s.trim()).filter(Boolean);

      await serviceApi.update(id, {
        name,
        base_url: base_url || undefined,
        redirect_uris,
        logout_uris
      });
      return json({ success: true });
    }

    if (intent === "delete") {
      const id = formData.get("id") as string;
      await serviceApi.delete(id);
      return json({ success: true });
    }

    if (intent === "regenerate_secret") {
      const id = formData.get("id") as string;
      const res = await serviceApi.regenerateSecret(id);
      return json({ success: true, secret: res.data.client_secret, intent: "regenerate_secret" });
    }
  } catch (error: any) {
    return json({ error: error.message }, { status: 400 });
  }

  return json({ error: "Invalid intent" }, { status: 400 });
}

export default function ServicesPage() {
  const data = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const submit = useSubmit();
  const [isCreateOpen, setIsCreateOpen] = useState(false);
  const [editingService, setEditingService] = useState<any>(null);
  const [newSecret, setNewSecret] = useState<string | null>(null);

  const isSubmitting = navigation.state === "submitting";

  useEffect(() => {
    if (actionData && "success" in actionData && actionData.success) {
      if (actionData.intent !== "regenerate_secret") {
        setIsCreateOpen(false);
        setEditingService(null);
      }
      if (actionData.intent === "regenerate_secret" && actionData.secret) {
        setNewSecret(actionData.secret);
      }
    }
  }, [actionData]);

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold text-gray-900">Services</h1>
          <p className="text-sm text-gray-500">Register and manage OIDC clients</p>
        </div>
        <Dialog open={isCreateOpen} onOpenChange={setIsCreateOpen}>
          <DialogTrigger asChild>
            <Button>
              <PlusIcon className="mr-2 h-4 w-4" /> Register Service
            </Button>
          </DialogTrigger>
          <DialogContent className="max-w-2xl">
            <DialogHeader>
              <DialogTitle>Register Service</DialogTitle>
              <DialogDescription>
                Register a new OIDC client application.
              </DialogDescription>
            </DialogHeader>
            <Form method="post" className="space-y-4">
              <input type="hidden" name="intent" value="create" />
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label htmlFor="create-name">Service Name</Label>
                  <Input id="create-name" name="name" placeholder="My App" required />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="create-client-id">Client ID (Optional)</Label>
                  <Input id="create-client-id" name="client_id" placeholder="my-app-client" />
                </div>
              </div>
              <div className="space-y-2">
                <Label htmlFor="create-base-url">Base URL</Label>
                <Input id="create-base-url" name="base_url" placeholder="https://myapp.com" />
              </div>
              <div className="space-y-2">
                <Label htmlFor="create-redirect-uris">Redirect URIs (comma separated)</Label>
                <Input id="create-redirect-uris" name="redirect_uris" placeholder="https://myapp.com/callback, https://dev.myapp.com/callback" />
              </div>
              <div className="space-y-2">
                <Label htmlFor="create-logout-uris">Logout URIs (comma separated)</Label>
                <Input id="create-logout-uris" name="logout_uris" placeholder="https://myapp.com/logout" />
              </div>
              {actionData && "error" in actionData && (
                <p className="text-sm text-red-500">{actionData.error}</p>
              )}
              <DialogFooter>
                <Button type="button" variant="outline" onClick={() => setIsCreateOpen(false)}>
                  Cancel
                </Button>
                <Button type="submit" disabled={isSubmitting}>
                  {isSubmitting ? "Registering..." : "Register"}
                </Button>
              </DialogFooter>
            </Form>
          </DialogContent>
        </Dialog>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Service Registry</CardTitle>
          <CardDescription>
            {data.pagination.total} services • Page {data.pagination.page} of{" "}
            {data.pagination.total_pages}
          </CardDescription>
        </CardHeader>
        <div className="px-6 pb-6">
          <div className="overflow-hidden rounded-apple border border-gray-100">
            <table className="min-w-full divide-y divide-gray-100 text-sm">
              <thead className="bg-gray-50 text-left text-gray-500">
                <tr>
                  <th className="px-4 py-3 font-medium">Name</th>
                  <th className="px-4 py-3 font-medium">Client ID</th>
                  <th className="px-4 py-3 font-medium">Status</th>
                  <th className="px-4 py-3 font-medium">Updated</th>
                  <th className="px-4 py-3 font-medium w-10"></th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {data.data.map((service) => (
                  <tr key={service.id} className="text-gray-700 hover:bg-gray-50/50">
                    <td className="px-4 py-3 font-medium text-gray-900">{service.name}</td>
                    <td className="px-4 py-3 font-mono text-xs">{service.client_id}</td>
                    <td className="px-4 py-3 capitalize">{service.status}</td>
                    <td className="px-4 py-3">
                      {new Date(service.updated_at).toLocaleString()}
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
                          <DropdownMenuItem onClick={() => setEditingService(service)}>
                            <Pencil2Icon className="mr-2 h-3.5 w-3.5" /> Edit
                          </DropdownMenuItem>
                          <DropdownMenuSeparator />
                          <DropdownMenuItem
                            className="text-red-600 focus:text-red-600"
                            onClick={() => {
                              if (confirm("Are you sure you want to delete this service?")) {
                                submit({ intent: "delete", id: service.id }, { method: "post" });
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
                {data.data.length === 0 && (
                  <tr>
                    <td className="px-4 py-6 text-center text-gray-500" colSpan={5}>
                      No services found
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      </Card>

      {/* Edit Dialog */}
      <Dialog open={!!editingService} onOpenChange={(open) => !open && setEditingService(null)}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Edit Service</DialogTitle>
            <DialogDescription>
              Update service details.
            </DialogDescription>
          </DialogHeader>
          <Form method="post" className="space-y-4">
            <input type="hidden" name="intent" value="update" />
            <input type="hidden" name="id" value={editingService?.id || ""} />
            <div className="space-y-2">
              <Label htmlFor="edit-name">Service Name</Label>
              <Input
                id="edit-name"
                name="name"
                defaultValue={editingService?.name}
                required
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-base-url">Base URL</Label>
              <Input
                id="edit-base-url"
                name="base_url"
                defaultValue={editingService?.base_url}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-redirect-uris">Redirect URIs (comma separated)</Label>
              <Input
                id="edit-redirect-uris"
                name="redirect_uris"
                defaultValue={editingService?.redirect_uris?.join(", ")}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-logout-uris">Logout URIs (comma separated)</Label>
              <Input
                id="edit-logout-uris"
                name="logout_uris"
                defaultValue={editingService?.logout_uris?.join(", ")}
              />
            </div>
            <div className="rounded-md bg-amber-50 p-3 text-sm text-amber-900 border border-amber-200">
              <div className="font-semibold mb-1">Client Credentials</div>
              <p className="mb-2">Client ID: <span className="font-mono">{editingService?.client_id}</span></p>
              <Button
                type="button"
                variant="outline"
                size="sm"
                className="w-full bg-white"
                onClick={() => {
                  if (confirm("This will invalidate the old secret. Generate new secret?")) {
                    submit({ intent: "regenerate_secret", id: editingService.id }, { method: "post" });
                  }
                }}
              >
                Regenerate Client Secret
              </Button>
            </div>
            {actionData && "error" in actionData && (
              <p className="text-sm text-red-500">{actionData.error}</p>
            )}
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setEditingService(null)}>
                Cancel
              </Button>
              <Button type="submit" disabled={isSubmitting}>
                {isSubmitting ? "Saving..." : "Save Changes"}
              </Button>
            </DialogFooter>
          </Form>
        </DialogContent>
      </Dialog>

      {/* Secret Display Dialog */}
      <Dialog open={!!newSecret} onOpenChange={(open) => !open && setNewSecret(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Client Secret Generated</DialogTitle>
            <DialogDescription>Please copy this secret now. It will not be shown again.</DialogDescription>
          </DialogHeader>
          <div className="p-4 bg-gray-100 rounded border font-mono text-center break-all select-all">
            {newSecret}
          </div>
          <DialogFooter>
            <Button onClick={() => setNewSecret(null)}>Close</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
