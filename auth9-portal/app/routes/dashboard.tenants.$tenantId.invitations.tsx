import type { MetaFunction, LoaderFunctionArgs, ActionFunctionArgs } from "react-router";
import { Form, Link, useActionData, useLoaderData, useNavigation, useParams, useSearchParams, useSubmit } from "react-router";
import { PlusIcon, DotsHorizontalIcon, TrashIcon, ReloadIcon, Cross2Icon, ArrowLeftIcon, EnvelopeClosedIcon } from "@radix-ui/react-icons";
import { useEffect, useState } from "react";
import { useConfirm } from "~/hooks/useConfirm";
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
import { redirect } from "react-router";
import { invitationApi, tenantApi, tenantServiceApi, rbacApi, type Invitation, type Role, type Tenant, type InvitationStatusFilter } from "~/services/api";
import { getAccessToken } from "~/services/session.server";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";
import { mapApiError } from "~/lib/error-messages";
import { useFormatters } from "~/i18n/format";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "tenants.invitations.metaTitle");
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
  status: InvitationStatusFilter | "all";
}

export async function loader({ params, request }: LoaderFunctionArgs) {
  const tenantId = params.tenantId;
  const locale = await resolveLocale(request);
  if (!tenantId) {
    throw new Response(translate(locale, "tenants.errors.tenantIdRequired"), { status: 400 });
  }

  const accessToken = await getAccessToken(request);
  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const perPage = Number(url.searchParams.get("perPage") || "20");
  const statusParam = url.searchParams.get("status");
  const status = statusParam && ["pending", "accepted", "expired", "revoked"].includes(statusParam)
    ? (statusParam as InvitationStatusFilter)
    : undefined;

  try {
    const [tenantResult, invitationsResult, servicesResult] = await Promise.all([
      tenantApi.get(tenantId, accessToken || undefined),
      invitationApi.list(tenantId, page, perPage, status, accessToken || undefined),
      tenantServiceApi.getEnabledServices(tenantId, accessToken || undefined),
    ]);

    const rolesPromises = servicesResult.data.map(async (service) => {
      const rolesResult = await rbacApi.listRoles(service.id, accessToken || undefined);
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
      status: status ?? "all",
    } satisfies LoaderData;
  } catch (error) {
    if (error instanceof Response) throw error;
    const status = (error as { status?: number })?.status;
    if (status === 401) {
      throw redirect("/login");
    }
    throw new Response("Failed to load invitation data", { status: status || 500 });
  }
}

export async function action({ params, request }: ActionFunctionArgs) {
  const tenantId = params.tenantId;
  const locale = await resolveLocale(request);
  if (!tenantId) {
    throw new Response(translate(locale, "tenants.errors.tenantIdRequired"), { status: 400 });
  }

  const formData = await request.formData();
  const intent = formData.get("intent");
  const accessToken = await getAccessToken(request);

  if (!accessToken) {
    return Response.json({ error: translate(locale, "tenants.invitations.authRequired") }, { status: 401 });
  }

  try {
    if (intent === "create") {
      const email = formData.get("email") as string;
      const expiresInHours = parseInt(formData.get("expires_in_hours") as string, 10) || 72;
      const roleIds: string[] = [];
      for (const [key, value] of formData.entries()) {
        if (key.startsWith("role_") && value === "on") {
          roleIds.push(key.replace("role_", ""));
        }
      }

      if (roleIds.length === 0) {
        return Response.json({ error: translate(locale, "tenants.invitations.roleRequired") }, { status: 400 });
      }

      await invitationApi.create(tenantId, { email, role_ids: roleIds, expires_in_hours: expiresInHours }, accessToken);
      return { success: true };
    }

    if (intent === "revoke") {
      const id = formData.get("id") as string;
      await invitationApi.revoke(id, accessToken);
      return { success: true };
    }

    if (intent === "resend") {
      const id = formData.get("id") as string;
      await invitationApi.resend(id, accessToken);
      return { success: true, message: translate(locale, "tenants.invitations.resent") };
    }

    if (intent === "delete") {
      const id = formData.get("id") as string;
      await invitationApi.delete(id, accessToken);
      return { success: true };
    }
  } catch (error) {
    const message = mapApiError(error, locale);
    return Response.json({ error: message }, { status: 400 });
  }

  return Response.json({ error: translate(locale, "tenants.errors.invalidIntent") }, { status: 400 });
}

function getStatusBadge(status: Invitation["status"], t: ReturnType<typeof useI18n>["t"]) {
  const styles = {
    pending: "bg-yellow-50 text-yellow-700 border-yellow-200",
    accepted: "bg-[var(--accent-green)]/10 text-[var(--accent-green)] border-[var(--accent-green)]/20",
    expired: "bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)] border-[var(--glass-border-subtle)]",
    revoked: "bg-red-50 text-red-700 border-red-200",
  };

  return (
    <span className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium border ${styles[status]}`}>
      {getInvitationStatusLabel(status, t)}
    </span>
  );
}

function getInvitationStatusLabel(status: Invitation["status"], t: ReturnType<typeof useI18n>["t"]) {
  switch (status) {
    case "pending":
      return t("tenants.statuses.pending");
    case "accepted":
      return "Accepted";
    case "expired":
      return "Expired";
    case "revoked":
      return "Revoked";
    default:
      return status;
  }
}

export default function InvitationsPage() {
  const { t } = useI18n();
  const formatters = useFormatters();
  const { tenant, invitations, pagination, roles, servicesCount, status } = useLoaderData<LoaderData>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const submit = useSubmit();
  const confirm = useConfirm();
  const [searchParams] = useSearchParams();
  const params = useParams();

  const [isCreateOpen, setIsCreateOpen] = useState(false);
  const [selectedRoles, setSelectedRoles] = useState<Set<string>>(new Set());

  const isSubmitting = navigation.state === "submitting";

  useEffect(() => {
    if (actionData && "success" in actionData && actionData.success) {
      setIsCreateOpen(false);
      setSelectedRoles(new Set());
    }
  }, [actionData]);

  const handleRoleToggle = (roleId: string) => {
    setSelectedRoles((prev) => {
      const next = new Set(prev);
      if (next.has(roleId)) next.delete(roleId); else next.add(roleId);
      return next;
    });
  };

  const handleDelete = async (id: string) => {
    const ok = await confirm({
      title: t("tenants.invitations.deleteTitle"),
      description: t("tenants.invitations.deleteDescription"),
      variant: "destructive",
    });
    if (ok) submit({ intent: "delete", id }, { method: "post" });
  };

  const handleRevoke = async (id: string) => {
    const ok = await confirm({
      title: t("tenants.invitations.revokeTitle"),
      description: t("tenants.invitations.revokeDescription"),
      confirmLabel: t("tenants.invitations.revokeConfirm"),
      variant: "destructive",
    });
    if (ok) submit({ intent: "revoke", id }, { method: "post" });
  };

  const handleResend = (id: string) => submit({ intent: "resend", id }, { method: "post" });

  const handleStatusChange = (value: string) => {
    const nextParams = new URLSearchParams(searchParams);
    if (value === "all") nextParams.delete("status"); else nextParams.set("status", value);
    nextParams.delete("page");
    submit(nextParams, { method: "get" });
  };

  const buildPageLink = (page: number) => {
    const nextParams = new URLSearchParams();
    nextParams.set("page", page.toString());
    if (status !== "all") nextParams.set("status", status);
    return `/dashboard/tenants/${params.tenantId}/invitations?${nextParams.toString()}`;
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
          <div>
            <div className="flex items-center gap-3 mb-1">
            <Button variant="ghost" size="icon" asChild>
              <Link to={`/dashboard/tenants/${tenant.id}`} aria-label={t("tenants.actions.backToList")}>
                <ArrowLeftIcon className="h-4 w-4" />
              </Link>
            </Button>
            <h1 className="text-[24px] font-semibold text-[var(--text-primary)] tracking-tight">{t("tenants.invitations.title")}</h1>
          </div>
          <p className="text-sm text-[var(--text-secondary)] ml-8">{t("tenants.invitations.description", { tenantName: tenant.name })}</p>
        </div>

        <Dialog open={isCreateOpen} onOpenChange={setIsCreateOpen}>
          <DialogTrigger asChild>
            <Button>
              <PlusIcon className="mr-2 h-4 w-4" /> {t("tenants.invitations.inviteUser")}
            </Button>
          </DialogTrigger>
          <DialogContent className="max-w-lg">
            <DialogHeader>
              <DialogTitle>{t("tenants.invitations.dialogTitle")}</DialogTitle>
              <DialogDescription>{t("tenants.invitations.dialogDescription", { tenantName: tenant.name })}</DialogDescription>
            </DialogHeader>
            <Form method="post" className="space-y-4">
              <input type="hidden" name="intent" value="create" />

              <div className="space-y-2">
                <Label htmlFor="email">{t("tenants.invitations.emailAddress")}</Label>
                <Input id="email" name="email" type="email" placeholder={t("tenants.invitations.emailPlaceholder")} required />
              </div>

              <div className="space-y-2">
                <Label htmlFor="expires_in_hours">{t("tenants.invitations.expiresIn")}</Label>
                <Select name="expires_in_hours" defaultValue="72">
                  <SelectTrigger><SelectValue placeholder={t("tenants.invitations.selectExpiration")} /></SelectTrigger>
                  <SelectContent>
                    <SelectItem value="24">{t("tenants.invitations.expiration24h")}</SelectItem>
                    <SelectItem value="48">{t("tenants.invitations.expiration48h")}</SelectItem>
                    <SelectItem value="72">{t("tenants.invitations.expiration72h")}</SelectItem>
                    <SelectItem value="168">{t("tenants.invitations.expiration7d")}</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-3">
                <Label>{t("tenants.invitations.assignRoles")}</Label>
                {roles.length === 0 ? (
                  <p className="text-sm text-[var(--text-secondary)]">
                    {servicesCount === 0 ? t("tenants.invitations.noServices") : t("tenants.invitations.noRoles")}
                  </p>
                ) : (
                  <div className="space-y-4 max-h-60 overflow-y-auto border rounded-md p-3">
                    {roles.map((serviceGroup) => (
                      <div key={serviceGroup.serviceId}>
                        <p className="text-xs font-medium text-[var(--text-secondary)] uppercase tracking-wider mb-2">{serviceGroup.serviceName}</p>
                        {serviceGroup.roles.length === 0 ? (
                          <p className="text-sm text-[var(--text-tertiary)] italic">{t("tenants.invitations.noRolesDefined")}</p>
                        ) : (
                          <div className="space-y-2">
                            {serviceGroup.roles.map((role) => (
                              <div key={role.id} className="flex items-center space-x-2">
                                <Checkbox id={`role_${role.id}`} name={`role_${role.id}`} checked={selectedRoles.has(role.id)} onCheckedChange={() => handleRoleToggle(role.id)} />
                                <Label htmlFor={`role_${role.id}`} className="font-normal cursor-pointer flex-1">
                                  <span className="font-medium">{role.name}</span>
                                  {role.description && <span className="text-[var(--text-secondary)] text-sm ml-2">- {role.description}</span>}
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

              {actionData && "error" in actionData && <p className="text-sm text-[var(--accent-red)]">{String(actionData.error)}</p>}

              <DialogFooter>
                <Button type="button" variant="outline" onClick={() => setIsCreateOpen(false)}>{t("tenants.invitations.cancel")}</Button>
                <Button type="submit" disabled={isSubmitting || selectedRoles.size === 0}>
                  {isSubmitting ? t("tenants.invitations.sending") : t("tenants.invitations.sendInvitation")}
                </Button>
              </DialogFooter>
            </Form>
          </DialogContent>
        </Dialog>
      </div>

      {actionData && "success" in actionData && actionData.success && "message" in actionData && (
        <div className="rounded-xl bg-[var(--accent-green)]/10 border border-[var(--accent-green)]/20 p-4 text-sm text-[var(--accent-green)]">{(actionData as { success: boolean; message: string }).message}</div>
      )}

      <Card>
        <CardHeader className="gap-4 sm:flex sm:flex-row sm:items-center sm:justify-between">
          <div>
            <CardTitle>{t("tenants.invitations.listTitle")}</CardTitle>
            <CardDescription>{t("tenants.invitations.listDescription", { total: pagination.total, page: pagination.page, totalPages: pagination.total_pages })}</CardDescription>
          </div>
          <div className="w-full sm:w-56">
            <Label className="text-xs uppercase tracking-wide text-[var(--text-secondary)]">{t("tenants.invitations.statusFilter")}</Label>
            <Select value={status} onValueChange={handleStatusChange}>
              <SelectTrigger className="mt-2"><SelectValue placeholder={t("tenants.invitations.allStatuses")} /></SelectTrigger>
              <SelectContent>
                <SelectItem value="all">{t("tenants.invitations.allStatuses")}</SelectItem>
                <SelectItem value="pending">Pending</SelectItem>
                <SelectItem value="accepted">Accepted</SelectItem>
                <SelectItem value="expired">Expired</SelectItem>
                <SelectItem value="revoked">Revoked</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </CardHeader>
        <div className="px-6 pb-6">
          <div className="overflow-hidden rounded-xl border border-[var(--glass-border-subtle)]">
            {invitations.length === 0 ? (
              <div className="flex flex-col items-center justify-center px-6 py-12 text-center">
                <div className="mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)]">
                  <EnvelopeClosedIcon className="h-6 w-6" />
                </div>
                <h3 className="text-[17px] font-semibold text-[var(--text-primary)]">{t("tenants.invitations.noInvitations")}</h3>
                <p className="mt-2 max-w-md text-[13px] text-[var(--text-secondary)]">
                  {t("tenants.invitations.noInvitationsDescription")}
                </p>
                <Button className="mt-5" onClick={() => setIsCreateOpen(true)}>
                  <PlusIcon className="mr-2 h-4 w-4" />
                  {t("tenants.invitations.sendInvitation")}
                </Button>
              </div>
            ) : (
            <table className="min-w-full divide-y divide-[var(--glass-border-subtle)] text-sm">
              <thead className="bg-[var(--sidebar-item-hover)] text-left text-[var(--text-secondary)]">
                <tr>
                  <th className="px-4 py-3 font-medium">{t("tenants.invitations.email")}</th>
                  <th className="px-4 py-3 font-medium">{t("tenants.invitations.status")}</th>
                  <th className="px-4 py-3 font-medium">{t("tenants.invitations.roles")}</th>
                  <th className="px-4 py-3 font-medium">{t("tenants.invitations.expiresAt")}</th>
                  <th className="px-4 py-3 font-medium">{t("tenants.invitations.created")}</th>
                  <th className="px-4 py-3 font-medium w-10"></th>
                </tr>
              </thead>
              <tbody className="divide-y divide-[var(--glass-border-subtle)]">
                {invitations.map((invitation) => (
                  <tr key={invitation.id} className="text-[var(--text-secondary)] hover:bg-[var(--sidebar-item-hover)]/50">
                    <td className="px-4 py-3 font-medium text-[var(--text-primary)]">{invitation.email}</td>
                    <td className="px-4 py-3">{getStatusBadge(invitation.status, t)}</td>
                    <td className="px-4 py-3 text-xs text-[var(--text-secondary)]">{t("tenants.invitations.roleCount", { count: invitation.role_ids.length })}</td>
                    <td className="px-4 py-3 text-[var(--text-secondary)]">{formatters.dateTime(invitation.expires_at)}</td>
                    <td className="px-4 py-3 text-[var(--text-secondary)]">{formatters.dateTime(invitation.created_at)}</td>
                    <td className="px-4 py-3">
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button variant="ghost" className="h-8 w-8 p-0" aria-label={t("tenants.actions.openMenu")}>
                              <span className="sr-only">{t("tenants.actions.openMenu")}</span>
                              <DotsHorizontalIcon className="h-4 w-4" />
                            </Button>
                          </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuLabel>{t("tenants.invitations.menuActions")}</DropdownMenuLabel>
                          {invitation.status === "pending" && (
                            <>
                              <DropdownMenuItem onClick={() => handleResend(invitation.id)}>
                                <ReloadIcon className="mr-2 h-3.5 w-3.5" /> {t("tenants.invitations.resendEmail")}
                              </DropdownMenuItem>
                              <DropdownMenuItem onClick={() => handleRevoke(invitation.id)}>
                                <Cross2Icon className="mr-2 h-3.5 w-3.5" /> {t("tenants.invitations.revoke")}
                              </DropdownMenuItem>
                              <DropdownMenuSeparator />
                            </>
                          )}
                          <DropdownMenuItem className="text-[var(--accent-red)] focus:text-[var(--accent-red)]" onClick={() => handleDelete(invitation.id)}>
                            <TrashIcon className="mr-2 h-3.5 w-3.5" /> {t("tenants.invitations.delete")}
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            )}
          </div>

          {pagination.total_pages > 1 && (
            <div className="flex items-center justify-center gap-2 mt-4">
              {Array.from({ length: pagination.total_pages }, (_, i) => i + 1).map((page) => (
                <Link key={page} to={buildPageLink(page)} className={`px-3 py-1 text-sm rounded-md ${page === pagination.page ? "bg-apple-blue text-white" : "bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)] hover:bg-[var(--sidebar-item-hover)]"}`}>
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
