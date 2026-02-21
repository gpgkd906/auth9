import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { Form, Link, useActionData, useLoaderData, useNavigation } from "react-router";
import { redirect } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Textarea } from "~/components/ui/textarea";
import { Switch } from "~/components/ui/switch";
import { ActionTrigger } from "@auth9/core";
import { getAuth9Client, withService, getTriggers } from "~/lib/auth9-client";
import { FormattedDate } from "~/components/ui/formatted-date";
import { getAccessToken } from "~/services/session.server";
import { ArrowLeftIcon } from "@radix-ui/react-icons";
import { useState } from "react";

export const meta: MetaFunction<typeof loader> = ({ data }) => {
  return [{ title: `Edit ${data?.action.name || "Action"} - Auth9` }];
};

export async function loader({ params, request }: LoaderFunctionArgs) {
  const { serviceId, actionId } = params;
  if (!serviceId || !actionId) throw new Error("Service ID and Action ID are required");
  const accessToken = await getAccessToken(request);

  const client = getAuth9Client(accessToken || undefined);
  const api = withService(client, serviceId);

  const [actionRes, triggersRes] = await Promise.all([
    api.actions.get(actionId),
    getTriggers(client),
  ]);

  return {
    serviceId,
    action: actionRes.data,
    triggers: triggersRes.data,
  };
}

export async function action({ params, request }: ActionFunctionArgs) {
  const { serviceId, actionId } = params;
  if (!serviceId || !actionId) return Response.json({ error: "IDs required" }, { status: 400 });
  const accessToken = await getAccessToken(request);

  const formData = await request.formData();
  const name = formData.get("name") as string;
  const description = formData.get("description") as string;
  const script = formData.get("script") as string;
  const enabled = formData.get("enabled") === "on";
  const strictMode = formData.get("strict_mode") === "on";
  const executionOrder = parseInt(formData.get("execution_order") as string) || 0;
  const timeoutMs = parseInt(formData.get("timeout_ms") as string) || 3000;

  try {
    const client = getAuth9Client(accessToken || undefined);
    const api = withService(client, serviceId);

    await api.actions.update(actionId, {
      name,
      description: description || undefined,
      script,
      enabled,
      strictMode,
      executionOrder,
      timeoutMs,
    });

    return redirect(`/dashboard/services/${serviceId}/actions/${actionId}`);
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown error";
    return { error: message };
  }
}

const TRIGGER_LABELS: Record<string, string> = {
  [ActionTrigger.PostLogin]: "Post Login",
  [ActionTrigger.PreUserRegistration]: "Pre Registration",
  [ActionTrigger.PostUserRegistration]: "Post Registration",
  [ActionTrigger.PostChangePassword]: "Post Password Change",
  [ActionTrigger.PostEmailVerification]: "Post Email Verification",
  [ActionTrigger.PreTokenRefresh]: "Pre Token Refresh",
};

export default function EditActionPage() {
  const { serviceId, action } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";

  const [script, setScript] = useState(action.script);

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="icon" asChild>
          <Link to={`/dashboard/services/${serviceId}/actions/${action.id}`}>
            <ArrowLeftIcon className="h-4 w-4" />
          </Link>
        </Button>
        <div>
          <h1 className="text-3xl font-bold">Edit Action</h1>
          <p className="text-muted-foreground mt-1">{action.name}</p>
        </div>
      </div>

      {actionData && typeof actionData === "object" && "error" in actionData && (
        <div className="p-4 bg-destructive/10 text-destructive rounded-md">
          {String(actionData.error)}
        </div>
      )}

      <Form method="post" className="space-y-6">
        {/* Basic Information */}
        <Card>
          <CardHeader>
            <CardTitle>Basic Information</CardTitle>
            <CardDescription>
              Update the basic settings for your action
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="name">Name *</Label>
              <Input
                id="name"
                name="name"
                defaultValue={action.name}
                required
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="description">Description</Label>
              <Input
                id="description"
                name="description"
                defaultValue={action.description || ""}
              />
            </div>

            <div className="space-y-2">
              <Label>Trigger</Label>
              <div className="p-2 bg-muted rounded-md">
                {TRIGGER_LABELS[action.triggerId]}
              </div>
              <p className="text-sm text-muted-foreground">
                Trigger cannot be changed after creation
              </p>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="execution_order">Execution Order</Label>
                <Input
                  id="execution_order"
                  name="execution_order"
                  type="number"
                  defaultValue={action.executionOrder}
                  min="0"
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="timeout_ms">Timeout (ms)</Label>
                <Input
                  id="timeout_ms"
                  name="timeout_ms"
                  type="number"
                  defaultValue={action.timeoutMs}
                  min="100"
                  max="30000"
                />
              </div>
            </div>

            <div className="flex items-center space-x-2">
              <Switch id="enabled" name="enabled" defaultChecked={action.enabled} />
              <Label htmlFor="enabled">Enabled</Label>
            </div>

            <div className="flex items-center space-x-2">
              <Switch id="strict_mode" name="strict_mode" defaultChecked={action.strictMode} />
              <Label htmlFor="strict_mode">Strict Mode</Label>
              <span className="text-sm text-muted-foreground">
                Block authentication flow on action failure
              </span>
            </div>
          </CardContent>
        </Card>

        {/* Script Editor */}
        <Card>
          <CardHeader>
            <CardTitle>Script</CardTitle>
            <CardDescription>
              Update the TypeScript code for this action
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="script">TypeScript Code *</Label>
              <Textarea
                id="script"
                name="script"
                value={script}
                onChange={(e) => setScript(e.target.value)}
                className="font-mono text-sm min-h-[400px]"
                required
              />
            </div>

            {/* Context Reference */}
            <div className="p-4 bg-muted rounded-md space-y-2">
              <div className="font-semibold text-sm">Context Structure:</div>
              <pre className="text-xs overflow-x-auto">
{`interface ActionContext {
  user: {
    id: string;
    email: string;
    display_name?: string;
    mfa_enabled: boolean;
  };
  tenant: {
    id: string;
    slug: string;
    name: string;
  };
  request: {
    ip?: string;
    user_agent?: string;
    timestamp: string;
  };
  claims?: Record<string, unknown>;
}`}
              </pre>
            </div>
          </CardContent>
        </Card>

        {/* Execution Stats */}
        <Card>
          <CardHeader>
            <CardTitle>Execution Statistics</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-3 gap-4 text-sm">
              <div>
                <div className="text-muted-foreground mb-1">Total Executions</div>
                <div className="text-2xl font-bold">{action.executionCount.toLocaleString()}</div>
              </div>
              <div>
                <div className="text-muted-foreground mb-1">Errors</div>
                <div className="text-2xl font-bold text-destructive">{action.errorCount.toLocaleString()}</div>
              </div>
              <div>
                <div className="text-muted-foreground mb-1">Last Executed</div>
                <div className="text-sm font-semibold">
                  {action.lastExecutedAt
                    ? <FormattedDate date={action.lastExecutedAt} />
                    : "Never"}
                </div>
              </div>
            </div>

            {action.lastError && (
              <div className="mt-4 p-3 bg-destructive/10 rounded-md">
                <div className="text-sm font-medium text-destructive mb-1">Last Error</div>
                <div className="text-sm text-muted-foreground">{action.lastError}</div>
              </div>
            )}
          </CardContent>
        </Card>

        {/* Actions */}
        <div className="flex gap-2">
          <Button type="submit" disabled={isSubmitting}>
            {isSubmitting ? "Saving..." : "Save Changes"}
          </Button>
          <Button type="button" variant="outline" asChild>
            <Link to={`/dashboard/services/${serviceId}/actions/${action.id}`}>Cancel</Link>
          </Button>
        </div>
      </Form>
    </div>
  );
}
