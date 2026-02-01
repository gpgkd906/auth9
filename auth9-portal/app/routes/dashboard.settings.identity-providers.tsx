import type { ActionFunctionArgs } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation } from "react-router";
import { useState, useEffect } from "react";
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
import {
  identityProviderApi,
  type IdentityProvider,
  type CreateIdentityProviderInput,
} from "~/services/api";
import { PlusIcon, Pencil2Icon, TrashIcon } from "@radix-ui/react-icons";

// Provider templates with required config fields
const PROVIDER_TEMPLATES = [
  {
    provider_id: "google",
    name: "Google",
    icon: "G",
    color: "bg-red-500",
    requiredFields: ["clientId", "clientSecret"],
  },
  {
    provider_id: "github",
    name: "GitHub",
    icon: "GH",
    color: "bg-gray-900",
    requiredFields: ["clientId", "clientSecret"],
  },
  {
    provider_id: "microsoft",
    name: "Microsoft",
    icon: "M",
    color: "bg-blue-600",
    requiredFields: ["clientId", "clientSecret"],
  },
  {
    provider_id: "oidc",
    name: "OpenID Connect",
    icon: "OIDC",
    color: "bg-purple-600",
    requiredFields: ["clientId", "clientSecret", "authorizationUrl", "tokenUrl"],
  },
  {
    provider_id: "saml",
    name: "SAML 2.0",
    icon: "SAML",
    color: "bg-orange-600",
    requiredFields: ["entityId", "ssoUrl", "certificate"],
  },
];

export async function loader() {
  try {
    const response = await identityProviderApi.list();
    return { providers: response.data };
  } catch {
    return { providers: [], error: "Failed to load identity providers" };
  }
}

export async function action({ request }: ActionFunctionArgs) {
  const formData = await request.formData();
  const intent = formData.get("intent");

  try {
    if (intent === "create") {
      const input: CreateIdentityProviderInput = {
        alias: formData.get("alias") as string,
        provider_id: formData.get("providerId") as string,
        display_name: formData.get("displayName") as string || undefined,
        enabled: formData.get("enabled") === "true",
        config: JSON.parse(formData.get("config") as string || "{}"),
      };
      await identityProviderApi.create(input);
      return { success: true, message: "Identity provider created" };
    }

    if (intent === "update") {
      const alias = formData.get("alias") as string;
      const input: Partial<CreateIdentityProviderInput> = {
        display_name: formData.get("displayName") as string || undefined,
        enabled: formData.get("enabled") === "true",
        config: JSON.parse(formData.get("config") as string || "{}"),
      };
      await identityProviderApi.update(alias, input);
      return { success: true, message: "Identity provider updated" };
    }

    if (intent === "delete") {
      const alias = formData.get("alias") as string;
      await identityProviderApi.delete(alias);
      return { success: true, message: "Identity provider deleted" };
    }

    if (intent === "toggle") {
      const alias = formData.get("alias") as string;
      const enabled = formData.get("enabled") === "true";
      await identityProviderApi.update(alias, { enabled });
      return { success: true };
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : "Operation failed";
    return { error: message };
  }

  return { error: "Invalid action" };
}

function getProviderTemplate(providerId: string) {
  return PROVIDER_TEMPLATES.find((t) => t.provider_id === providerId);
}

export default function IdentityProvidersPage() {
  const { providers, error: loadError } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();

  const [showDialog, setShowDialog] = useState(false);
  const [editingProvider, setEditingProvider] = useState<IdentityProvider | null>(null);
  const [selectedTemplate, setSelectedTemplate] = useState<string>("");
  const [formData, setFormData] = useState({
    alias: "",
    displayName: "",
    enabled: true,
    config: {} as Record<string, string>,
  });

  const isSubmitting = navigation.state === "submitting";

  // Close dialog on success
  useEffect(() => {
    if (actionData?.success && (showDialog || editingProvider)) {
      setShowDialog(false);
      setEditingProvider(null);
      resetForm();
    }
  }, [actionData, showDialog, editingProvider]);

  function resetForm() {
    setFormData({
      alias: "",
      displayName: "",
      enabled: true,
      config: {},
    });
    setSelectedTemplate("");
  }

  function openEditDialog(provider: IdentityProvider) {
    setEditingProvider(provider);
    setFormData({
      alias: provider.alias,
      displayName: provider.display_name || "",
      enabled: provider.enabled,
      config: provider.config,
    });
    setSelectedTemplate(provider.provider_id);
  }

  function openCreateDialog() {
    resetForm();
    setShowDialog(true);
  }

  const template = getProviderTemplate(selectedTemplate);

  return (
    <div className="space-y-6">
      {/* Header */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>Identity Providers</CardTitle>
              <CardDescription>
                Configure social logins and enterprise SSO for your users.
              </CardDescription>
            </div>
            <Button onClick={openCreateDialog}>
              <PlusIcon className="h-4 w-4 mr-2" />
              Add provider
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

      {/* Providers List */}
      <Card>
        <CardContent className="pt-6">
          {providers.length === 0 ? (
            <div className="text-center py-12">
              <h3 className="text-lg font-medium text-[var(--text-primary)] mb-2">
                No identity providers configured
              </h3>
              <p className="text-[var(--text-secondary)] mb-4">
                Add social logins like Google or GitHub to make sign-in easier for your users.
              </p>
              <Button onClick={openCreateDialog}>
                <PlusIcon className="h-4 w-4 mr-2" />
                Add your first provider
              </Button>
            </div>
          ) : (
            <div className="divide-y">
              {providers.map((provider: IdentityProvider) => {
                const tmpl = getProviderTemplate(provider.provider_id);
                return (
                  <div
                    key={provider.alias}
                    className="flex items-center gap-4 py-4 first:pt-0 last:pb-0"
                  >
                    <div
                      className={`w-10 h-10 rounded-lg flex items-center justify-center text-white text-xs font-bold ${
                        tmpl?.color || "bg-gray-600"
                      }`}
                    >
                      {tmpl?.icon || provider.provider_id.slice(0, 2).toUpperCase()}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="font-medium">
                        {provider.display_name || tmpl?.name || provider.alias}
                      </div>
                      <div className="text-sm text-[var(--text-secondary)]">
                        {provider.alias} • {provider.provider_id}
                      </div>
                    </div>
                    <div className="flex items-center gap-4">
                      <Form method="post">
                        <input type="hidden" name="intent" value="toggle" />
                        <input type="hidden" name="alias" value={provider.alias} />
                        <input
                          type="hidden"
                          name="enabled"
                          value={provider.enabled ? "false" : "true"}
                        />
                        <Switch
                          checked={provider.enabled}
                          onCheckedChange={() => {
                            const form = document.createElement("form");
                            form.method = "post";
                            form.innerHTML = `
                              <input name="intent" value="toggle" />
                              <input name="alias" value="${provider.alias}" />
                              <input name="enabled" value="${!provider.enabled}" />
                            `;
                            document.body.appendChild(form);
                            form.submit();
                          }}
                        />
                      </Form>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => openEditDialog(provider)}
                      >
                        <Pencil2Icon className="h-4 w-4" />
                      </Button>
                      <Form method="post">
                        <input type="hidden" name="intent" value="delete" />
                        <input type="hidden" name="alias" value={provider.alias} />
                        <Button
                          type="submit"
                          variant="ghost"
                          size="sm"
                          className="text-[var(--accent-red)] hover:text-[var(--accent-red)] hover:bg-[var(--accent-red)]/10"
                          disabled={isSubmitting}
                        >
                          <TrashIcon className="h-4 w-4" />
                        </Button>
                      </Form>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Create/Edit Dialog */}
      <Dialog
        open={showDialog || !!editingProvider}
        onOpenChange={(open) => {
          if (!open) {
            setShowDialog(false);
            setEditingProvider(null);
            resetForm();
          }
        }}
      >
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>
              {editingProvider ? "Edit Identity Provider" : "Add Identity Provider"}
            </DialogTitle>
            <DialogDescription>
              {editingProvider
                ? "Update the configuration for this identity provider."
                : "Choose a provider type and configure its settings."}
            </DialogDescription>
          </DialogHeader>

          <Form method="post" className="space-y-4">
            <input
              type="hidden"
              name="intent"
              value={editingProvider ? "update" : "create"}
            />
            <input type="hidden" name="alias" value={formData.alias} />
            <input type="hidden" name="providerId" value={selectedTemplate} />
            <input
              type="hidden"
              name="config"
              value={JSON.stringify(formData.config)}
            />
            <input
              type="hidden"
              name="enabled"
              value={formData.enabled ? "true" : "false"}
            />

            {/* Provider Type Selection (only for create) */}
            {!editingProvider && (
              <div className="space-y-2">
                <Label>Provider Type</Label>
                <div className="grid grid-cols-3 gap-2">
                  {PROVIDER_TEMPLATES.map((tmpl) => (
                    <button
                      key={tmpl.provider_id}
                      type="button"
                      onClick={() => {
                        setSelectedTemplate(tmpl.provider_id);
                        setFormData((prev) => ({
                          ...prev,
                          alias: tmpl.provider_id,
                        }));
                      }}
                      className={`p-3 rounded-lg border-2 text-center transition-colors ${
                        selectedTemplate === tmpl.provider_id
                          ? "border-blue-500 bg-blue-50"
                          : "border-[var(--glass-border-subtle)] hover:border-gray-300"
                      }`}
                    >
                      <div
                        className={`w-8 h-8 mx-auto rounded-md flex items-center justify-center text-white text-xs font-bold ${tmpl.color}`}
                      >
                        {tmpl.icon}
                      </div>
                      <div className="mt-1 text-sm font-medium">{tmpl.name}</div>
                    </button>
                  ))}
                </div>
              </div>
            )}

            {/* Common Fields */}
            {(selectedTemplate || editingProvider) && (
              <>
                <div className="space-y-2">
                  <Label htmlFor="alias">Alias (identifier)</Label>
                  <Input
                    id="alias"
                    value={formData.alias}
                    onChange={(e) =>
                      setFormData((prev) => ({ ...prev, alias: e.target.value }))
                    }
                    disabled={!!editingProvider}
                    placeholder="e.g., google-enterprise"
                  />
                </div>

                <div className="space-y-2">
                  <Label htmlFor="displayName">Display Name</Label>
                  <Input
                    id="displayName"
                    name="displayName"
                    value={formData.displayName}
                    onChange={(e) =>
                      setFormData((prev) => ({ ...prev, displayName: e.target.value }))
                    }
                    placeholder="e.g., Sign in with Google"
                  />
                </div>

                {/* Provider-specific fields */}
                {template?.requiredFields.includes("clientId") && (
                  <div className="space-y-2">
                    <Label htmlFor="clientId">Client ID</Label>
                    <Input
                      id="clientId"
                      value={formData.config.clientId || ""}
                      onChange={(e) =>
                        setFormData((prev) => ({
                          ...prev,
                          config: { ...prev.config, clientId: e.target.value },
                        }))
                      }
                      placeholder="OAuth Client ID"
                    />
                  </div>
                )}

                {template?.requiredFields.includes("clientSecret") && (
                  <div className="space-y-2">
                    <Label htmlFor="clientSecret">Client Secret</Label>
                    <Input
                      id="clientSecret"
                      type="password"
                      value={formData.config.clientSecret || ""}
                      onChange={(e) =>
                        setFormData((prev) => ({
                          ...prev,
                          config: { ...prev.config, clientSecret: e.target.value },
                        }))
                      }
                      placeholder="OAuth Client Secret"
                    />
                  </div>
                )}

                {template?.requiredFields.includes("authorizationUrl") && (
                  <div className="space-y-2">
                    <Label htmlFor="authorizationUrl">Authorization URL</Label>
                    <Input
                      id="authorizationUrl"
                      value={formData.config.authorizationUrl || ""}
                      onChange={(e) =>
                        setFormData((prev) => ({
                          ...prev,
                          config: { ...prev.config, authorizationUrl: e.target.value },
                        }))
                      }
                      placeholder="https://provider.com/oauth/authorize"
                    />
                  </div>
                )}

                {template?.requiredFields.includes("tokenUrl") && (
                  <div className="space-y-2">
                    <Label htmlFor="tokenUrl">Token URL</Label>
                    <Input
                      id="tokenUrl"
                      value={formData.config.tokenUrl || ""}
                      onChange={(e) =>
                        setFormData((prev) => ({
                          ...prev,
                          config: { ...prev.config, tokenUrl: e.target.value },
                        }))
                      }
                      placeholder="https://provider.com/oauth/token"
                    />
                  </div>
                )}

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
              </>
            )}

            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => {
                  setShowDialog(false);
                  setEditingProvider(null);
                  resetForm();
                }}
              >
                Cancel
              </Button>
              <Button
                type="submit"
                disabled={isSubmitting || (!selectedTemplate && !editingProvider)}
              >
                {isSubmitting
                  ? "Saving..."
                  : editingProvider
                  ? "Save changes"
                  : "Add provider"}
              </Button>
            </DialogFooter>
          </Form>
        </DialogContent>
      </Dialog>
    </div>
  );
}
