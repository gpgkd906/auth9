import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { Form, Link, redirect, useActionData, useLoaderData, useNavigation } from "react-router";
import { ArrowLeftIcon } from "@radix-ui/react-icons";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Switch } from "~/components/ui/switch";
import { getAccessToken } from "~/services/session.server";
import { tenantApi, tenantSsoApi } from "~/services/api";

export const meta: MetaFunction<typeof loader> = ({ data }) => {
  return [{ title: `Enterprise SSO - ${data?.tenant.name || "Tenant"} - Auth9` }];
};

export async function loader({ params, request }: LoaderFunctionArgs) {
  const { tenantId } = params;
  if (!tenantId) throw new Error("Tenant ID is required");
  const accessToken = await getAccessToken(request);

  try {
    const [tenantRes, connectorsRes] = await Promise.all([
      tenantApi.get(tenantId, accessToken || undefined),
      tenantSsoApi.list(tenantId, accessToken || undefined),
    ]);

    return {
      tenant: tenantRes.data,
      connectors: connectorsRes.data,
    };
  } catch {
    throw redirect("/dashboard/tenants");
  }
}

export async function action({ request, params }: ActionFunctionArgs) {
  const { tenantId } = params;
  if (!tenantId) return { error: "Tenant ID is required" };
  const accessToken = await getAccessToken(request);
  const formData = await request.formData();
  const intent = formData.get("intent");

  try {
    if (intent === "create") {
      const providerType = String(formData.get("provider_type") || "saml") as "saml" | "oidc";
      const domains = String(formData.get("domains") || "")
        .split(",")
        .map((v) => v.trim())
        .filter(Boolean);

      const config: Record<string, string> = {};
      if (providerType === "saml") {
        config.entityId = String(formData.get("entity_id") || "").trim();
        config.singleSignOnServiceUrl = String(formData.get("sso_url") || "").trim();
        config.signingCertificate = String(formData.get("certificate") || "").trim();
      } else {
        config.clientId = String(formData.get("client_id") || "").trim();
        config.clientSecret = String(formData.get("client_secret") || "").trim();
        config.authorizationUrl = String(formData.get("authorization_url") || "").trim();
        config.tokenUrl = String(formData.get("token_url") || "").trim();
      }

      await tenantSsoApi.create(
        tenantId,
        {
          alias: String(formData.get("alias") || "").trim(),
          display_name: String(formData.get("display_name") || "").trim() || undefined,
          provider_type: providerType,
          enabled: formData.get("enabled") === "true",
          priority: Number(formData.get("priority") || "100"),
          domains,
          config,
        },
        accessToken || undefined
      );
      return { success: true, message: "Connector created" };
    }

    if (intent === "delete") {
      const connectorId = String(formData.get("connector_id") || "");
      await tenantSsoApi.delete(tenantId, connectorId, accessToken || undefined);
      return { success: true, message: "Connector deleted" };
    }

    if (intent === "toggle") {
      const connectorId = String(formData.get("connector_id") || "");
      const enabled = formData.get("enabled") === "true";
      await tenantSsoApi.update(
        tenantId,
        connectorId,
        {
          enabled,
        },
        accessToken || undefined
      );
      return { success: true, message: "Connector updated" };
    }

    if (intent === "test") {
      const connectorId = String(formData.get("connector_id") || "");
      const result = await tenantSsoApi.test(tenantId, connectorId, accessToken || undefined);
      return { success: result.data.ok, message: result.data.message };
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : "Operation failed";
    return { error: message };
  }

  return { error: "Invalid action" };
}

export default function TenantSsoPage() {
  const { tenant, connectors } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";

  return (
    <div className="space-y-6">
      <div className="flex items-center space-x-4">
        <Button variant="ghost" size="icon" asChild>
          <Link to={`/dashboard/tenants/${tenant.id}`}>
            <ArrowLeftIcon className="h-4 w-4" />
          </Link>
        </Button>
        <div>
          <h1 className="text-[24px] font-semibold text-[var(--text-primary)] tracking-tight">
            Enterprise SSO for {tenant.name}
          </h1>
          <p className="text-sm text-[var(--text-secondary)]">
            Configure tenant-scoped enterprise SAML/OIDC connectors.
          </p>
        </div>
      </div>

      {actionData?.error && (
        <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md">
          {actionData.error}
        </div>
      )}
      {actionData?.message && (
        <div className="text-sm text-[var(--accent-green)] bg-[var(--accent-green)]/10 p-3 rounded-md">
          {actionData.message}
        </div>
      )}

      <Card>
        <CardHeader>
          <CardTitle>Create Connector</CardTitle>
          <CardDescription>
            Use comma-separated domains. For SAML, provide Entity ID, SSO URL, and certificate.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Form method="post" className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <input type="hidden" name="intent" value="create" />
            <input type="hidden" name="enabled" value="true" />
            <div className="space-y-2">
              <Label htmlFor="alias">Alias</Label>
              <Input id="alias" name="alias" required placeholder="corp-saml" />
            </div>
            <div className="space-y-2">
              <Label htmlFor="display_name">Display Name</Label>
              <Input id="display_name" name="display_name" placeholder="Acme SSO" />
            </div>
            <div className="space-y-2">
              <Label htmlFor="provider_type">Provider Type</Label>
              <select
                id="provider_type"
                name="provider_type"
                className="w-full h-10 rounded-md border bg-transparent px-3 text-sm"
                defaultValue="saml"
              >
                <option value="saml">SAML</option>
                <option value="oidc">OIDC</option>
              </select>
            </div>
            <div className="space-y-2">
              <Label htmlFor="priority">Priority</Label>
              <Input id="priority" name="priority" defaultValue="100" />
            </div>
            <div className="space-y-2 md:col-span-2">
              <Label htmlFor="domains">Domains</Label>
              <Input id="domains" name="domains" required placeholder="acme.com, acme.org" />
            </div>

            <div className="space-y-2">
              <Label htmlFor="entity_id">SAML Entity ID</Label>
              <Input id="entity_id" name="entity_id" placeholder="https://idp.acme.com/entity" />
            </div>
            <div className="space-y-2">
              <Label htmlFor="sso_url">SAML SSO URL</Label>
              <Input id="sso_url" name="sso_url" placeholder="https://idp.acme.com/sso" />
            </div>
            <div className="space-y-2 md:col-span-2">
              <Label htmlFor="certificate">SAML Signing Certificate</Label>
              <Input id="certificate" name="certificate" placeholder="-----BEGIN CERTIFICATE-----..." />
            </div>

            <div className="space-y-2">
              <Label htmlFor="client_id">OIDC Client ID</Label>
              <Input id="client_id" name="client_id" />
            </div>
            <div className="space-y-2">
              <Label htmlFor="client_secret">OIDC Client Secret</Label>
              <Input id="client_secret" name="client_secret" type="password" />
            </div>
            <div className="space-y-2">
              <Label htmlFor="authorization_url">OIDC Authorization URL</Label>
              <Input id="authorization_url" name="authorization_url" />
            </div>
            <div className="space-y-2">
              <Label htmlFor="token_url">OIDC Token URL</Label>
              <Input id="token_url" name="token_url" />
            </div>

            <div className="md:col-span-2">
              <Button type="submit" disabled={isSubmitting}>
                {isSubmitting ? "Saving..." : "Create Connector"}
              </Button>
            </div>
          </Form>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Configured Connectors</CardTitle>
          <CardDescription>Manage availability and validate tenant routing connectors.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {connectors.length === 0 ? (
            <p className="text-sm text-[var(--text-secondary)]">No enterprise connectors configured.</p>
          ) : (
            connectors.map((connector) => (
              <div
                key={connector.id}
                className="border border-[var(--border-primary)] rounded-lg p-4 flex flex-col gap-3"
              >
                <div className="flex items-center justify-between">
                  <div>
                    <div className="font-medium text-[var(--text-primary)]">
                      {connector.display_name || connector.alias}
                    </div>
                    <div className="text-sm text-[var(--text-secondary)]">
                      {connector.provider_type.toUpperCase()} • {connector.domains.join(", ")}
                    </div>
                  </div>
                  <Form method="post" className="flex items-center gap-3">
                    <input type="hidden" name="intent" value="toggle" />
                    <input type="hidden" name="connector_id" value={connector.id} />
                    <input type="hidden" name="enabled" value={String(!connector.enabled)} />
                    <Switch
                      checked={connector.enabled}
                      onCheckedChange={() => null}
                      onClick={(e) => {
                        const form = (e.currentTarget as HTMLElement).closest("form") as HTMLFormElement;
                        form?.requestSubmit();
                      }}
                    />
                  </Form>
                </div>
                <div className="flex items-center gap-2">
                  <Form method="post">
                    <input type="hidden" name="intent" value="test" />
                    <input type="hidden" name="connector_id" value={connector.id} />
                    <Button type="submit" variant="outline" size="sm">
                      Test
                    </Button>
                  </Form>
                  <Form method="post">
                    <input type="hidden" name="intent" value="delete" />
                    <input type="hidden" name="connector_id" value={connector.id} />
                    <Button
                      type="submit"
                      variant="outline"
                      size="sm"
                      className="text-[var(--accent-red)] border-[var(--accent-red)]/40"
                    >
                      Delete
                    </Button>
                  </Form>
                </div>
              </div>
            ))
          )}
        </CardContent>
      </Card>
    </div>
  );
}
