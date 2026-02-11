import type { MetaFunction, LoaderFunctionArgs, ActionFunctionArgs } from "react-router";
import { Form, Link, useActionData, useLoaderData, useNavigation, useSubmit } from "react-router";
import { PlusIcon, DotsHorizontalIcon, Pencil2Icon, TrashIcon, EnvelopeClosedIcon, Link2Icon, MagnifyingGlassIcon } from "@radix-ui/react-icons";
import { useEffect, useState } from "react";
import { useConfirm } from "~/hooks/useConfirm";
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
import { tenantApi, type Tenant } from "~/services/api";
import { requireAuthWithUpdate } from "~/services/session.server";
import { formatErrorMessage } from "~/lib/error-messages";

export const meta: MetaFunction = () => {
  return [{ title: "Tenants - Auth9" }];
};

export async function loader({ request }: LoaderFunctionArgs) {
  const { session, headers } = await requireAuthWithUpdate(request);
  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const perPage = Number(url.searchParams.get("perPage") || "20");
  const search = url.searchParams.get("search") || undefined;
  const tenants = await tenantApi.list(page, perPage, search, session.accessToken);
  const data = { ...tenants, search: search || "" };

  if (headers) {
    return Response.json(data, { headers });
  }
  return data;
}

export async function action({ request }: ActionFunctionArgs) {
  const formData = await request.formData();
  const intent = formData.get("intent");
  const { session, headers } = await requireAuthWithUpdate(request);
  const accessToken = session.accessToken;

  const returnSuccess = () => {
    if (headers) {
      return Response.json({ success: true }, { headers });
    }
    return { success: true };
  };

  const returnError = (message: string, status = 400) => {
    const errorData = { error: message, intent: String(intent) };
    if (headers) {
      return Response.json(errorData, { status, headers });
    }
    return Response.json(errorData, { status });
  };

  try {
    if (intent === "create") {
      const name = formData.get("name") as string;
      const slug = formData.get("slug") as string;
      const logo_url = formData.get("logo_url") as string;

      await tenantApi.create({ name, slug, logo_url: logo_url || undefined }, accessToken);
      return returnSuccess();
    }

    if (intent === "update") {
      const id = formData.get("id") as string;
      const name = formData.get("name") as string;
      const slug = formData.get("slug") as string;
      const logo_url = formData.get("logo_url") as string;

      await tenantApi.update(id, { name, slug, logo_url: logo_url || undefined }, accessToken);
      return returnSuccess();
    }

    if (intent === "delete") {
      const id = formData.get("id") as string;
      await tenantApi.delete(id, accessToken);
      return returnSuccess();
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown error";
    return returnError(message);
  }

  return returnError("Invalid intent");
}

export default function TenantsIndexPage() {
  const data = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const submit = useSubmit();
  const confirm = useConfirm();
  const [isCreateOpen, setIsCreateOpen] = useState(false);
  const [editingTenant, setEditingTenant] = useState<Tenant | null>(null);
  const [searchValue, setSearchValue] = useState(data.search || "");

  const isSubmitting = navigation.state === "submitting";

  // Close dialogs on success
  useEffect(() => {
    if (actionData && "success" in actionData && actionData.success) {
      setIsCreateOpen(false);
      setEditingTenant(null);
    }
  }, [actionData]);

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-[24px] font-semibold text-[var(--text-primary)] tracking-tight">Tenants</h1>
          <p className="text-sm text-[var(--text-secondary)]">Manage tenant lifecycle and settings</p>
        </div>
        <Dialog open={isCreateOpen} onOpenChange={setIsCreateOpen}>
          <DialogTrigger asChild>
            <Button className="w-full sm:w-auto">
              <PlusIcon className="mr-2 h-4 w-4" /> Create Tenant
            </Button>
          </DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Create Tenant</DialogTitle>
              <DialogDescription>
                Add a new tenant to the system. Slug must be unique.
              </DialogDescription>
            </DialogHeader>
            <Form method="post" className="space-y-4">
              <input type="hidden" name="intent" value="create" />
              <div className="space-y-1.5">
                <Label htmlFor="create-name">Name</Label>
                <Input id="create-name" name="name" placeholder="Acme Corp" required />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="create-slug">Slug</Label>
                <Input id="create-slug" name="slug" placeholder="acme" required />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="create-logo">Logo URL</Label>
                <Input id="create-logo" name="logo_url" placeholder="https://..." />
              </div>
              {actionData && "error" in actionData && (
                <p className="text-sm text-[var(--accent-red)]">{formatErrorMessage(String(actionData.error))}</p>
              )}
              <DialogFooter>
                <Button type="button" variant="outline" className="bg-[var(--glass-bg)]" onClick={() => setIsCreateOpen(false)}>
                  Cancel
                </Button>
                <Button type="submit" disabled={isSubmitting}>
                  {isSubmitting ? "Creating..." : "Create"}
                </Button>
              </DialogFooter>
            </Form>
          </DialogContent>
        </Dialog>
      </div>

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>Tenant List</CardTitle>
              <CardDescription>
                {data.pagination.total} tenants • Page {data.pagination.page} of{" "}
                {data.pagination.total_pages}
              </CardDescription>
            </div>
            <Form method="get" className="flex flex-col sm:flex-row items-center gap-2 w-full sm:w-auto mb-6">
              <div className="relative w-full sm:w-auto">
                <MagnifyingGlassIcon className="absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-[var(--text-tertiary)]" />
                <Input
                  name="search"
                  placeholder="Search tenants..."
                  value={searchValue}
                  onChange={(e) => setSearchValue(e.target.value)}
                  className="w-full sm:w-[200px] pl-8"
                />
              </div>
              <div className="flex gap-2 w-full sm:w-auto">
                <Button type="submit" variant="outline" className="bg-[var(--glass-bg)] flex-1 sm:flex-none" size="default">
                  Search
                </Button>
                {data.search && (
                  <Button
                    type="button"
                    variant="ghost"
                    size="default"
                    className="flex-1 sm:flex-none"
                    onClick={() => {
                      setSearchValue("");
                      window.location.href = "/dashboard/tenants";
                    }}
                  >
                    Clear
                  </Button>
                )}
              </div>
            </Form>
          </div>
        </CardHeader>
        <div className="px-6 pb-6">
          <div className="mt-2 overflow-hidden rounded-xl border border-[var(--glass-border-subtle)]">
            <table className="min-w-full divide-y divide-[var(--glass-border-subtle)] text-sm">
              <thead className="bg-[var(--sidebar-item-hover)] text-left text-[var(--text-tertiary)] uppercase tracking-[0.04em] text-[11px]">
                <tr>
                  <th className="px-4 py-3 font-semibold">Name</th>
                  <th className="px-4 py-3 font-semibold">Slug</th>
                  <th className="px-4 py-3 font-semibold">Status</th>
                  <th className="px-4 py-3 font-semibold">Updated</th>
                  <th className="px-4 py-3 font-semibold w-10"></th>
                </tr>
              </thead>
              <tbody className="divide-y divide-[var(--glass-border-subtle)]">
                {data.data.map((tenant) => (
                  <tr key={tenant.id} className="text-[var(--text-secondary)] hover:bg-[var(--sidebar-item-hover)]/50">
                    <td className="px-4 py-3 font-medium text-[var(--text-primary)]">
                      <Link
                        to={`/dashboard/tenants/${tenant.id}`}
                        className="flex items-center gap-2 hover:underline"
                      >
                        {tenant.logo_url && (
                          <img src={tenant.logo_url} alt="" className="h-6 w-6 rounded object-cover" />
                        )}
                        {tenant.name}
                      </Link>
                    </td>
                    <td className="px-4 py-3">{tenant.slug}</td>
                    <td className="px-4 py-3 capitalize">{tenant.status}</td>
                    <td className="px-4 py-3">
                      {new Date(tenant.updated_at).toLocaleString()}
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
                          <DropdownMenuItem onClick={() => setEditingTenant(tenant)}>
                            <Pencil2Icon className="mr-2 h-3.5 w-3.5" /> Edit
                          </DropdownMenuItem>
                          <DropdownMenuItem asChild>
                            <Link to={`/dashboard/tenants/${tenant.id}/invitations`}>
                              <EnvelopeClosedIcon className="mr-2 h-3.5 w-3.5" /> Invitations
                            </Link>
                          </DropdownMenuItem>
                          <DropdownMenuItem asChild>
                            <Link to={`/dashboard/tenants/${tenant.id}/webhooks`}>
                              <Link2Icon className="mr-2 h-3.5 w-3.5" /> Webhooks
                            </Link>
                          </DropdownMenuItem>
                          <DropdownMenuSeparator />
                          <DropdownMenuItem
                            className="text-[var(--accent-red)] focus:text-[var(--accent-red)]"
                            onClick={async () => {
                              const ok = await confirm({
                                title: "Delete Tenant",
                                description: "Are you sure you want to delete this tenant?",
                                variant: "destructive",
                              });
                              if (ok) {
                                submit({ intent: "delete", id: tenant.id }, { method: "post" });
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
                    <td className="px-4 py-6 text-center text-[var(--text-tertiary)]" colSpan={5}>
                      No tenants found
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      </Card>

      {/* Edit Dialog */}
      <Dialog open={!!editingTenant} onOpenChange={(open) => !open && setEditingTenant(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit Tenant</DialogTitle>
            <DialogDescription>
              Update tenant details.
            </DialogDescription>
          </DialogHeader>
          <Form method="post" className="space-y-4">
            <input type="hidden" name="intent" value="update" />
            <input type="hidden" name="id" value={editingTenant?.id || ""} />
            <div className="space-y-1.5">
              <Label htmlFor="edit-name">Name</Label>
              <Input
                id="edit-name"
                name="name"
                defaultValue={editingTenant?.name}
                required
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="edit-slug">Slug</Label>
              <Input
                id="edit-slug"
                name="slug"
                defaultValue={editingTenant?.slug}
                required
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="edit-logo">Logo URL</Label>
              <Input
                id="edit-logo"
                name="logo_url"
                defaultValue={editingTenant?.logo_url}
              />
            </div>
            {actionData && "error" in actionData && "intent" in actionData && actionData.intent === "update" && (
              <p className="text-sm text-[var(--accent-red)]">{formatErrorMessage(String(actionData.error))}</p>
            )}
            <DialogFooter>
              <Button type="button" variant="outline" className="bg-[var(--glass-bg)]" onClick={() => setEditingTenant(null)}>
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
