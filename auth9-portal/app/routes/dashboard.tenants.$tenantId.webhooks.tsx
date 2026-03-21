import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { Form, Link, useActionData, useLoaderData, useNavigation, useSubmit } from "react-router";
import { useState, useEffect, useRef } from "react";
import { useConfirm } from "~/hooks/useConfirm";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Switch } from "~/components/ui/switch";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "~/components/ui/dialog";
import { redirect } from "react-router";
import { webhookApi, tenantApi, type Webhook, type CreateWebhookInput } from "~/services/api";
import { getAccessToken, getAccessTokenWithUpdate } from "~/services/session.server";
import {
  ArrowLeftIcon,
  PlusIcon,
  Pencil2Icon,
  TrashIcon,
  RocketIcon,
  ReloadIcon,
  CopyIcon,
  CheckIcon,
} from "@radix-ui/react-icons";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";
import { mapApiError } from "~/lib/error-messages";
import { useFormatters } from "~/i18n/format";

interface WebhookActionData {
  success?: boolean;
  error?: string;
  message?: string;
  createdSecret?: string;
  newSecret?: string;
}

const WEBHOOK_EVENTS = [
  { id: "login.success", label: "Login Success" },
  { id: "login.failed", label: "Login Failed" },
  { id: "user.created", label: "User Created" },
  { id: "user.updated", label: "User Updated" },
  { id: "user.deleted", label: "User Deleted" },
  { id: "password.changed", label: "Password Changed" },
  { id: "mfa.enabled", label: "MFA Enabled" },
  { id: "mfa.disabled", label: "MFA Disabled" },
  { id: "session.revoked", label: "Session Revoked" },
  { id: "security.alert", label: "Security Alert" },
];

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "tenants.webhooks.metaTitle");
};

export async function loader({ params, request }: LoaderFunctionArgs) {
  const { tenantId } = params;
  const locale = await resolveLocale(request);
  if (!tenantId) {
    return { webhooks: [] as Webhook[], error: translate(locale, "tenants.errors.tenantIdRequired"), tenantId: "" };
  }

  try {
    const { token: accessToken, headers } = await getAccessTokenWithUpdate(request);
    await tenantApi.get(tenantId, accessToken || undefined);
    const response = await webhookApi.list(tenantId, accessToken || undefined);
    const data = { webhooks: response.data, tenantId, error: null as string | null };
    if (headers) {
      return Response.json(data, { headers });
    }
    return data;
  } catch {
    throw redirect("/dashboard/tenants");
  }
}

export async function action({ request, params }: ActionFunctionArgs) {
  const { tenantId } = params;
  const locale = await resolveLocale(request);
  if (!tenantId) {
    return { error: translate(locale, "tenants.errors.tenantIdRequired") };
  }
  const { token: accessToken, headers: sessionHeaders } = await getAccessTokenWithUpdate(request);

  const formData = await request.formData();
  const intent = formData.get("intent");

  const returnData = (data: WebhookActionData): WebhookActionData => {
    if (sessionHeaders) {
      return Response.json(data, { headers: sessionHeaders }) as unknown as WebhookActionData;
    }
    return data;
  };

  try {
    if (intent === "create") {
      const input: CreateWebhookInput = {
        name: formData.get("name") as string,
        url: formData.get("url") as string,
        secret: (formData.get("secret") as string) || undefined,
        events: JSON.parse((formData.get("events") as string) || "[]"),
        enabled: formData.get("enabled") === "true",
      };
      const result = await webhookApi.create(tenantId, input, accessToken || undefined);
      return returnData({
        success: true,
        message: translate(locale, "tenants.webhooks.created"),
        createdSecret: result.data.secret,
      });
    }

    if (intent === "update") {
      const id = formData.get("id") as string;
      const input: Partial<CreateWebhookInput> = {
        name: formData.get("name") as string,
        url: formData.get("url") as string,
        secret: (formData.get("secret") as string) || undefined,
        events: JSON.parse((formData.get("events") as string) || "[]"),
        enabled: formData.get("enabled") === "true",
      };
      await webhookApi.update(tenantId, id, input, accessToken || undefined);
      return returnData({ success: true, message: translate(locale, "tenants.webhooks.updated") });
    }

    if (intent === "delete") {
      const id = formData.get("id") as string;
      await webhookApi.delete(tenantId, id, accessToken || undefined);
      return returnData({ success: true, message: translate(locale, "tenants.webhooks.deleted") });
    }

    if (intent === "regenerate_secret") {
      const id = formData.get("id") as string;
      const result = await webhookApi.regenerateSecret(tenantId, id, accessToken || undefined);
      return returnData({
        success: true,
        message: translate(locale, "tenants.webhooks.secretRegenerated"),
        newSecret: result.data.secret,
      });
    }

    if (intent === "test") {
      const id = formData.get("id") as string;
      const result = await webhookApi.test(tenantId, id, accessToken || undefined);
      if (result.data.success) {
        return returnData({
          success: true,
          message: translate(locale, "tenants.webhooks.testSuccess", {
            statusCode: result.data.status_code,
            responseTime: result.data.response_time_ms,
          }),
        });
      }
      return returnData({ error: translate(locale, "tenants.webhooks.testFailed", { error: result.data.error }) });
    }
  } catch (error) {
    const message = mapApiError(error, locale);
    return returnData({ error: message });
  }

  return returnData({ error: translate(locale, "tenants.webhooks.invalidAction") });
}

export default function WebhooksPage() {
  const { t } = useI18n();
  const formatters = useFormatters();
  const { webhooks, error: loadError, tenantId } = useLoaderData<typeof loader>();
  const actionData = useActionData<WebhookActionData>();
  const navigation = useNavigation();
  const submit = useSubmit();
  const confirm = useConfirm();

  const [showDialog, setShowDialog] = useState(false);
  const [editingWebhook, setEditingWebhook] = useState<Webhook | null>(null);
  const [createdSecret, setCreatedSecret] = useState<string | null>(null);
  const [secretCopied, setSecretCopied] = useState(false);
  const [formData, setFormData] = useState({
    name: "",
    url: "",
    secret: "",
    events: [] as string[],
    enabled: true,
  });

  const isSubmitting = navigation.state === "submitting";
  const dialogSubmitting = useRef(false);

  useEffect(() => {
    if (navigation.state === "submitting" && (showDialog || editingWebhook)) {
      dialogSubmitting.current = true;
    }
  }, [navigation.state, showDialog, editingWebhook]);

  useEffect(() => {
    if (actionData?.success && dialogSubmitting.current) {
      dialogSubmitting.current = false;
      if (actionData.createdSecret) {
        setCreatedSecret(actionData.createdSecret);
        setSecretCopied(false);
        resetForm();
      } else {
        setShowDialog(false);
        setEditingWebhook(null);
        resetForm();
      }
    }
  }, [actionData]);

  function resetForm() {
    setFormData({ name: "", url: "", secret: "", events: [], enabled: true });
  }

  function openEditDialog(webhook: Webhook) {
    setEditingWebhook(webhook);
    setFormData({
      name: webhook.name,
      url: webhook.url,
      secret: webhook.secret || "",
      events: webhook.events,
      enabled: webhook.enabled,
    });
  }

  function openCreateDialog() {
    resetForm();
    setShowDialog(true);
  }

  function toggleEvent(eventId: string) {
    setFormData((prev) => ({
      ...prev,
      events: prev.events.includes(eventId)
        ? prev.events.filter((e) => e !== eventId)
        : [...prev.events, eventId],
    }));
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-3">
        <Button variant="ghost" size="icon" asChild>
          <Link to={`/dashboard/tenants/${tenantId}`} aria-label={t("tenants.actions.backToList")}>
            <ArrowLeftIcon className="h-4 w-4" />
          </Link>
        </Button>
        <div>
          <h1 className="text-[24px] font-semibold tracking-tight text-[var(--text-primary)]">{t("tenants.webhooks.title")}</h1>
          <p className="text-sm text-[var(--text-secondary)]">{t("tenants.webhooks.description")}</p>
        </div>
      </div>

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>{t("tenants.webhooks.title")}</CardTitle>
              <CardDescription>{t("tenants.webhooks.description")}</CardDescription>
            </div>
            <Button onClick={openCreateDialog}>
              <PlusIcon className="h-4 w-4 mr-2" />
              {t("tenants.webhooks.addWebhook")}
            </Button>
          </div>
        </CardHeader>
      </Card>

      {loadError && <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md">{loadError}</div>}
      {actionData?.error && <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md">{actionData.error}</div>}
      {actionData?.success && actionData.message && (
        <div className="text-sm text-[var(--accent-green)] bg-[var(--accent-green)]/10 p-3 rounded-md">
          {actionData.message}
          {actionData.newSecret && (
            <div className="mt-2 flex items-center gap-2">
              <span className="text-[var(--text-primary)]">{t("tenants.webhooks.newSecret")}</span>
              <code className="bg-[var(--bg-secondary)] px-2 py-1 rounded text-xs font-mono break-all select-all text-[var(--text-primary)]">
                {actionData.newSecret}
              </code>
            </div>
          )}
        </div>
      )}

      <Card>
        <CardContent className="pt-6">
          {webhooks.length === 0 ? (
            <div className="text-center py-12">
              <RocketIcon className="h-12 w-12 text-gray-300 mx-auto mb-4" />
              <h3 className="text-lg font-medium text-[var(--text-primary)] mb-2">{t("tenants.webhooks.emptyTitle")}</h3>
              <p className="text-[var(--text-secondary)] mb-4">{t("tenants.webhooks.emptyDescription")}</p>
              <Button onClick={openCreateDialog}>
                <PlusIcon className="h-4 w-4 mr-2" />
                {t("tenants.webhooks.addFirstWebhook")}
              </Button>
            </div>
          ) : (
            <div className="divide-y">
              {webhooks.map((webhook: Webhook) => (
                <div key={webhook.id} className="flex items-center gap-4 py-4 first:pt-0 last:pb-0">
                  <div className={`w-3 h-3 rounded-full ${webhook.enabled ? "bg-[var(--accent-green)]/100" : "bg-gray-300"}`} />
                  <div className="flex-1 min-w-0">
                    <div className="font-medium">{webhook.name}</div>
                    <div className="text-sm text-[var(--text-secondary)] truncate">{webhook.url}</div>
                    <div className="text-xs text-[var(--text-tertiary)] mt-1" suppressHydrationWarning>
                      {t("tenants.webhooks.list.eventCount", { count: webhook.events.length })} •{" "}
                      {webhook.failure_count > 0 && (
                        <span className="text-[var(--accent-red)]">
                          {t("tenants.webhooks.list.failures", { count: webhook.failure_count })} •{" "}
                        </span>
                      )}
                      {webhook.last_triggered_at
                        ? t("tenants.webhooks.list.lastTriggered", {
                            date: formatters.dateTime(webhook.last_triggered_at),
                          })
                        : t("tenants.webhooks.list.neverTriggered")}
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <Form method="post">
                      <input type="hidden" name="intent" value="test" />
                      <input type="hidden" name="id" value={webhook.id} />
                      <Button type="submit" variant="outline" size="sm" disabled={isSubmitting || !webhook.enabled}>
                        {t("tenants.webhooks.test")}
                      </Button>
                    </Form>
                    <Button
                      variant="ghost"
                      size="sm"
                      title={t("tenants.webhooks.list.regenerateSecret")}
                      aria-label={t("tenants.webhooks.list.regenerateSecret")}
                      disabled={isSubmitting}
                      onClick={async () => {
                        const ok = await confirm({
                          title: t("tenants.webhooks.list.regenerateTitle"),
                          description: t("tenants.webhooks.list.regenerateDescription"),
                          variant: "destructive",
                        });
                        if (ok) {
                          submit({ intent: "regenerate_secret", id: webhook.id }, { method: "post" });
                        }
                      }}
                    >
                      <ReloadIcon className="h-4 w-4" />
                    </Button>
                    <Button variant="ghost" size="sm" onClick={() => openEditDialog(webhook)} aria-label={t("common.buttons.edit")}>
                      <Pencil2Icon className="h-4 w-4" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="text-[var(--accent-red)] hover:text-[var(--accent-red)] hover:bg-[var(--accent-red)]/10"
                      aria-label={t("common.buttons.delete")}
                      disabled={isSubmitting}
                      onClick={async () => {
                        const ok = await confirm({
                          title: t("tenants.webhooks.list.deleteTitle"),
                          description: t("tenants.webhooks.list.deleteDescription"),
                          variant: "destructive",
                        });
                        if (ok) {
                          submit({ intent: "delete", id: webhook.id }, { method: "post" });
                        }
                      }}
                    >
                      <TrashIcon className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      <Dialog open={!!createdSecret} onOpenChange={(open) => {
        if (!open) {
          setCreatedSecret(null);
          setShowDialog(false);
        }
      }}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>{t("tenants.webhooks.createdDialog.title")}</DialogTitle>
            <DialogDescription>{t("tenants.webhooks.createdDialog.description")}</DialogDescription>
          </DialogHeader>
          <div className="space-y-3">
            <Label>{t("tenants.webhooks.createdDialog.signingSecret")}</Label>
            <div className="flex items-center gap-2">
              <code className="flex-1 bg-[var(--bg-secondary)] px-3 py-2 rounded-md text-sm font-mono break-all select-all">{createdSecret}</code>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => {
                  navigator.clipboard.writeText(createdSecret || "");
                  setSecretCopied(true);
                  setTimeout(() => setSecretCopied(false), 2000);
                }}
              >
                {secretCopied ? <CheckIcon className="h-4 w-4 text-[var(--accent-green)]" /> : <CopyIcon className="h-4 w-4" />}
              </Button>
            </div>
            <p className="text-xs text-[var(--text-secondary)]">{t("tenants.webhooks.createdDialog.help")}</p>
          </div>
          <DialogFooter>
            <Button onClick={() => {
              setCreatedSecret(null);
              setShowDialog(false);
            }}>
              {t("tenants.webhooks.createdDialog.done")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={(showDialog || !!editingWebhook) && !createdSecret} onOpenChange={(open) => {
        if (!open) {
          setShowDialog(false);
          setEditingWebhook(null);
          resetForm();
        }
      }}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>{editingWebhook ? t("tenants.webhooks.dialog.editTitle") : t("tenants.webhooks.dialog.createTitle")}</DialogTitle>
            <DialogDescription>
              {editingWebhook ? t("tenants.webhooks.dialog.editDescription") : t("tenants.webhooks.dialog.createDescription")}
            </DialogDescription>
          </DialogHeader>

          <Form method="post" className="space-y-4">
            <input type="hidden" name="intent" value={editingWebhook ? "update" : "create"} />
            {editingWebhook && <input type="hidden" name="id" value={editingWebhook.id} />}
            <input type="hidden" name="events" value={JSON.stringify(formData.events)} />
            <input type="hidden" name="enabled" value={formData.enabled ? "true" : "false"} />

            <div className="space-y-2">
              <Label htmlFor="name">{t("tenants.webhooks.dialog.name")}</Label>
              <Input id="name" name="name" value={formData.name} onChange={(e) => setFormData((prev) => ({ ...prev, name: e.target.value }))} placeholder={t("tenants.webhooks.dialog.placeholderName")} required />
            </div>

            <div className="space-y-2">
              <Label htmlFor="url">{t("tenants.webhooks.dialog.endpointUrl")}</Label>
              <Input id="url" name="url" type="url" value={formData.url} onChange={(e) => setFormData((prev) => ({ ...prev, url: e.target.value }))} placeholder={t("tenants.webhooks.dialog.placeholderUrl")} required />
            </div>

            <div className="space-y-2">
              <Label htmlFor="secret">{t("tenants.webhooks.dialog.secret")}</Label>
              <Input id="secret" name="secret" type="password" value={formData.secret} onChange={(e) => setFormData((prev) => ({ ...prev, secret: e.target.value }))} placeholder={t("tenants.webhooks.dialog.placeholderSecret")} />
              <p className="text-xs text-[var(--text-secondary)]">{t("tenants.webhooks.dialog.secretHelp")}</p>
            </div>

            <div className="space-y-2">
              <Label>{t("tenants.webhooks.dialog.events")}</Label>
              <div className="grid grid-cols-2 gap-2 max-h-48 overflow-y-auto border rounded-md p-3">
                {WEBHOOK_EVENTS.map((event) => (
                  <label key={event.id} className="flex items-center gap-2 text-sm cursor-pointer">
                    <input type="checkbox" checked={formData.events.includes(event.id)} onChange={() => toggleEvent(event.id)} className="rounded border-gray-300" />
                    {event.label}
                  </label>
                ))}
              </div>
            </div>

            <div className="flex items-center justify-between">
              <Label htmlFor="enabled">{t("tenants.webhooks.dialog.enabled")}</Label>
              <Switch id="enabled" checked={formData.enabled} onCheckedChange={(checked: boolean) => setFormData((prev) => ({ ...prev, enabled: checked }))} />
            </div>

            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => {
                setShowDialog(false);
                setEditingWebhook(null);
                resetForm();
              }}>
                {t("common.buttons.cancel")}
              </Button>
              <Button type="submit" disabled={isSubmitting || formData.events.length === 0}>
                {isSubmitting
                  ? t("tenants.webhooks.dialog.saving")
                  : editingWebhook
                  ? t("tenants.webhooks.dialog.save")
                  : t("tenants.webhooks.dialog.add")}
              </Button>
            </DialogFooter>
          </Form>
        </DialogContent>
      </Dialog>
    </div>
  );
}
