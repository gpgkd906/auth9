import type { ActionFunctionArgs, LoaderFunctionArgs } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation, useSubmit } from "react-router";
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
import { getAccessToken } from "~/services/session.server";
import {
  PlusIcon,
  Pencil2Icon,
  TrashIcon,
  RocketIcon,
  ReloadIcon,
} from "@radix-ui/react-icons";

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

export async function loader({ params, request }: LoaderFunctionArgs) {
  const { tenantId } = params;
  if (!tenantId) {
    return { webhooks: [], error: "Tenant ID is required" };
  }

  try {
    const accessToken = await getAccessToken(request);
    // Validate tenant exists first
    await tenantApi.get(tenantId, accessToken || undefined);
    const response = await webhookApi.list(tenantId, accessToken || undefined);
    return { webhooks: response.data, tenantId };
  } catch {
    throw redirect("/dashboard/tenants");
  }
}

export async function action({ request, params }: ActionFunctionArgs) {
  const { tenantId } = params;
  if (!tenantId) {
    return { error: "Tenant ID is required" };
  }
  const accessToken = await getAccessToken(request);

  const formData = await request.formData();
  const intent = formData.get("intent");

  try {
    if (intent === "create") {
      const input: CreateWebhookInput = {
        name: formData.get("name") as string,
        url: formData.get("url") as string,
        secret: (formData.get("secret") as string) || undefined,
        events: JSON.parse(formData.get("events") as string || "[]"),
        enabled: formData.get("enabled") === "true",
      };
      await webhookApi.create(tenantId, input, accessToken || undefined);
      return { success: true, message: "Webhook created" };
    }

    if (intent === "update") {
      const id = formData.get("id") as string;
      const input: Partial<CreateWebhookInput> = {
        name: formData.get("name") as string,
        url: formData.get("url") as string,
        secret: (formData.get("secret") as string) || undefined,
        events: JSON.parse(formData.get("events") as string || "[]"),
        enabled: formData.get("enabled") === "true",
      };
      await webhookApi.update(tenantId, id, input, accessToken || undefined);
      return { success: true, message: "Webhook updated" };
    }

    if (intent === "delete") {
      const id = formData.get("id") as string;
      await webhookApi.delete(tenantId, id, accessToken || undefined);
      return { success: true, message: "Webhook deleted" };
    }

    if (intent === "regenerate_secret") {
      const id = formData.get("id") as string;
      const result = await webhookApi.regenerateSecret(tenantId, id, accessToken || undefined);
      return { success: true, message: "Secret regenerated", newSecret: result.data.secret };
    }

    if (intent === "test") {
      const id = formData.get("id") as string;
      const result = await webhookApi.test(tenantId, id, accessToken || undefined);
      if (result.data.success) {
        return {
          success: true,
          message: `Test successful (${result.data.status_code}, ${result.data.response_time_ms}ms)`,
        };
      } else {
        return { error: `Test failed: ${result.data.error}` };
      }
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : "Operation failed";
    return { error: message };
  }

  return { error: "Invalid action" };
}

export default function WebhooksPage() {
  const { webhooks, error: loadError } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const submit = useSubmit();
  const confirm = useConfirm();

  const [showDialog, setShowDialog] = useState(false);
  const [editingWebhook, setEditingWebhook] = useState<Webhook | null>(null);
  const [formData, setFormData] = useState({
    name: "",
    url: "",
    secret: "",
    events: [] as string[],
    enabled: true,
  });

  const isSubmitting = navigation.state === "submitting";
  const dialogSubmitting = useRef(false);

  // Track when a form is submitted from within the dialog
  useEffect(() => {
    if (navigation.state === "submitting" && (showDialog || editingWebhook)) {
      dialogSubmitting.current = true;
    }
  }, [navigation.state, showDialog, editingWebhook]);

  // Close dialog on success only if the submission came from the dialog
  useEffect(() => {
    if (actionData?.success && dialogSubmitting.current) {
      dialogSubmitting.current = false;
      setShowDialog(false);
      setEditingWebhook(null);
      resetForm();
    }
  }, [actionData]);

  function resetForm() {
    setFormData({
      name: "",
      url: "",
      secret: "",
      events: [],
      enabled: true,
    });
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
      {/* Header */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>Webhooks</CardTitle>
              <CardDescription>
                Receive real-time notifications for events in your application.
              </CardDescription>
            </div>
            <Button onClick={openCreateDialog}>
              <PlusIcon className="h-4 w-4 mr-2" />
              Add webhook
            </Button>
          </div>
        </CardHeader>
      </Card>

      {/* Messages */}
      {loadError && (
        <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md">
          {loadError}
        </div>
      )}

      {actionData?.error && (
        <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md">
          {actionData.error}
        </div>
      )}

      {actionData?.success && actionData.message && (
        <div className="text-sm text-[var(--accent-green)] bg-[var(--accent-green)]/10 p-3 rounded-md">
          {actionData.message}
        </div>
      )}

      {/* Webhooks List */}
      <Card>
        <CardContent className="pt-6">
          {webhooks.length === 0 ? (
            <div className="text-center py-12">
              <RocketIcon className="h-12 w-12 text-gray-300 mx-auto mb-4" />
              <h3 className="text-lg font-medium text-[var(--text-primary)] mb-2">
                No webhooks configured
              </h3>
              <p className="text-[var(--text-secondary)] mb-4">
                Add a webhook to receive real-time event notifications.
              </p>
              <Button onClick={openCreateDialog}>
                <PlusIcon className="h-4 w-4 mr-2" />
                Add your first webhook
              </Button>
            </div>
          ) : (
            <div className="divide-y">
              {webhooks.map((webhook: Webhook) => (
                <div
                  key={webhook.id}
                  className="flex items-center gap-4 py-4 first:pt-0 last:pb-0"
                >
                  <div
                    className={`w-3 h-3 rounded-full ${
                      webhook.enabled ? "bg-[var(--accent-green)]/100" : "bg-gray-300"
                    }`}
                  />
                  <div className="flex-1 min-w-0">
                    <div className="font-medium">{webhook.name}</div>
                    <div className="text-sm text-[var(--text-secondary)] truncate">
                      {webhook.url}
                    </div>
                    <div className="text-xs text-[var(--text-tertiary)] mt-1" suppressHydrationWarning>
                      {webhook.events.length} events •{" "}
                      {webhook.failure_count > 0 && (
                        <span className="text-[var(--accent-red)]">
                          {webhook.failure_count} failures •{" "}
                        </span>
                      )}
                      {webhook.last_triggered_at
                        ? `Last triggered: ${new Date(
                            webhook.last_triggered_at
                          ).toLocaleString()}`
                        : "Never triggered"}
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <Form method="post">
                      <input type="hidden" name="intent" value="test" />
                      <input type="hidden" name="id" value={webhook.id} />
                      <Button
                        type="submit"
                        variant="outline"
                        size="sm"
                        disabled={isSubmitting || !webhook.enabled}
                      >
                        Test
                      </Button>
                    </Form>
                    <Button
                      variant="ghost"
                      size="sm"
                      title="Regenerate Secret"
                      disabled={isSubmitting}
                      onClick={async () => {
                        const ok = await confirm({
                          title: "Regenerate Secret",
                          description: "Are you sure? The old secret will be invalidated immediately.",
                          variant: "destructive",
                        });
                        if (ok) {
                          submit(
                            { intent: "regenerate_secret", id: webhook.id },
                            { method: "post" }
                          );
                        }
                      }}
                    >
                      <ReloadIcon className="h-4 w-4" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => openEditDialog(webhook)}
                    >
                      <Pencil2Icon className="h-4 w-4" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="text-[var(--accent-red)] hover:text-[var(--accent-red)] hover:bg-[var(--accent-red)]/10"
                      disabled={isSubmitting}
                      onClick={async () => {
                        const ok = await confirm({
                          title: "Delete Webhook",
                          description: "Are you sure you want to delete this webhook? This action cannot be undone.",
                          variant: "destructive",
                        });
                        if (ok) {
                          submit(
                            { intent: "delete", id: webhook.id },
                            { method: "post" }
                          );
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

      {/* Create/Edit Dialog */}
      <Dialog
        open={showDialog || !!editingWebhook}
        onOpenChange={(open) => {
          if (!open) {
            setShowDialog(false);
            setEditingWebhook(null);
            resetForm();
          }
        }}
      >
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>
              {editingWebhook ? "Edit Webhook" : "Add Webhook"}
            </DialogTitle>
            <DialogDescription>
              {editingWebhook
                ? "Update the webhook configuration."
                : "Configure a new webhook endpoint."}
            </DialogDescription>
          </DialogHeader>

          <Form method="post" className="space-y-4">
            <input
              type="hidden"
              name="intent"
              value={editingWebhook ? "update" : "create"}
            />
            {editingWebhook && (
              <input type="hidden" name="id" value={editingWebhook.id} />
            )}
            <input
              type="hidden"
              name="events"
              value={JSON.stringify(formData.events)}
            />
            <input
              type="hidden"
              name="enabled"
              value={formData.enabled ? "true" : "false"}
            />

            <div className="space-y-2">
              <Label htmlFor="name">Name</Label>
              <Input
                id="name"
                name="name"
                value={formData.name}
                onChange={(e) =>
                  setFormData((prev) => ({ ...prev, name: e.target.value }))
                }
                placeholder="My Webhook"
                required
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="url">Endpoint URL</Label>
              <Input
                id="url"
                name="url"
                type="url"
                value={formData.url}
                onChange={(e) =>
                  setFormData((prev) => ({ ...prev, url: e.target.value }))
                }
                placeholder="https://example.com/webhook"
                required
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="secret">Secret (optional)</Label>
              <Input
                id="secret"
                name="secret"
                type="password"
                value={formData.secret}
                onChange={(e) =>
                  setFormData((prev) => ({ ...prev, secret: e.target.value }))
                }
                placeholder="For HMAC signature verification"
              />
              <p className="text-xs text-[var(--text-secondary)]">
                Used to sign webhook payloads for verification.
              </p>
            </div>

            <div className="space-y-2">
              <Label>Events</Label>
              <div className="grid grid-cols-2 gap-2 max-h-48 overflow-y-auto border rounded-md p-3">
                {WEBHOOK_EVENTS.map((event) => (
                  <label
                    key={event.id}
                    className="flex items-center gap-2 text-sm cursor-pointer"
                  >
                    <input
                      type="checkbox"
                      checked={formData.events.includes(event.id)}
                      onChange={() => toggleEvent(event.id)}
                      className="rounded border-gray-300"
                    />
                    {event.label}
                  </label>
                ))}
              </div>
            </div>

            <div className="flex items-center justify-between">
              <Label htmlFor="enabled">Enabled</Label>
              <Switch
                id="enabled"
                checked={formData.enabled}
                onCheckedChange={(checked: boolean) =>
                  setFormData((prev) => ({ ...prev, enabled: checked }))
                }
              />
            </div>

            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => {
                  setShowDialog(false);
                  setEditingWebhook(null);
                  resetForm();
                }}
              >
                Cancel
              </Button>
              <Button
                type="submit"
                disabled={isSubmitting || formData.events.length === 0}
              >
                {isSubmitting
                  ? "Saving..."
                  : editingWebhook
                  ? "Save changes"
                  : "Add webhook"}
              </Button>
            </DialogFooter>
          </Form>
        </DialogContent>
      </Dialog>
    </div>
  );
}
