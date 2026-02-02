import type { MetaFunction, LoaderFunctionArgs, ActionFunctionArgs } from "react-router";
import { Form, Link, useActionData, useLoaderData, useNavigation, useParams, useSubmit } from "react-router";
import { PlusIcon, DotsHorizontalIcon, TrashIcon, ReloadIcon, Cross2Icon, ArrowLeftIcon } from "@radix-ui/react-icons";
import { useEffect, useState } from "react";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Checkbox } from "~/components/ui/checkbox";
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "~/components/ui/select";
import { invitationApi, tenantApi, tenantServiceApi, rbacApi, type Invitation, type Role, type Tenant } from "~/services/api";
import { formatDateTime } from "~/lib/utils";
import { getAccessToken } from "~/services/session.server";

export const meta: MetaFunction = () => {
  return [{ title: "Invitations - Auth9" }];
};

interface LoaderData {
  tenant: Tenant;
  invitations: Invitation[];
  pagination: {
    page: number;
    per_page: number;
    total: number;
    total_pages: number;
  };
  roles: { serviceId: string; serviceName: string; roles: Role[] }[];
  servicesCount: number;
}

export async function loader({ params, request }: LoaderFunctionArgs) {
  const tenantId = params.tenantId;
  if (!tenantId) {
    throw new Response("Tenant ID required", { status: 400 });
  }

  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const perPage = Number(url.searchParams.get("perPage") || "20");

  // Fetch tenant, invitations, and enabled services in parallel
  const [tenantResult, invitationsResult, servicesResult] = await Promise.all([
    tenantApi.get(tenantId),
    invitationApi.list(tenantId, page, perPage),
    tenantServiceApi.getEnabledServices(tenantId), // Get enabled services for this tenant
  ]);

  // Fetch roles for each service
  const rolesPromises = servicesResult.data.map(async (service) => {
    const rolesResult = await rbacApi.listRoles(service.id);
    return {
      serviceId: service.id,
      serviceName: service.name,
      roles: rolesResult.data,
    };
  });

  const roles = await Promise.all(rolesPromises);

  return {
    tenant: tenantResult.data,
    invitations: invitationsResult.data,
    pagination: invitationsResult.pagination,
    roles,
    servicesCount: servicesResult.data.length,
  } satisfies LoaderData;
}

export async function action({ params, request }: ActionFunctionArgs) {
  const tenantId = params.tenantId;
  if (!tenantId) {
    throw new Response("Tenant ID required", { status: 400 });
  }

  const formData = await request.formData();
  const intent = formData.get("intent");
  const accessToken = await getAccessToken(request);

  if (!accessToken) {
    return Response.json({ error: "Authentication required" }, { status: 401 });
  }

  try {
    if (intent === "create") {
      const email = formData.get("email") as string;
      const expiresInHours = parseInt(formData.get("expires_in_hours") as string, 10) || 72;

      // Get selected role IDs from form
      const roleIds: string[] = [];
      for (const [key, value] of formData.entries()) {
        if (key.startsWith("role_") && value === "on") {
          roleIds.push(key.replace("role_", ""));
        }
      }

      if (roleIds.length === 0) {
        return Response.json({ error: "At least one role must be selected" }, { status: 400 });
      }

      await invitationApi.create(tenantId, {
        email,
        role_ids: roleIds,
        expires_in_hours: expiresInHours,
      }, accessToken);

      return { success: true };
    }

    if (intent === "revoke") {
      const id = formData.get("id") as string;
      await invitationApi.revoke(id);
      return { success: true };
    }

    if (intent === "resend") {
      const id = formData.get("id") as string;
      await invitationApi.resend(id);
      return { success: true, message: "Invitation email resent" };
    }

    if (intent === "delete") {
      const id = formData.get("id") as string;
      await invitationApi.delete(id);
      return { success: true };
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown error";
    return Response.json({ error: message }, { status: 400 });
  }

  return Response.json({ error: "Invalid intent" }, { status: 400 });
}

function getStatusBadge(status: Invitation["status"]) {
  const styles = {
    pending: "bg-yellow-50 text-yellow-700 border-yellow-200",
    accepted: "bg-[var(--accent-green)]/10 text-[var(--accent-green)] border-[var(--accent-green)]/20",
    expired: "bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)] border-[var(--glass-border-subtle)]",
    revoked: "bg-red-50 text-red-700 border-red-200",
  };

  const labels = {
    pending: "Pending",
    accepted: "Accepted",
    expired: "Expired",
    revoked: "Revoked",
  };

  return (
    <span className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium border ${styles[status]}`}>
      {labels[status]}
    </span>
  );
}

export default function InvitationsPage() {
  const { tenant, invitations, pagination, roles, servicesCount } = useLoaderData<LoaderData>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const submit = useSubmit();
  const params = useParams();

  const [isCreateOpen, setIsCreateOpen] = useState(false);
  const [selectedRoles, setSelectedRoles] = useState<Set<string>>(new Set());

  const isSubmitting = navigation.state === "submitting";

  // Close dialog on success
  useEffect(() => {
    if (actionData && "success" in actionData && actionData.success) {
      setIsCreateOpen(false);
      setSelectedRoles(new Set());
    }
  }, [actionData]);

  const handleRoleToggle = (roleId: string) => {
    setSelectedRoles((prev) => {
      const next = new Set(prev);
      if (next.has(roleId)) {
        next.delete(roleId);
      } else {
        next.add(roleId);
      }
      return next;
    });
  };

  const handleDelete = (id: string) => {
    if (confirm("Are you sure you want to delete this invitation?")) {
      submit({ intent: "delete", id }, { method: "post" });
    }
  };

  const handleRevoke = (id: string) => {
    if (confirm("Are you sure you want to revoke this invitation?")) {
      submit({ intent: "revoke", id }, { method: "post" });
    }
  };

  const handleResend = (id: string) => {
    submit({ intent: "resend", id }, { method: "post" });
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <div className="flex items-center gap-3 mb-1">
            <Link
              to="/dashboard/tenants"
              className="text-[var(--text-secondary)] hover:text-[var(--text-secondary)] transition-colors"
            >
              <ArrowLeftIcon className="h-5 w-5" />
            </Link>
            <h1 className="text-[24px] font-semibold text-[var(--text-primary)] tracking-tight">Invitations</h1>
          </div>
          <p className="text-sm text-[var(--text-secondary)] ml-8">
            Manage user invitations for <span className="font-medium">{tenant.name}</span>
          </p>
        </div>

        <Dialog open={isCreateOpen} onOpenChange={setIsCreateOpen}>
          <DialogTrigger asChild>
            <Button>
              <PlusIcon className="mr-2 h-4 w-4" /> Invite User
            </Button>
          </DialogTrigger>
          <DialogContent className="max-w-lg">
            <DialogHeader>
              <DialogTitle>Invite User</DialogTitle>
              <DialogDescription>
                Send an invitation email to join {tenant.name}
              </DialogDescription>
            </DialogHeader>
            <Form method="post" className="space-y-4">
              <input type="hidden" name="intent" value="create" />

              <div className="space-y-2">
                <Label htmlFor="email">Email Address</Label>
                <Input
                  id="email"
                  name="email"
                  type="email"
                  placeholder="user@example.com"
                  required
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="expires_in_hours">Expires In</Label>
                <Select name="expires_in_hours" defaultValue="72">
                  <SelectTrigger>
                    <SelectValue placeholder="Select expiration" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="24">24 hours</SelectItem>
                    <SelectItem value="48">48 hours</SelectItem>
                    <SelectItem value="72">72 hours (default)</SelectItem>
                    <SelectItem value="168">7 days</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-3">
                <Label>Assign Roles</Label>
                {roles.length === 0 ? (
                  <p className="text-sm text-[var(--text-secondary)]">
                    {servicesCount === 0
                      ? "No services configured for this tenant. Please create a service first."
                      : "Services exist but no roles are defined. Please create roles for your services first."}
                  </p>
                ) : (
                  <div className="space-y-4 max-h-60 overflow-y-auto border rounded-md p-3">
                    {roles.map((serviceGroup) => (
                      <div key={serviceGroup.serviceId}>
                        <p className="text-xs font-medium text-[var(--text-secondary)] uppercase tracking-wider mb-2">
                          {serviceGroup.serviceName}
                        </p>
                        {serviceGroup.roles.length === 0 ? (
                          <p className="text-sm text-[var(--text-tertiary)] italic">No roles defined</p>
                        ) : (
                          <div className="space-y-2">
                            {serviceGroup.roles.map((role) => (
                              <div key={role.id} className="flex items-center space-x-2">
                                <Checkbox
                                  id={`role_${role.id}`}
                                  name={`role_${role.id}`}
                                  checked={selectedRoles.has(role.id)}
                                  onCheckedChange={() => handleRoleToggle(role.id)}
                                />
                                <Label
                                  htmlFor={`role_${role.id}`}
                                  className="font-normal cursor-pointer flex-1"
                                >
                                  <span className="font-medium">{role.name}</span>
                                  {role.description && (
                                    <span className="text-[var(--text-secondary)] text-sm ml-2">
                                      - {role.description}
                                    </span>
                                  )}
                                </Label>
                              </div>
                            ))}
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                )}
              </div>

              {actionData && "error" in actionData && (
                <p className="text-sm text-[var(--accent-red)]">{String(actionData.error)}</p>
              )}

              <DialogFooter>
                <Button type="button" variant="outline" onClick={() => setIsCreateOpen(false)}>
                  Cancel
                </Button>
                <Button type="submit" disabled={isSubmitting || selectedRoles.size === 0}>
                  {isSubmitting ? "Sending..." : "Send Invitation"}
                </Button>
              </DialogFooter>
            </Form>
          </DialogContent>
        </Dialog>
      </div>

      {actionData && "success" in actionData && actionData.success && "message" in actionData && (
        <div className="rounded-xl bg-[var(--accent-green)]/10 border border-[var(--accent-green)]/20 p-4 text-sm text-[var(--accent-green)]">
          {(actionData as { success: boolean; message: string }).message}
        </div>
      )}

      <Card>
        <CardHeader>
          <CardTitle>Pending & Past Invitations</CardTitle>
          <CardDescription>
            {pagination.total} invitations • Page {pagination.page} of {pagination.total_pages}
          </CardDescription>
        </CardHeader>
        <div className="px-6 pb-6">
          <div className="overflow-hidden rounded-xl border border-[var(--glass-border-subtle)]">
            <table className="min-w-full divide-y divide-[var(--glass-border-subtle)] text-sm">
              <thead className="bg-[var(--sidebar-item-hover)] text-left text-[var(--text-secondary)]">
                <tr>
                  <th className="px-4 py-3 font-medium">Email</th>
                  <th className="px-4 py-3 font-medium">Status</th>
                  <th className="px-4 py-3 font-medium">Roles</th>
                  <th className="px-4 py-3 font-medium">Expires At</th>
                  <th className="px-4 py-3 font-medium">Created</th>
                  <th className="px-4 py-3 font-medium w-10"></th>
                </tr>
              </thead>
              <tbody className="divide-y divide-[var(--glass-border-subtle)]">
                {invitations.map((invitation) => (
                  <tr key={invitation.id} className="text-[var(--text-secondary)] hover:bg-[var(--sidebar-item-hover)]/50">
                    <td className="px-4 py-3 font-medium text-[var(--text-primary)]">
                      {invitation.email}
                    </td>
                    <td className="px-4 py-3">
                      {getStatusBadge(invitation.status)}
                    </td>
                    <td className="px-4 py-3 text-xs text-[var(--text-secondary)]">
                      {invitation.role_ids.length} role{invitation.role_ids.length !== 1 ? "s" : ""}
                    </td>
                    <td className="px-4 py-3 text-[var(--text-secondary)]">
                      {formatDateTime(invitation.expires_at)}
                    </td>
                    <td className="px-4 py-3 text-[var(--text-secondary)]">
                      {formatDateTime(invitation.created_at)}
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
                          {invitation.status === "pending" && (
                            <>
                              <DropdownMenuItem onClick={() => handleResend(invitation.id)}>
                                <ReloadIcon className="mr-2 h-3.5 w-3.5" /> Resend Email
                              </DropdownMenuItem>
                              <DropdownMenuItem onClick={() => handleRevoke(invitation.id)}>
                                <Cross2Icon className="mr-2 h-3.5 w-3.5" /> Revoke
                              </DropdownMenuItem>
                              <DropdownMenuSeparator />
                            </>
                          )}
                          <DropdownMenuItem
                            className="text-[var(--accent-red)] focus:text-[var(--accent-red)]"
                            onClick={() => handleDelete(invitation.id)}
                          >
                            <TrashIcon className="mr-2 h-3.5 w-3.5" /> Delete
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </td>
                  </tr>
                ))}
                {invitations.length === 0 && (
                  <tr>
                    <td className="px-4 py-6 text-center text-[var(--text-secondary)]" colSpan={6}>
                      No invitations found. Click &quot;Invite User&quot; to send an invitation.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>

          {/* Pagination */}
          {pagination.total_pages > 1 && (
            <div className="flex items-center justify-center gap-2 mt-4">
              {Array.from({ length: pagination.total_pages }, (_, i) => i + 1).map((page) => (
                <Link
                  key={page}
                  to={`/dashboard/tenants/${params.tenantId}/invitations?page=${page}`}
                  className={`px-3 py-1 text-sm rounded-md ${
                    page === pagination.page
                      ? "bg-apple-blue text-white"
                      : "bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)] hover:bg-[var(--sidebar-item-hover)]"
                  }`}
                >
                  {page}
                </Link>
              ))}
            </div>
          )}
        </div>
      </Card>
    </div>
  );
}
