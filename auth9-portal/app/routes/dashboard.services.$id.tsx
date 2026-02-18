import type { MetaFunction, LoaderFunctionArgs, ActionFunctionArgs } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation, useSubmit } from "react-router";
import { PlusIcon, TrashIcon, ArrowLeftIcon, CopyIcon, UpdateIcon, EyeOpenIcon, EyeClosedIcon } from "@radix-ui/react-icons";
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
import { Tabs, TabsContent, TabsList, TabsTrigger } from "~/components/ui/tabs";
import { serviceApi } from "~/services/api";
import type { ServiceIntegrationInfo } from "~/services/api";
import { getAccessToken } from "~/services/session.server";

export const meta: MetaFunction = () => {
    return [{ title: "Service Details - Auth9" }];
};

export async function loader({ params, request }: LoaderFunctionArgs) {
    const { id } = params;
    if (!id) throw new Error("Service ID is required");
    const accessToken = await getAccessToken(request);

    // Fetch Service Details, Clients, and Integration in parallel
    const [serviceRes, clientsRes, integrationRes] = await Promise.all([
        serviceApi.get(id, accessToken || undefined),
        serviceApi.listClients(id, accessToken || undefined),
        serviceApi.getIntegration(id, accessToken || undefined).catch(() => null),
    ]);

    return {
        service: serviceRes.data,
        clients: clientsRes.data,
        integration: integrationRes?.data ?? null,
    };
}

export async function action({ request, params }: ActionFunctionArgs) {
    const { id } = params;
    if (!id) return Response.json({ error: "Service ID required" }, { status: 400 });
    const accessToken = await getAccessToken(request);

    const formData = await request.formData();
    const intent = formData.get("intent");

    try {
        if (intent === "update_service") {
            const name = formData.get("name") as string;
            const base_url = formData.get("base_url") as string;
            const redirect_uris = (formData.get("redirect_uris") as string)?.split(",").map(s => s.trim()).filter(Boolean);
            const logout_uris = (formData.get("logout_uris") as string)?.split(",").map(s => s.trim()).filter(Boolean);

            await serviceApi.update(id, {
                name,
                base_url: base_url || undefined,
                redirect_uris,
                logout_uris
            }, accessToken || undefined);
            return { success: true, intent };
        }

        if (intent === "create_client") {
            const name = formData.get("name") as string;
            const res = await serviceApi.createClient(id, { name: name || undefined }, accessToken || undefined);
            return { success: true, intent, secret: res.data.client_secret, clientId: res.data.client_id };
        }

        if (intent === "delete_client") {
            const clientId = formData.get("client_id") as string;
            await serviceApi.deleteClient(id, clientId, accessToken || undefined);
            return { success: true, intent };
        }

        if (intent === "regenerate_secret") {
            const clientId = formData.get("client_id") as string;
            const res = await serviceApi.regenerateClientSecret(id, clientId, accessToken || undefined);
            return { success: true, intent, secret: res.data.client_secret, regeneratedClientId: clientId };
        }

    } catch (error) {
        const message = error instanceof Error ? error.message : "Unknown error";
        return Response.json({ error: message }, { status: 400 });
    }

    return Response.json({ error: "Invalid intent" }, { status: 400 });
}

// Helper function to copy text to clipboard
function copyToClipboard(text: string): Promise<void> {
    return navigator.clipboard.writeText(text);
}

// Copiable code block component
function CodeBlock({ children, label }: { children: string; label?: string }) {
    const [copied, setCopied] = useState(false);

    const handleCopy = async () => {
        await copyToClipboard(children);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
    };

    return (
        <div className="relative group">
            {label && <div className="text-xs text-[var(--text-tertiary)] mb-1">{label}</div>}
            <div className="bg-[#0d1117] rounded-lg p-4 font-mono text-sm text-[#c9d1d9] overflow-x-auto whitespace-pre">
                {children}
            </div>
            <Button
                variant="ghost"
                size="icon"
                className="absolute top-2 right-2 h-7 w-7 opacity-0 group-hover:opacity-100 transition-opacity text-[#8b949e] hover:text-white hover:bg-[#30363d]"
                onClick={handleCopy}
            >
                {copied ? <span className="text-xs text-[var(--accent-green)]">&#10003;</span> : <CopyIcon className="h-3.5 w-3.5" />}
            </Button>
        </div>
    );
}

// Copiable inline value
function CopyValue({ value, fieldId }: { value: string; fieldId: string }) {
    const [copied, setCopied] = useState(false);

    return (
        <div className="flex items-center gap-2 min-w-0">
            <code className="flex-1 min-w-0 font-mono text-sm text-[var(--text-primary)] break-all select-all whitespace-normal">{value}</code>
            <Button
                variant="ghost"
                className="h-11 min-w-11 px-2 shrink-0 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] sm:h-8 sm:min-w-8 sm:px-2"
                onClick={async () => {
                    await copyToClipboard(value);
                    setCopied(true);
                    setTimeout(() => setCopied(false), 2000);
                }}
                title={`Copy ${fieldId}`}
            >
                {copied ? <span className="text-xs text-[var(--accent-green)]">&#10003;</span> : <CopyIcon className="h-3.5 w-3.5" />}
                <span className="ml-1 hidden sm:inline text-xs">Copy</span>
            </Button>
        </div>
    );
}

// Integration Tab content component
function IntegrationTab({ integration }: { integration: ServiceIntegrationInfo }) {
    const [revealedSecrets, setRevealedSecrets] = useState<Set<string>>(new Set());

    const toggleReveal = (clientId: string) => {
        setRevealedSecrets(prev => {
            const next = new Set(prev);
            if (next.has(clientId)) next.delete(clientId);
            else next.add(clientId);
            return next;
        });
    };

    // Build .env block
    const envBlock = integration.environment_variables
        .map(v => `${v.key}=${v.value}`)
        .join("\n");

    return (
        <div className="space-y-6">
            {/* Clients & Credentials */}
            <Card>
                <CardHeader>
                    <CardTitle>Clients &amp; Credentials</CardTitle>
                    <CardDescription>Client IDs and secrets for SDK integration</CardDescription>
                </CardHeader>
                <div className="p-6 pt-0 space-y-4">
                    {integration.clients.length === 0 && (
                        <p className="text-sm text-[var(--text-secondary)]">No clients configured. Create a client in the Configuration tab.</p>
                    )}
                    {integration.clients.map(client => (
                        <div key={client.client_id} className="p-4 rounded-lg border border-[var(--glass-border-subtle)] bg-[var(--glass-bg)] space-y-3">
                            <div className="flex items-center justify-between">
                                <div className="flex items-center gap-2">
                                    <span className="text-sm font-medium text-[var(--text-primary)]">{client.name || client.client_id}</span>
                                    <span className={`px-2 py-0.5 rounded-full text-[11px] font-medium ${client.public_client
                                        ? "bg-[var(--accent-blue)]/10 text-[var(--accent-blue)]"
                                        : "bg-[var(--accent-purple)]/10 text-[var(--accent-purple)]"
                                        }`}>
                                        {client.public_client ? "Public" : "Confidential"}
                                    </span>
                                </div>
                            </div>
                            <div className="space-y-2">
                                <div>
                                    <Label className="text-xs text-[var(--text-tertiary)]">Client ID</Label>
                                    <CopyValue value={client.client_id} fieldId="client_id" />
                                </div>
                                {client.public_client ? (
                                    <div className="text-sm text-[var(--text-secondary)] italic">
                                        Public client — no secret required
                                    </div>
                                ) : (
                                    <div>
                                        <Label className="text-xs text-[var(--text-tertiary)]">Client Secret</Label>
                                        {client.client_secret ? (
                                            <div className="flex items-center gap-2 min-w-0">
                                                <code className="flex-1 min-w-0 font-mono text-sm text-[var(--text-primary)] break-all select-all whitespace-normal">
                                                    {revealedSecrets.has(client.client_id) ? client.client_secret : "••••••••••••••••••••••••"}
                                                </code>
                                                <Button
                                                    variant="ghost"
                                                    className="h-11 min-w-11 px-2 shrink-0 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] sm:h-8 sm:min-w-8 sm:px-2"
                                                    onClick={() => toggleReveal(client.client_id)}
                                                    title={revealedSecrets.has(client.client_id) ? "Hide" : "Reveal"}
                                                >
                                                    {revealedSecrets.has(client.client_id) ? <EyeClosedIcon className="h-3.5 w-3.5" /> : <EyeOpenIcon className="h-3.5 w-3.5" />}
                                                    <span className="ml-1 hidden sm:inline text-xs">
                                                        {revealedSecrets.has(client.client_id) ? "Hide" : "Reveal"}
                                                    </span>
                                                </Button>
                                                <Button
                                                    variant="ghost"
                                                    className="h-11 min-w-11 px-2 shrink-0 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] sm:h-8 sm:min-w-8 sm:px-2"
                                                    onClick={() => copyToClipboard(client.client_secret!)}
                                                    title="Copy secret"
                                                >
                                                    <CopyIcon className="h-3.5 w-3.5" />
                                                    <span className="ml-1 hidden sm:inline text-xs">Copy</span>
                                                </Button>
                                            </div>
                                        ) : (
                                            <span className="text-sm text-[var(--text-secondary)] italic">Unable to retrieve — check Keycloak</span>
                                        )}
                                    </div>
                                )}
                            </div>
                        </div>
                    ))}
                </div>
            </Card>

            {/* Environment Variables */}
            <Card>
                <CardHeader>
                    <CardTitle>Environment Variables</CardTitle>
                    <CardDescription>Add these to your application&apos;s .env file</CardDescription>
                </CardHeader>
                <div className="p-6 pt-0">
                    <CodeBlock label=".env">{envBlock}</CodeBlock>
                </div>
            </Card>

            {/* OAuth/OIDC Endpoints */}
            <Card>
                <CardHeader>
                    <CardTitle>OAuth/OIDC Endpoints</CardTitle>
                    <CardDescription>Standard endpoints for OIDC integration</CardDescription>
                </CardHeader>
                <div className="p-6 pt-0">
                    <div className="overflow-x-auto">
                        <table className="w-full text-sm">
                            <thead>
                                <tr className="border-b border-[var(--glass-border-subtle)]">
                                    <th className="text-left py-2 pr-4 font-medium text-[var(--text-secondary)]">Endpoint</th>
                                    <th className="text-left py-2 font-medium text-[var(--text-secondary)]">URL</th>
                                </tr>
                            </thead>
                            <tbody className="divide-y divide-[var(--glass-border-subtle)]">
                                {[
                                    ["Authorize", integration.endpoints.authorize],
                                    ["Token", integration.endpoints.token],
                                    ["Callback", integration.endpoints.callback],
                                    ["Logout", integration.endpoints.logout],
                                    ["UserInfo", integration.endpoints.userinfo],
                                    ["OIDC Discovery", integration.endpoints.openid_configuration],
                                    ["JWKS", integration.endpoints.jwks],
                                ].map(([name, url]) => (
                                    <tr key={name}>
                                        <td className="py-2 pr-4 text-[var(--text-primary)] font-medium whitespace-nowrap">{name}</td>
                                        <td className="py-2"><CopyValue value={url} fieldId={name} /></td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>
                </div>
            </Card>

            {/* SDK Initialization */}
            <Card>
                <CardHeader>
                    <CardTitle>SDK Initialization</CardTitle>
                    <CardDescription>Quick start code for your application</CardDescription>
                </CardHeader>
                <div className="p-6 pt-0 space-y-4">
                    <CodeBlock label="TypeScript — SDK Setup">{`import { Auth9 } from '@auth9/sdk';

const auth9 = new Auth9({
  domain: '${integration.endpoints.auth9_domain}',
  audience: '${integration.clients[0]?.client_id || '<your-client-id>'}',${integration.clients[0] && !integration.clients[0].public_client ? `
  clientSecret: process.env.AUTH9_CLIENT_SECRET,` : ''}
});`}</CodeBlock>

                    <CodeBlock label="TypeScript — Express Middleware">{`import { auth9Middleware, requireRole } from '@auth9/express';

app.use(auth9Middleware({
  domain: process.env.AUTH9_DOMAIN!,
  audience: process.env.AUTH9_AUDIENCE!,
}));

// Protect a route with role check
app.get('/admin', requireRole('admin'), (req, res) => {
  res.json({ user: req.auth });
});`}</CodeBlock>

                    <CodeBlock label="TypeScript — gRPC Token Exchange">{`import { Auth9GrpcClient } from '@auth9/grpc';

const grpc = new Auth9GrpcClient({
  address: '${integration.grpc.address}',
  apiKey: process.env.AUTH9_GRPC_API_KEY!,
});

// Exchange identity token first, then use tenant access token for downstream calls
const { accessToken } = await grpc.exchangeToken({
  identityToken: userIdToken,
  tenantId: 'tenant-uuid',
  audience: '${integration.clients[0]?.client_id || '<your-client-id>'}',
});`}</CodeBlock>
                </div>
            </Card>
        </div>
    );
}

export default function ServiceDetailPage() {
    const { service, clients, integration } = useLoaderData<typeof loader>();
    const actionData = useActionData<typeof action>();
    const navigation = useNavigation();
    const submit = useSubmit();
    const confirm = useConfirm();

    const [isAddClientOpen, setIsAddClientOpen] = useState(false);
    const [secretDialog, setSecretDialog] = useState<{ clientId: string; secret: string; isNew: boolean } | null>(null);
    const [copiedField, setCopiedField] = useState<string | null>(null);

    const isSubmitting = navigation.state === "submitting";

    useEffect(() => {
        if (actionData) {
            if ("success" in actionData && actionData.success) {
                if (actionData.intent === "create_client" && "secret" in actionData && "clientId" in actionData && actionData.secret && actionData.clientId) {
                    setIsAddClientOpen(false);
                    setSecretDialog({ clientId: actionData.clientId as string, secret: actionData.secret as string, isNew: true });
                }
                if (actionData.intent === "regenerate_secret" && "secret" in actionData && "regeneratedClientId" in actionData) {
                    setSecretDialog({
                        clientId: actionData.regeneratedClientId as string,
                        secret: actionData.secret as string,
                        isNew: false
                    });
                }
            }
        }
    }, [actionData]);

    const handleCopy = async (text: string, fieldName: string) => {
        await copyToClipboard(text);
        setCopiedField(fieldName);
        setTimeout(() => setCopiedField(null), 2000);
    };

    return (
        <div className="space-y-6">
            <div className="flex items-center space-x-4">
                <Button variant="ghost" size="icon" asChild>
                    <a href="/dashboard/services"><ArrowLeftIcon className="h-4 w-4" /></a>
                </Button>
                <div>
                    <h1 className="text-[24px] font-semibold text-[var(--text-primary)] tracking-tight">{service.name}</h1>
                    <p className="text-sm text-[var(--text-secondary)]">Service Configuration and Integration</p>
                </div>
            </div>

            <Tabs defaultValue="configuration">
                <TabsList>
                    <TabsTrigger value="configuration">Configuration</TabsTrigger>
                    <TabsTrigger value="integration">Integration</TabsTrigger>
                </TabsList>

                <TabsContent value="configuration">
                    <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                        {/* Service Config */}
                        <div className="md:col-span-2">
                            <Card>
                                <CardHeader>
                                    <CardTitle>Configuration</CardTitle>
                                    <CardDescription>General settings for the service</CardDescription>
                                </CardHeader>
                                <div className="p-6">
                                    {actionData && "error" in actionData && (
                                        <div className="mb-4 p-3 rounded-lg bg-[var(--accent-red)]/10 border border-[var(--accent-red)]/30 text-[var(--accent-red)] text-sm">
                                            {String(actionData.error)}
                                        </div>
                                    )}
                                    <Form method="post" className="space-y-4">
                                        <input type="hidden" name="intent" value="update_service" />
                                        <div className="space-y-2">
                                            <Label htmlFor="name">Service Name</Label>
                                            <Input id="name" name="name" defaultValue={service.name} required />
                                        </div>
                                        <div className="space-y-2">
                                            <Label htmlFor="base_url">Base URL</Label>
                                            <Input id="base_url" name="base_url" defaultValue={service.base_url} placeholder="https://myapp.com" />
                                        </div>
                                        <div className="space-y-2">
                                            <Label htmlFor="redirect_uris">Redirect URIs (comma separated)</Label>
                                            <Input id="redirect_uris" name="redirect_uris" defaultValue={service.redirect_uris?.join(", ")} />
                                        </div>
                                        <div className="space-y-2">
                                            <Label htmlFor="logout_uris">Logout URIs (comma separated)</Label>
                                            <Input id="logout_uris" name="logout_uris" defaultValue={service.logout_uris?.join(", ")} />
                                        </div>
                                        <div className="flex justify-end pt-4">
                                            <Button type="submit" disabled={isSubmitting}>
                                                {isSubmitting ? "Saving..." : "Save Changes"}
                                            </Button>
                                        </div>
                                    </Form>
                                </div>
                            </Card>
                        </div>

                        {/* Clients List */}
                        <div>
                            <Card className="h-full">
                                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                                    <div className="space-y-1">
                                        <CardTitle>Clients</CardTitle>
                                        <CardDescription>Credentials (API Keys)</CardDescription>
                                    </div>
                                    <Dialog open={isAddClientOpen} onOpenChange={setIsAddClientOpen}>
                                        <DialogTrigger asChild>
                                            <Button size="sm" variant="outline"><PlusIcon className="h-4 w-4" /></Button>
                                        </DialogTrigger>
                                        <DialogContent>
                                            <DialogHeader>
                                                <DialogTitle>Create New Client</DialogTitle>
                                                <DialogDescription>Create a new set of credentials for this service.</DialogDescription>
                                            </DialogHeader>
                                            <Form method="post" className="space-y-4">
                                                <input type="hidden" name="intent" value="create_client" />
                                                <div className="space-y-2">
                                                    <Label htmlFor="client-name">Description (Optional)</Label>
                                                    <Input id="client-name" name="name" placeholder="e.g. Production Web App" />
                                                </div>
                                                <DialogFooter>
                                                    <Button type="button" variant="outline" onClick={() => setIsAddClientOpen(false)}>Cancel</Button>
                                                    <Button type="submit" disabled={isSubmitting}>Create</Button>
                                                </DialogFooter>
                                            </Form>
                                        </DialogContent>
                                    </Dialog>
                                </CardHeader>
                                <div className="p-0">
                                    <ul className="divide-y divide-[var(--glass-border-subtle)]">
                                        {clients.map(client => (
                                            <li key={client.id} className="p-4 hover:bg-[var(--sidebar-item-hover)]">
                                                <div className="flex items-start justify-between mb-2">
                                                    <div className="flex-1 min-w-0">
                                                        <div className="flex items-center gap-2">
                                                            <code className="font-mono text-sm font-medium text-[var(--text-primary)] truncate">
                                                                {client.client_id}
                                                            </code>
                                                            <Button
                                                                variant="ghost"
                                                                size="icon"
                                                                className="h-6 w-6 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)]"
                                                                onClick={() => handleCopy(client.client_id, `client-${client.id}`)}
                                                                title="Copy Client ID"
                                                            >
                                                                {copiedField === `client-${client.id}` ? (
                                                                    <span className="text-xs text-[var(--accent-green)]">&#10003;</span>
                                                                ) : (
                                                                    <CopyIcon className="h-3 w-3" />
                                                                )}
                                                            </Button>
                                                        </div>
                                                        <div className="text-xs text-[var(--text-secondary)] mt-1">
                                                            {client.name || "No description"}
                                                        </div>
                                                        <div className="text-xs text-[var(--text-tertiary)] mt-0.5">
                                                            Created: {new Date(client.created_at).toLocaleDateString()}
                                                        </div>
                                                    </div>
                                                </div>
                                                <div className="flex items-center gap-2 mt-2">
                                                    <Button
                                                        variant="outline"
                                                        size="sm"
                                                        className="h-7 text-xs"
                                                        onClick={async () => {
                                                            const ok = await confirm({
                                                                title: "Regenerate Secret",
                                                                description: "Regenerate secret? The old secret will stop working immediately.",
                                                                confirmLabel: "Regenerate",
                                                            });
                                                            if (ok) {
                                                                submit({ intent: "regenerate_secret", client_id: client.client_id }, { method: "post" });
                                                            }
                                                        }}
                                                    >
                                                        <UpdateIcon className="h-3 w-3 mr-1" />
                                                        Regenerate
                                                    </Button>
                                                    <Button
                                                        variant="ghost"
                                                        size="sm"
                                                        className="h-7 text-xs text-[var(--accent-red)] hover:text-[var(--accent-red)] hover:bg-[var(--accent-red)]/10"
                                                        onClick={async () => {
                                                            const ok = await confirm({
                                                                title: "Delete Client",
                                                                description: "Delete this client? This action cannot be undone.",
                                                                variant: "destructive",
                                                            });
                                                            if (ok) {
                                                                submit({ intent: "delete_client", client_id: client.client_id }, { method: "post" });
                                                            }
                                                        }}
                                                    >
                                                        <TrashIcon className="h-3 w-3 mr-1" />
                                                        Delete
                                                    </Button>
                                                </div>
                                            </li>
                                        ))}
                                        {clients.length === 0 && (
                                            <li className="p-4 text-center text-sm text-[var(--text-secondary)]">No clients found.</li>
                                        )}
                                    </ul>
                                </div>
                            </Card>
                        </div>
                    </div>
                </TabsContent>

                <TabsContent value="integration">
                    {integration ? (
                        <IntegrationTab integration={integration} />
                    ) : (
                        <Card>
                            <div className="p-6 text-center text-[var(--text-secondary)]">
                                <p>Integration info is not available. Ensure Auth9 Core is running and Keycloak is reachable.</p>
                            </div>
                        </Card>
                    )}
                </TabsContent>
            </Tabs>

            {/* Secret Display Dialog */}
            <Dialog open={!!secretDialog} onOpenChange={(open) => !open && setSecretDialog(null)}>
                <DialogContent>
                    <DialogHeader>
                        <DialogTitle>
                            {secretDialog?.isNew ? "Client Created Successfully" : "Secret Regenerated"}
                        </DialogTitle>
                        <DialogDescription>
                            Copy the Client Secret now. It will not be shown again.
                        </DialogDescription>
                    </DialogHeader>
                    <div className="space-y-4">
                        <div>
                            <Label className="text-xs text-[var(--text-secondary)]">Client ID</Label>
                            <div className="flex items-center gap-2 mt-1">
                                <div className="flex-1 p-2 bg-[var(--sidebar-item-hover)] rounded border font-mono text-sm break-all select-all">
                                    {secretDialog?.clientId}
                                </div>
                                <Button
                                    variant="outline"
                                    size="icon"
                                    className="h-8 w-8 shrink-0"
                                    onClick={() => secretDialog && handleCopy(secretDialog.clientId, 'dialog-id')}
                                >
                                    {copiedField === 'dialog-id' ? (
                                        <span className="text-xs text-[var(--accent-green)]">&#10003;</span>
                                    ) : (
                                        <CopyIcon className="h-4 w-4" />
                                    )}
                                </Button>
                            </div>
                        </div>
                        <div>
                            <Label className="text-xs text-[var(--text-secondary)]">Client Secret</Label>
                            <div className="flex items-center gap-2 mt-1">
                                <div className="flex-1 p-3 bg-[var(--accent-green)]/10 rounded border border-[var(--accent-green)]/20 font-mono text-center break-all select-all font-bold text-[var(--accent-green)]">
                                    {secretDialog?.secret}
                                </div>
                                <Button
                                    variant="outline"
                                    size="icon"
                                    className="h-8 w-8 shrink-0"
                                    onClick={() => secretDialog && handleCopy(secretDialog.secret, 'dialog-secret')}
                                >
                                    {copiedField === 'dialog-secret' ? (
                                        <span className="text-xs text-[var(--accent-green)]">&#10003;</span>
                                    ) : (
                                        <CopyIcon className="h-4 w-4" />
                                    )}
                                </Button>
                            </div>
                        </div>
                    </div>
                    <DialogFooter>
                        <Button onClick={() => setSecretDialog(null)}>Close</Button>
                    </DialogFooter>
                </DialogContent>
            </Dialog>
        </div>
    );
}
