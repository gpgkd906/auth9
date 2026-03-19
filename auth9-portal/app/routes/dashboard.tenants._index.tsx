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
import { FormattedDate } from "~/components/ui/formatted-date";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "~/components/ui/dropdown-menu";
import { tenantApi, type Tenant } from "~/services/api";
import { getAccessTokenWithUpdate } from "~/services/session.server";
import { mapApiError } from "~/lib/error-messages";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "tenants.metaTitle");
};

export async function loader({ request }: LoaderFunctionArgs) {
  const { token: accessToken, headers } = await getAccessTokenWithUpdate(request);
  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const perPage = Number(url.searchParams.get("perPage") || "20");
  const search = url.searchParams.get("search") || undefined;
  const tenants = await tenantApi.list(page, perPage, search, accessToken || undefined);
  const data = { ...tenants, search: search || "" };

  if (headers) {
    return Response.json(data, { headers });
  }
  return data;
}

export async function action({ request }: ActionFunctionArgs) {
  const locale = await resolveLocale(request);
  const formData = await request.formData();
  const intent = formData.get("intent");
  const { token: accessToken, headers } = await getAccessTokenWithUpdate(request);

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

      await tenantApi.create({ name, slug, logo_url: logo_url || undefined }, accessToken || undefined);
      return returnSuccess();
    }

    if (intent === "update") {
      const id = formData.get("id") as string;
      const name = formData.get("name") as string;
      const slug = formData.get("slug") as string;
      const logo_url = formData.get("logo_url") as string;

      await tenantApi.update(id, { name, slug, logo_url: logo_url || undefined }, accessToken || undefined);
      return returnSuccess();
    }

    if (intent === "delete") {
      const id = formData.get("id") as string;
      await tenantApi.delete(id, accessToken || undefined);
      return returnSuccess();
    }
  } catch (error) {
    const message = mapApiError(error, locale);
    return returnError(message);
  }

  return returnError(translate(locale, "tenants.errors.invalidIntent"));
}

function getStatusLabel(status: Tenant["status"], t: ReturnType<typeof useI18n>["t"]) {
  return t(`tenants.statuses.${status}`);
}

export default function TenantsIndexPage() {
  const { t } = useI18n();
  const data = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const submit = useSubmit();
  const confirm = useConfirm();
  const [isCreateOpen, setIsCreateOpen] = useState(false);
  const [editingTenant, setEditingTenant] = useState<Tenant | null>(null);
  const [searchValue, setSearchValue] = useState(data.search || "");

  const isSubmitting = navigation.state === "submitting";
  const createError = actionData && "error" in actionData && "intent" in actionData && actionData.intent === "create"
    ? String(actionData.error)
    : null;
  const updateError = actionData && "error" in actionData && "intent" in actionData && actionData.intent === "update"
    ? String(actionData.error)
    : null;

  useEffect(() => {
    if (actionData && "success" in actionData && actionData.success) {
      setIsCreateOpen(false);
      setEditingTenant(null);
    }
  }, [actionData]);

  return (
    <div className="space-y-6">
      <div className="mb-6 flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div className="space-y-2">
          <h1 className="text-[24px] font-semibold text-[var(--text-primary)] tracking-tight">{t("tenants.title")}</h1>
          <p className="text-sm text-[var(--text-secondary)]">{t("tenants.description")}</p>
        </div>
        <Dialog open={isCreateOpen} onOpenChange={setIsCreateOpen}>
          <DialogTrigger asChild>
            <Button className="w-full sm:w-auto">
              <PlusIcon className="mr-2 h-4 w-4" /> {t("tenants.actions.create")}
            </Button>
          </DialogTrigger>
          <DialogContent aria-modal="true">
            <DialogHeader>
              <DialogTitle>{t("tenants.createTitle")}</DialogTitle>
              <DialogDescription>{t("tenants.createDescription")}</DialogDescription>
            </DialogHeader>
            <Form method="post" className="space-y-4">
              <input type="hidden" name="intent" value="create" />
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="create-name">{t("tenants.fields.name")}</Label>
                <Input
                  id="create-name"
                  name="name"
                  placeholder={t("tenants.placeholders.name")}
                  required
                  aria-required="true"
                  aria-describedby={createError ? "create-name-help create-tenant-form-error" : "create-name-help"}
                  aria-invalid={createError ? true : undefined}
                  aria-errormessage={createError ? "create-tenant-form-error" : undefined}
                />
                <p id="create-name-help" className="text-xs text-[var(--text-tertiary)]">{t("tenants.help.name")}</p>
              </div>
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="create-slug">{t("tenants.fields.slug")}</Label>
                <Input
                  id="create-slug"
                  name="slug"
                  placeholder={t("tenants.placeholders.slug")}
                  required
                  aria-required="true"
                  aria-describedby={createError ? "create-slug-help create-tenant-form-error" : "create-slug-help"}
                  aria-invalid={createError ? true : undefined}
                  aria-errormessage={createError ? "create-tenant-form-error" : undefined}
                />
                <p id="create-slug-help" className="text-xs text-[var(--text-tertiary)]">{t("tenants.help.slug")}</p>
              </div>
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="create-logo">{t("tenants.fields.logoUrl")}</Label>
                <Input
                  id="create-logo"
                  name="logo_url"
                  placeholder={t("tenants.placeholders.logoUrl")}
                  aria-describedby={createError ? "create-logo-help create-tenant-form-error" : "create-logo-help"}
                  aria-invalid={createError ? true : undefined}
                  aria-errormessage={createError ? "create-tenant-form-error" : undefined}
                />
                <p id="create-logo-help" className="text-xs text-[var(--text-tertiary)]">{t("tenants.help.logoUrl")}</p>
              </div>
              {createError && (
                <p id="create-tenant-form-error" className="text-sm text-[var(--accent-red)]">
                  {createError}
                </p>
              )}
              <DialogFooter className="-mx-6 sm:mx-0">
                <Button
                  type="button"
                  variant="outline"
                  className="w-full bg-[var(--glass-bg)] sm:w-auto"
                  onClick={() => setIsCreateOpen(false)}
                >
                  {t("common.buttons.cancel")}
                </Button>
                <Button type="submit" className="w-full sm:w-auto" disabled={isSubmitting}>
                  {isSubmitting ? t("tenants.actions.creating") : t("tenants.actions.createSubmit")}
                </Button>
              </DialogFooter>
            </Form>
          </DialogContent>
        </Dialog>
      </div>

      <Card>
        <CardHeader className="pb-4">
          <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between sm:gap-4">
            <div>
              <CardTitle>{t("tenants.listTitle")}</CardTitle>
              <CardDescription>
                {t("tenants.listDescription", {
                  total: data.pagination.total,
                  page: data.pagination.page,
                  totalPages: data.pagination.total_pages,
                })}
              </CardDescription>
            </div>
            <Form method="get" className="flex flex-col md:flex-row items-stretch md:items-center gap-2 w-full md:w-auto">
              <div className="relative w-full md:w-auto">
                <MagnifyingGlassIcon className="absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-[var(--text-tertiary)]" />
                <Input
                  name="search"
                  placeholder={t("tenants.placeholders.search")}
                  aria-label={t("tenants.placeholders.search")}
                  value={searchValue}
                  onChange={(e) => setSearchValue(e.target.value)}
                  className="w-full md:w-[200px] pl-8"
                />
              </div>
              <div className="flex gap-2 w-full md:w-auto">
                <Button type="submit" variant="outline" className="w-full bg-[var(--glass-bg)] md:w-auto" size="default">
                  {t("tenants.actions.search")}
                </Button>
                {data.search && (
                  <Button
                    type="button"
                    variant="ghost"
                    size="default"
                    className="w-full md:w-auto"
                    onClick={() => {
                      setSearchValue("");
                      window.location.href = "/dashboard/tenants";
                    }}
                  >
                    {t("tenants.actions.clear")}
                  </Button>
                )}
              </div>
            </Form>
          </div>
        </CardHeader>
        <div className="px-6 pb-6 pt-6">
          <div className="space-y-3 md:hidden">
            {data.data.map((tenant) => (
              <div
                key={tenant.id}
                className="rounded-xl border border-[var(--glass-border-subtle)] bg-[var(--glass-bg)] p-4"
              >
                <Link to={`/dashboard/tenants/${tenant.id}`} className="block hover:underline">
                  <p className="text-sm font-semibold text-[var(--text-primary)]">{tenant.name}</p>
                  <p className="mt-1 text-xs text-[var(--text-tertiary)]">{tenant.slug}</p>
                </Link>
                <div className="mt-3 flex items-center gap-2">
                  <span className="text-xs text-[var(--text-tertiary)]">{t("tenants.fields.status")}</span>
                  <span className="inline-flex items-center rounded-full bg-[var(--accent-blue)]/10 px-2 py-1 text-[11px] font-medium text-[var(--accent-blue)] capitalize">
                    {getStatusLabel(tenant.status, t)}
                  </span>
                </div>
                <p className="mt-2 text-xs text-[var(--text-tertiary)]">
                  {t("tenants.fields.updated")} <FormattedDate date={tenant.updated_at} />
                </p>
                <div className="mt-4 flex items-center gap-2">
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className="h-11 flex-1"
                    onClick={() => setEditingTenant(tenant)}
                  >
                    <Pencil2Icon className="mr-2 h-3.5 w-3.5" />
                    {t("tenants.actions.edit")}
                  </Button>
                  <Button asChild variant="outline" size="sm" className="h-11 flex-1">
                    <Link to={`/dashboard/tenants/${tenant.id}/invitations`}>{t("tenants.actions.invitations")}</Link>
                  </Button>
                  <Button
                    type="button"
                    variant="destructive"
                    size="sm"
                    className="h-11"
                    onClick={async () => {
                      const ok = await confirm({
                        title: t("tenants.delete.title"),
                        description: (
                          <>
                            {t("tenants.delete.descriptionLead")}{" "}
                            <strong className="font-semibold text-[var(--text-primary)]">{tenant.name}</strong>
                            {t("tenants.delete.descriptionTail")}
                          </>
                        ),
                        variant: "destructive",
                      });
                      if (ok) {
                        submit({ intent: "delete", id: tenant.id }, { method: "post" });
                      }
                    }}
                  >
                    <TrashIcon className="h-3.5 w-3.5" />
                  </Button>
                </div>
              </div>
            ))}
            {data.data.length === 0 && (
              <div className="rounded-xl border border-[var(--glass-border-subtle)] bg-[var(--glass-bg)] px-6 py-10 text-center">
                <MagnifyingGlassIcon className="mx-auto h-8 w-8 text-[var(--text-tertiary)]" />
                <p className="mt-3 text-sm font-medium text-[var(--text-primary)]">
                  {data.search ? t("tenants.emptySearchTitle") : t("tenants.empty")}
                </p>
                {data.search && (
                  <p className="mt-1 text-sm text-[var(--text-tertiary)]">{t("tenants.emptySearchDescription")}</p>
                )}
              </div>
            )}
          </div>

          <div className="hidden overflow-hidden rounded-xl border border-[var(--glass-border-subtle)] md:block">
            <table className="min-w-full divide-y divide-[var(--glass-border-subtle)] text-sm">
              <thead className="bg-[var(--sidebar-item-hover)] text-left text-[var(--text-tertiary)] uppercase tracking-[0.04em] text-[11px]">
                <tr>
                  <th className="px-4 py-3 font-semibold">{t("tenants.fields.name")}</th>
                  <th className="px-4 py-3 font-semibold">{t("tenants.fields.slug")}</th>
                  <th className="px-4 py-3 font-semibold">{t("tenants.fields.status")}</th>
                  <th className="px-4 py-3 font-semibold">{t("tenants.fields.updated")}</th>
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
                    <td className="px-4 py-3 capitalize">{getStatusLabel(tenant.status, t)}</td>
                    <td className="px-4 py-3">
                      <FormattedDate date={tenant.updated_at} />
                    </td>
                    <td className="px-4 py-3">
                        <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" className="h-8 w-8 p-0" aria-label={t("tenants.actions.openMenu")}>
                            <span className="sr-only">{t("tenants.actions.openMenu")}</span>
                            <DotsHorizontalIcon className="h-4 w-4" />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuLabel>{t("tenants.menu.actions")}</DropdownMenuLabel>
                          <DropdownMenuItem onClick={() => setEditingTenant(tenant)}>
                            <Pencil2Icon className="mr-2 h-3.5 w-3.5" /> {t("tenants.actions.edit")}
                          </DropdownMenuItem>
                          <DropdownMenuItem asChild>
                            <Link to={`/dashboard/tenants/${tenant.id}/invitations`}>
                              <EnvelopeClosedIcon className="mr-2 h-3.5 w-3.5" /> {t("tenants.actions.invitations")}
                            </Link>
                          </DropdownMenuItem>
                          <DropdownMenuItem asChild>
                            <Link to={`/dashboard/tenants/${tenant.id}/webhooks`}>
                              <Link2Icon className="mr-2 h-3.5 w-3.5" /> {t("tenants.actions.webhooks")}
                            </Link>
                          </DropdownMenuItem>
                          <DropdownMenuSeparator />
                          <DropdownMenuItem
                            className="text-[var(--accent-red)] focus:text-[var(--accent-red)]"
                            onClick={async () => {
                              const ok = await confirm({
                                title: t("tenants.delete.title"),
                                description: (
                                  <>
                                    {t("tenants.delete.descriptionLead")}{" "}
                                    <strong className="font-semibold text-[var(--text-primary)]">{tenant.name}</strong>
                                    {t("tenants.delete.descriptionTail")}
                                  </>
                                ),
                                variant: "destructive",
                              });
                              if (ok) {
                                submit({ intent: "delete", id: tenant.id }, { method: "post" });
                              }
                            }}
                          >
                            <TrashIcon className="mr-2 h-3.5 w-3.5" /> {t("common.buttons.delete")}
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </td>
                  </tr>
                ))}
                {data.data.length === 0 && (
                  <tr>
                    <td className="px-4 py-12 text-center" colSpan={5}>
                      <MagnifyingGlassIcon className="mx-auto h-8 w-8 text-[var(--text-tertiary)]" />
                      <p className="mt-3 text-sm font-medium text-[var(--text-primary)]">
                        {data.search ? t("tenants.emptySearchTitle") : t("tenants.empty")}
                      </p>
                      {data.search && (
                        <p className="mt-1 text-sm text-[var(--text-tertiary)]">{t("tenants.emptySearchDescription")}</p>
                      )}
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>

          {data.pagination.total_pages > 1 && (
            <div className="flex items-center justify-between mt-4 pt-4 border-t border-[var(--glass-border-subtle)]">
              <div className="text-sm text-[var(--text-secondary)]">
                {t("tenants.pagination.page", { page: data.pagination.page, totalPages: data.pagination.total_pages })}
              </div>
              <div className="flex gap-2">
                {data.pagination.page > 1 && (
                  <Link to={`?page=${data.pagination.page - 1}${data.search ? `&search=${encodeURIComponent(data.search)}` : ""}`}>
                    <Button variant="outline" size="sm" className="bg-[var(--glass-bg)]">
                      {t("tenants.pagination.previous")}
                    </Button>
                  </Link>
                )}
                {data.pagination.page < data.pagination.total_pages && (
                  <Link to={`?page=${data.pagination.page + 1}${data.search ? `&search=${encodeURIComponent(data.search)}` : ""}`}>
                    <Button variant="outline" size="sm" className="bg-[var(--glass-bg)]">
                      {t("tenants.pagination.next")}
                    </Button>
                  </Link>
                )}
              </div>
            </div>
          )}
        </div>
      </Card>

      <Dialog open={!!editingTenant} onOpenChange={(open) => !open && setEditingTenant(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("tenants.editTitle")}</DialogTitle>
            <DialogDescription>{t("tenants.editDescription")}</DialogDescription>
          </DialogHeader>
          <Form method="post" className="space-y-4">
            <input type="hidden" name="intent" value="update" />
            <input type="hidden" name="id" value={editingTenant?.id || ""} />
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="edit-name">{t("tenants.fields.name")}</Label>
              <Input
                id="edit-name"
                name="name"
                defaultValue={editingTenant?.name}
                required
                aria-required="true"
                aria-invalid={updateError ? true : undefined}
                aria-errormessage={updateError ? "edit-tenant-form-error" : undefined}
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="edit-slug">{t("tenants.fields.slug")}</Label>
              <Input
                id="edit-slug"
                name="slug"
                defaultValue={editingTenant?.slug}
                required
                aria-required="true"
                aria-invalid={updateError ? true : undefined}
                aria-errormessage={updateError ? "edit-tenant-form-error" : undefined}
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="edit-logo">{t("tenants.fields.logoUrl")}</Label>
              <Input
                id="edit-logo"
                name="logo_url"
                defaultValue={editingTenant?.logo_url}
                aria-invalid={updateError ? true : undefined}
                aria-errormessage={updateError ? "edit-tenant-form-error" : undefined}
              />
            </div>
            {updateError && (
              <p id="edit-tenant-form-error" className="text-sm text-[var(--accent-red)]">
                {updateError}
              </p>
            )}
            <DialogFooter>
              <Button type="button" variant="outline" className="bg-[var(--glass-bg)]" onClick={() => setEditingTenant(null)}>
                {t("common.buttons.cancel")}
              </Button>
              <Button type="submit" disabled={isSubmitting}>
                {isSubmitting ? t("tenants.actions.saving") : t("tenants.actions.save")}
              </Button>
            </DialogFooter>
          </Form>
        </DialogContent>
      </Dialog>
    </div>
  );
}
