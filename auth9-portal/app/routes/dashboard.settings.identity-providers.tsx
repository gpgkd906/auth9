import type { ActionFunctionArgs, LoaderFunctionArgs } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation, useSubmit } from "react-router";
import { useState, useEffect, useRef, useMemo } from "react";
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
import { getAccessToken } from "~/services/session.server";
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
    requiredFields: ["entityId", "singleSignOnServiceUrl", "signingCertificate"],
  },
];

export async function loader({ request }: LoaderFunctionArgs) {
  const accessToken = await getAccessToken(request);
  try {
    const response = await identityProviderApi.list(accessToken || undefined);
    return { providers: response.data };
  } catch {
    return { providers: [], error: "Failed to load identity providers" };
  }
}

export async function action({ request }: ActionFunctionArgs) {
  const accessToken = await getAccessToken(request);
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
      await identityProviderApi.create(input, accessToken || undefined);
      return { success: true, message: "Identity provider created" };
    }

    if (intent === "update") {
      const alias = formData.get("alias") as string;
      const input: Partial<CreateIdentityProviderInput> = {
        display_name: formData.get("displayName") as string || undefined,
        enabled: formData.get("enabled") === "true",
        config: JSON.parse(formData.get("config") as string || "{}"),
      };
      await identityProviderApi.update(alias, input, accessToken || undefined);
      return { success: true, message: "Identity provider updated" };
    }

    if (intent === "delete") {
      const alias = formData.get("alias") as string;
      await identityProviderApi.delete(alias, accessToken || undefined);
      return { success: true, message: "Identity provider deleted" };
    }

    if (intent === "toggle") {
      const alias = formData.get("alias") as string;
      const enabled = formData.get("enabled") === "true";
      await identityProviderApi.update(alias, { enabled }, accessToken || undefined);
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

  const submit = useSubmit();
  const isSubmitting = navigation.state === "submitting";
  // Track when form submission starts to avoid closing dialog on stale actionData
  const wasSubmitting = useRef(false);

  // Close dialog on success - only after actual form submission
  useEffect(() => {
    // Track submission state
    if (isSubmitting) {
      wasSubmitting.current = true;
    }
    // Only close dialog if we were submitting and got success
    if (wasSubmitting.current && !isSubmitting && actionData?.success && (showDialog || editingProvider)) {
      setShowDialog(false);
      setEditingProvider(null);
      resetForm();
      wasSubmitting.current = false;
    }
  }, [actionData, isSubmitting, showDialog, editingProvider]);

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
    wasSubmitting.current = false; // Reset to avoid stale actionData closing dialog
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
    wasSubmitting.current = false; // Reset to avoid stale actionData closing dialog
    resetForm();
    setShowDialog(true);
  }

  const template = getProviderTemplate(selectedTemplate);

  // Check for duplicate alias (only during creation, not editing)
  const isDuplicateAlias = useMemo(() => {
    if (editingProvider) return false;
    const alias = formData.alias.trim();
    if (!alias) return false;
    return providers.some((p) => p.alias.toLowerCase() === alias.toLowerCase());
  }, [formData.alias, providers, editingProvider]);

  // Validate required fields are filled
  const hasRequiredFields = (() => {
    if (!template) return false;
    return template.requiredFields.every(
      (field) => formData.config[field] && formData.config[field].trim() !== ""
    );
  })();

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
                      <Switch
                        checked={provider.enabled}
                        onCheckedChange={() => {
                          submit(
                            {
                              intent: "toggle",
                              alias: provider.alias,
                              enabled: String(!provider.enabled),
                            },
                            { method: "post" }
                          );
                        }}
                      />
                      <Button
                        variant="ghost"
                        size="sm"
                        aria-label="Edit provider"
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
                          aria-label="Delete provider"
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

          <form
            className="space-y-4"
            onSubmit={(e) => {
              e.preventDefault();
              submit(
                {
                  intent: editingProvider ? "update" : "create",
                  alias: formData.alias,
                  providerId: selectedTemplate,
                  config: JSON.stringify(formData.config),
                  enabled: formData.enabled ? "true" : "false",
                  ...(formData.displayName ? { displayName: formData.displayName } : {}),
                },
                { method: "post" }
              );
            }}
          >

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
                  {isDuplicateAlias && (
                    <p className="text-sm text-[var(--accent-red)]">
                      Identity provider with this alias already exists
                    </p>
                  )}
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
                    <Label htmlFor="clientId">Client ID <span className="text-[var(--accent-red)]">*</span></Label>
                    <Input
                      id="clientId"
                      required
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
                    <Label htmlFor="clientSecret">Client Secret <span className="text-[var(--accent-red)]">*</span></Label>
                    <Input
                      id="clientSecret"
                      type="password"
                      required
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
                    <Label htmlFor="authorizationUrl">Authorization URL <span className="text-[var(--accent-red)]">*</span></Label>
                    <Input
                      id="authorizationUrl"
                      required
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
                    <Label htmlFor="tokenUrl">Token URL <span className="text-[var(--accent-red)]">*</span></Label>
                    <Input
                      id="tokenUrl"
                      required
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

                {template?.requiredFields.includes("entityId") && (
                  <div className="space-y-2">
                    <Label htmlFor="entityId">Entity ID <span className="text-[var(--accent-red)]">*</span></Label>
                    <Input
                      id="entityId"
                      required
                      value={formData.config.entityId || ""}
                      onChange={(e) =>
                        setFormData((prev) => ({
                          ...prev,
                          config: { ...prev.config, entityId: e.target.value },
                        }))
                      }
                      placeholder="https://idp.example.com/entity"
                    />
                  </div>
                )}

                {template?.requiredFields.includes("singleSignOnServiceUrl") && (
                  <div className="space-y-2">
                    <Label htmlFor="singleSignOnServiceUrl">Single Sign-On Service URL <span className="text-[var(--accent-red)]">*</span></Label>
                    <Input
                      id="singleSignOnServiceUrl"
                      required
                      value={formData.config.singleSignOnServiceUrl || ""}
                      onChange={(e) =>
                        setFormData((prev) => ({
                          ...prev,
                          config: { ...prev.config, singleSignOnServiceUrl: e.target.value },
                        }))
                      }
                      placeholder="https://idp.example.com/sso"
                    />
                  </div>
                )}

                {template?.requiredFields.includes("signingCertificate") && (
                  <div className="space-y-2">
                    <Label htmlFor="signingCertificate">Signing Certificate <span className="text-[var(--accent-red)]">*</span></Label>
                    <Input
                      id="signingCertificate"
                      required
                      value={formData.config.signingCertificate || ""}
                      onChange={(e) =>
                        setFormData((prev) => ({
                          ...prev,
                          config: { ...prev.config, signingCertificate: e.target.value },
                        }))
                      }
                      placeholder="-----BEGIN CERTIFICATE-----..."
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
                disabled={isSubmitting || (!selectedTemplate && !editingProvider) || !formData.alias.trim() || (!!template && !hasRequiredFields) || isDuplicateAlias}
              >
                {isSubmitting
                  ? "Saving..."
                  : editingProvider
                  ? "Save changes"
                  : "Add provider"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>
  );
}
