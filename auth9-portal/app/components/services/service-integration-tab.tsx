import { CopyIcon, EyeClosedIcon, EyeOpenIcon } from "@radix-ui/react-icons";
import { useState } from "react";
import { Button } from "~/components/ui/button";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Label } from "~/components/ui/label";
import { useI18n } from "~/i18n";
import type { ServiceIntegrationInfo } from "~/services/api";
import { CodeBlock, CopyValue, copyToClipboard } from "./copyable-value";

export function ServiceIntegrationTab({ integration }: { integration: ServiceIntegrationInfo }) {
  const { t } = useI18n();
  const [revealedSecrets, setRevealedSecrets] = useState<Set<string>>(new Set());

  const toggleReveal = (clientId: string) => {
    setRevealedSecrets((previous) => {
      const next = new Set(previous);
      if (next.has(clientId)) {
        next.delete(clientId);
      } else {
        next.add(clientId);
      }
      return next;
    });
  };

  const envBlock = integration.environment_variables.map((variable) => `${variable.key}=${variable.value}`).join("\n");
  const endpoints = [
    [t("services.integration.endpointLabels.authorize"), integration.endpoints.authorize],
    [t("services.integration.endpointLabels.token"), integration.endpoints.token],
    [t("services.integration.endpointLabels.callback"), integration.endpoints.callback],
    [t("services.integration.endpointLabels.logout"), integration.endpoints.logout],
    [t("services.integration.endpointLabels.userinfo"), integration.endpoints.userinfo],
    [t("services.integration.endpointLabels.openidConfiguration"), integration.endpoints.openid_configuration],
    [t("services.integration.endpointLabels.jwks"), integration.endpoints.jwks],
  ] as const;

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>{t("services.integration.clientsCredentials")}</CardTitle>
          <CardDescription>{t("services.integration.clientsCredentialsDescription")}</CardDescription>
        </CardHeader>
        <div className="space-y-4 p-6 pt-0">
          {integration.clients.length === 0 && (
            <p className="text-sm text-[var(--text-secondary)]">{t("services.integration.noClientsConfigured")}</p>
          )}
          {integration.clients.map((client) => (
            <div
              key={client.client_id}
              className="space-y-3 rounded-lg border border-[var(--glass-border-subtle)] bg-[var(--glass-bg)] p-4"
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium text-[var(--text-primary)]">
                    {client.name || client.client_id}
                  </span>
                  <span
                    className={`rounded-full px-2 py-0.5 text-[11px] font-medium ${
                      client.public_client
                        ? "bg-[var(--accent-blue)]/10 text-[var(--accent-blue)]"
                        : "bg-[var(--accent-purple)]/10 text-[var(--accent-purple)]"
                    }`}
                  >
                    {client.public_client ? t("services.integration.public") : t("services.integration.confidential")}
                  </span>
                </div>
              </div>
              <div className="space-y-2">
                <div>
                  <Label className="text-xs text-[var(--text-tertiary)]">{t("services.clientId")}</Label>
                  <CopyValue value={client.client_id} fieldId={t("services.clientId")} />
                </div>
                {client.public_client ? (
                  <div className="text-sm italic text-[var(--text-secondary)]">
                    {t("services.integration.publicNoSecret")}
                  </div>
                ) : (
                  <div>
                    <Label className="text-xs text-[var(--text-tertiary)]">{t("services.detail.clientSecret")}</Label>
                    {client.client_secret ? (
                      <div className="flex min-w-0 items-center gap-2">
                        <code className="min-w-0 flex-1 select-all break-all whitespace-normal font-mono text-sm text-[var(--text-primary)] [word-break:break-all]">
                          {revealedSecrets.has(client.client_id)
                            ? client.client_secret
                            : "••••••••••••••••••••••••"}
                        </code>
                        <Button
                          variant="ghost"
                          className="h-8 w-8 shrink-0 p-0 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)]"
                          onClick={() => toggleReveal(client.client_id)}
                          title={
                            revealedSecrets.has(client.client_id)
                              ? t("services.integration.hide")
                              : t("services.integration.reveal")
                          }
                        >
                          {revealedSecrets.has(client.client_id) ? (
                            <EyeClosedIcon className="h-3.5 w-3.5" />
                          ) : (
                            <EyeOpenIcon className="h-3.5 w-3.5" />
                          )}
                        </Button>
                        <Button
                          variant="ghost"
                          className="h-8 w-8 shrink-0 p-0 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)]"
                          onClick={() => copyToClipboard(client.client_secret!)}
                          title={t("services.integration.copySecret")}
                        >
                          <CopyIcon className="h-3.5 w-3.5" />
                        </Button>
                      </div>
                    ) : (
                      <span className="text-sm italic text-[var(--text-secondary)]">
                        {t("services.integration.clientSecretUnavailable")}
                      </span>
                    )}
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("services.integration.environmentVariables")}</CardTitle>
          <CardDescription>{t("services.integration.environmentVariablesDescription")}</CardDescription>
        </CardHeader>
        <div className="p-6 pt-0">
          <CodeBlock label=".env">{envBlock}</CodeBlock>
        </div>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("services.integration.endpoints")}</CardTitle>
          <CardDescription>{t("services.integration.endpointsDescription")}</CardDescription>
        </CardHeader>
        <div className="p-6 pt-0">
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-[var(--glass-border-subtle)]">
                  <th className="py-2 pr-4 text-left font-medium text-[var(--text-secondary)]">
                    {t("services.integration.endpoint")}
                  </th>
                  <th className="py-2 text-left font-medium text-[var(--text-secondary)]">
                    {t("services.integration.url")}
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-[var(--glass-border-subtle)]">
                {endpoints.map(([name, url]) => (
                  <tr key={name}>
                    <td className="whitespace-nowrap py-2 pr-4 font-medium text-[var(--text-primary)]">{name}</td>
                    <td className="py-2">
                      <CopyValue value={url} fieldId={name} />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("services.integration.sdkInitialization")}</CardTitle>
          <CardDescription>{t("services.integration.sdkInitializationDescription")}</CardDescription>
        </CardHeader>
        <div className="space-y-4 p-6 pt-0">
          <CodeBlock label="TypeScript - SDK Setup">{`import { Auth9 } from '@auth9/sdk';

const auth9 = new Auth9({
  domain: '${integration.endpoints.auth9_domain}',
  audience: '${integration.clients[0]?.client_id || "<your-client-id>"}',${integration.clients[0] && !integration.clients[0].public_client ? `
  clientSecret: process.env.AUTH9_CLIENT_SECRET,` : ""}
});`}</CodeBlock>

          <CodeBlock label="TypeScript - Express Middleware">{`import { auth9Middleware, requireRole } from '@auth9/express';

app.use(auth9Middleware({
  domain: process.env.AUTH9_DOMAIN!,
  audience: process.env.AUTH9_AUDIENCE!,
}));

// Protect a route with role check
app.get('/admin', requireRole('admin'), (req, res) => {
  res.json({ user: req.auth });
});`}</CodeBlock>

          <CodeBlock label="TypeScript - gRPC Token Exchange">{`import { Auth9GrpcClient } from '@auth9/grpc';

const grpc = new Auth9GrpcClient({
  address: '${integration.grpc.address}',
  apiKey: process.env.AUTH9_GRPC_API_KEY!,
});

// Exchange identity token first, then use tenant access token for downstream calls
const { accessToken } = await grpc.exchangeToken({
  identityToken: userIdToken,
  tenantId: 'tenant-uuid',
  audience: '${integration.clients[0]?.client_id || "<your-client-id>"}',
});`}</CodeBlock>
        </div>
      </Card>
    </div>
  );
}
