import type { MetaFunction, LoaderFunctionArgs, ActionFunctionArgs } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation, useSubmit } from "react-router";
import { PlusIcon, TrashIcon, ArrowLeftIcon, CopyIcon, UpdateIcon } from "@radix-ui/react-icons";
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
import { serviceApi } from "~/services/api";
import { getAccessToken } from "~/services/session.server";

export const meta: MetaFunction = () => {
    return [{ title: "Service Details - Auth9" }];
};

export async function loader({ params, request }: LoaderFunctionArgs) {
    const { id } = params;
    if (!id) throw new Error("Service ID is required");
    const accessToken = await getAccessToken(request);

    // Fetch Service Details and Clients in parallel
    const [serviceRes, clientsRes] = await Promise.all([
        serviceApi.get(id, accessToken || undefined),
        serviceApi.listClients(id, accessToken || undefined)
    ]);

    return {
        service: serviceRes.data,
        clients: clientsRes.data
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
            // ClientWithSecret is flattened - client fields are at root level
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

export default function ServiceDetailPage() {
    const { service, clients } = useLoaderData<typeof loader>();
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
                    <p className="text-sm text-[var(--text-secondary)]">Service Configuration and Clients</p>
                </div>
            </div>

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
                                                            <span className="text-xs text-[var(--accent-green)]">✓</span>
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
                                        <span className="text-xs text-[var(--accent-green)]">✓</span>
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
                                        <span className="text-xs text-[var(--accent-green)]">✓</span>
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
