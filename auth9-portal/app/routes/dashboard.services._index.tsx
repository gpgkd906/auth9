import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation, useSubmit } from "react-router";
import { PlusIcon, DotsHorizontalIcon, Pencil2Icon, TrashIcon, CopyIcon } from "@radix-ui/react-icons";
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
import { serviceApi } from "~/services/api";
import { getAccessToken } from "~/services/session.server";
import { formatErrorMessage } from "~/lib/error-messages";
import { FormattedDate } from "~/components/ui/formatted-date";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "services.metaTitle");
};

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const perPage = Number(url.searchParams.get("perPage") || "20");
  const accessToken = await getAccessToken(request);
  const services = await serviceApi.list(undefined, page, perPage, accessToken || undefined);
  return services;
}

export async function action({ request }: ActionFunctionArgs) {
  const locale = await resolveLocale(request);
  const accessToken = await getAccessToken(request);
  const formData = await request.formData();
  const intent = formData.get("intent");

  try {
    if (intent === "create") {
      const name = formData.get("name") as string;
      const clientId = formData.get("client_id") as string;
      const baseUrl = formData.get("base_url") as string;
      const redirectUris = (formData.get("redirect_uris") as string)?.split(",").map((s) => s.trim()).filter(Boolean);
      const logoutUris = (formData.get("logout_uris") as string)?.split(",").map((s) => s.trim()).filter(Boolean);
      const finalClientId = clientId?.trim() || crypto.randomUUID();

      const res = await serviceApi.create(
        {
          name,
          client_id: finalClientId,
          base_url: baseUrl || undefined,
          redirect_uris: redirectUris,
          logout_uris: logoutUris,
        },
        accessToken || undefined
      );

      if (res.data.client) {
        return { success: true, intent, secret: res.data.client.client_secret };
      }
      return { success: true, intent };
    }

    if (intent === "delete") {
      const id = formData.get("id") as string;
      await serviceApi.delete(id, accessToken || undefined);
      return { success: true, intent };
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : translate(locale, "services.errors.unknown");
    return Response.json({ error: message }, { status: 400 });
  }

  return Response.json({ error: translate(locale, "services.errors.invalidIntent") }, { status: 400 });
}

function getServiceStatusLabel(status: string, locale: string) {
  switch (status) {
    case "active":
      return translate(locale as "zh-CN" | "en-US", "services.statuses.active");
    case "inactive":
      return translate(locale as "zh-CN" | "en-US", "services.statuses.inactive");
    case "suspended":
      return translate(locale as "zh-CN" | "en-US", "services.statuses.suspended");
    case "pending":
      return translate(locale as "zh-CN" | "en-US", "services.statuses.pending");
    default:
      return status;
  }
}

export default function ServicesPage() {
  const data = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const submit = useSubmit();
  const confirm = useConfirm();
  const { t, i18n } = useI18n();
  const [isCreateOpen, setIsCreateOpen] = useState(false);
  const [newSecret, setNewSecret] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const isSubmitting = navigation.state === "submitting";

  useEffect(() => {
    if (actionData && "success" in actionData && actionData.success) {
      setIsCreateOpen(false);
      if (actionData.intent === "create" && "secret" in actionData && actionData.secret) {
        setNewSecret(actionData.secret as string);
      }
    }
  }, [actionData]);

  return (
    <div className="space-y-6">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-[24px] font-semibold text-[var(--text-primary)] tracking-tight">{t("services.title")}</h1>
          <p className="text-sm text-[var(--text-secondary)]">{t("services.description")}</p>
        </div>
        <Dialog open={isCreateOpen} onOpenChange={setIsCreateOpen}>
          <DialogTrigger asChild>
            <Button className="w-full sm:w-auto">
              <PlusIcon className="mr-2 h-4 w-4" /> {t("services.registerService")}
            </Button>
          </DialogTrigger>
          <DialogContent className="max-w-md">
            <DialogHeader>
              <DialogTitle>{t("services.registerService")}</DialogTitle>
              <DialogDescription>{t("services.registerDescription")}</DialogDescription>
            </DialogHeader>
            <Form method="post" className="space-y-4">
              <input type="hidden" name="intent" value="create" />
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-1.5">
                  <Label htmlFor="create-name">{t("services.serviceName")}</Label>
                  <Input id="create-name" name="name" placeholder={t("services.serviceNamePlaceholder")} required />
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="create-client-id">
                    {t("services.clientId")} <span className="text-[var(--text-secondary)] text-xs font-normal">({t("services.optional")})</span>
                  </Label>
                  <Input id="create-client-id" name="client_id" placeholder={t("services.clientIdPlaceholder")} />
                </div>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="create-base-url">{t("services.baseUrl")}</Label>
                <Input id="create-base-url" name="base_url" placeholder={t("services.baseUrlPlaceholder")} />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="create-redirect-uris">{t("services.redirectUris")}</Label>
                <Input id="create-redirect-uris" name="redirect_uris" placeholder={t("services.redirectUrisPlaceholder")} />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="create-logout-uris">{t("services.logoutUris")}</Label>
                <Input id="create-logout-uris" name="logout_uris" placeholder={t("services.logoutUrisPlaceholder")} />
              </div>
              {actionData && "error" in actionData && (
                <p className="text-sm text-[var(--accent-red)]">{formatErrorMessage(String(actionData.error))}</p>
              )}
              <DialogFooter>
                <Button type="button" variant="outline" className="bg-[var(--glass-bg)]" onClick={() => setIsCreateOpen(false)}>
                  {t("common.buttons.cancel")}
                </Button>
                <Button type="submit" disabled={isSubmitting}>
                  {isSubmitting ? t("services.registering") : t("services.register")}
                </Button>
              </DialogFooter>
            </Form>
          </DialogContent>
        </Dialog>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>{t("services.registry")}</CardTitle>
          <CardDescription>
            {t("services.registrySummary", {
              total: data.pagination.total,
              page: data.pagination.page,
              totalPages: data.pagination.total_pages,
            })}
          </CardDescription>
        </CardHeader>
        <div className="px-6 pb-6">
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {data.data.map((service) => (
              <div key={service.id} className="h-full liquid-glass p-5 pb-6 flex flex-col gap-3">
                <div className="flex items-center justify-between gap-2">
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-base font-semibold text-[var(--text-primary)]" title={service.name}>
                      {service.name}
                    </p>
                    <p className="mt-1 text-xs text-[var(--text-tertiary)]">{t("services.serviceId", { id: service.id })}</p>
                  </div>
                  <span className="shrink-0 rounded-full bg-[var(--accent-blue)]/10 px-2 py-1 text-[11px] font-medium text-[var(--accent-blue)] capitalize">
                    {getServiceStatusLabel(service.status, i18n.resolvedLanguage || "zh-CN")}
                  </span>
                </div>

                <div className="text-xs text-[var(--text-secondary)]">
                  {t("services.updated", { date: "" }).replace(/\s*$/, "")} <FormattedDate date={service.updated_at} />
                </div>

                <div className="mt-auto [margin-top:auto] flex items-center justify-between gap-2 pt-2">
                  <a
                    href={`/dashboard/services/${service.id}`}
                    className="inline-flex items-center rounded-md border border-[var(--glass-border-subtle)] px-3 py-2 text-xs font-medium text-[var(--text-primary)] hover:bg-[var(--sidebar-item-hover)]"
                  >
                    <Pencil2Icon className="mr-1.5 h-3.5 w-3.5" />
                    {t("services.details")}
                  </a>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button variant="ghost" className="h-9 w-9 p-0">
                        <span className="sr-only">{t("services.openMenu")}</span>
                        <DotsHorizontalIcon className="h-4 w-4" />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuLabel>{t("services.menuActions")}</DropdownMenuLabel>
                      <DropdownMenuItem asChild>
                        <a href={`/dashboard/services/${service.id}`} className="flex items-center cursor-pointer">
                          <Pencil2Icon className="mr-2 h-3.5 w-3.5" /> {t("services.details")}
                        </a>
                      </DropdownMenuItem>
                      <DropdownMenuSeparator />
                      <DropdownMenuItem
                        className="text-[var(--accent-red)] focus:text-[var(--accent-red)]"
                        onClick={async () => {
                          const ok = await confirm({
                            title: t("services.deleteTitle"),
                            description: t("services.deleteDescription"),
                            variant: "destructive",
                          });
                          if (ok) {
                            submit({ intent: "delete", id: service.id }, { method: "post" });
                          }
                        }}
                      >
                        <TrashIcon className="mr-2 h-3.5 w-3.5" /> {t("common.buttons.delete")}
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              </div>
            ))}
            {data.data.length === 0 && (
              <div className="col-span-full rounded-xl border border-[var(--glass-border-subtle)] bg-[var(--glass-bg)] px-4 py-6 text-center text-[var(--text-secondary)]">
                {t("services.noServices")}
              </div>
            )}
          </div>
        </div>
      </Card>

      <Dialog open={!!newSecret} onOpenChange={(open) => !open && setNewSecret(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("services.initialSecretTitle")}</DialogTitle>
            <DialogDescription>{t("services.initialSecretDescription")}</DialogDescription>
          </DialogHeader>
          <div className="p-4 bg-[var(--sidebar-item-hover)] rounded border font-mono text-center break-all [word-break:break-all] select-all">
            {newSecret}
          </div>
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={async () => {
                if (newSecret) {
                  await navigator.clipboard.writeText(newSecret);
                  setCopied(true);
                  setTimeout(() => setCopied(false), 2000);
                }
              }}
            >
              <CopyIcon className="mr-2 h-4 w-4" /> {copied ? "Copied" : "Copy"}
            </Button>
            <Button type="button" onClick={() => setNewSecret(null)}>{t("services.close")}</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
