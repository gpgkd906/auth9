import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { Form, Link, useActionData, useLoaderData, useNavigation } from "react-router";
import { redirect } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Textarea } from "~/components/ui/textarea";
import { Switch } from "~/components/ui/switch";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "~/components/ui/select";
import { ActionTrigger } from "@auth9/core";
import { getAuth9Client, withTenant, getTriggers } from "~/lib/auth9-client";
import { getAccessToken } from "~/services/session.server";
import { ArrowLeftIcon } from "@radix-ui/react-icons";
import { useState } from "react";

export const meta: MetaFunction = () => {
  return [{ title: "New Action - Auth9" }];
};

export async function loader({ params, request }: LoaderFunctionArgs) {
  const { tenantId } = params;
  if (!tenantId) throw new Error("Tenant ID is required");
  const accessToken = await getAccessToken(request);

  const client = getAuth9Client(accessToken || undefined);
  const triggersRes = await getTriggers(client);

  return {
    tenantId,
    triggers: triggersRes.data,
  };
}

export async function action({ params, request }: ActionFunctionArgs) {
  const { tenantId } = params;
  if (!tenantId) return Response.json({ error: "Tenant ID required" }, { status: 400 });
  const accessToken = await getAccessToken(request);

  const formData = await request.formData();
  const name = formData.get("name") as string;
  const description = formData.get("description") as string;
  const triggerId = formData.get("trigger_id") as string;
  const script = formData.get("script") as string;
  const enabled = formData.get("enabled") === "on";
  const strictMode = formData.get("strict_mode") === "on";
  const executionOrder = parseInt(formData.get("execution_order") as string) || 0;
  const timeoutMs = parseInt(formData.get("timeout_ms") as string) || 3000;

  try {
    const client = getAuth9Client(accessToken || undefined);
    const api = withTenant(client, tenantId);

    const result = await api.actions.create({
      name,
      description: description || undefined,
      triggerId,
      script,
      enabled,
      strictMode,
      executionOrder,
      timeoutMs,
    });

    return redirect(`/dashboard/tenants/${tenantId}/actions/${result.data.id}`);
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

const SCRIPT_TEMPLATES: Record<string, { name: string; description: string; script: string }> = {
  "add-claims": {
    name: "Add Custom Claims",
    description: "Add custom claims to the user's token",
    script: `// Add custom claims to token
context.claims = context.claims || {};
context.claims.department = "engineering";
context.claims.tier = "premium";

// Return modified context
context;`,
  },
  "block-domain": {
    name: "Block Email Domain",
    description: "Block users from specific email domains",
    script: `// Block specific email domains
const blockedDomains = ["@competitor.com", "@spam.com"];
if (blockedDomains.some(domain => context.user.email.endsWith(domain))) {
  throw new Error("Email domain not allowed");
}

context;`,
  },
  "require-mfa": {
    name: "Conditional MFA",
    description: "Require MFA for specific IP ranges",
    script: `// Require MFA for specific IP ranges
if (context.request.ip?.startsWith("203.")) {
  context.claims = context.claims || {};
  context.claims.require_mfa = true;
}

context;`,
  },
  "service-access": {
    name: "Service Access Control",
    description: "Grant access to services based on roles",
    script: `// Check user roles and grant service access
const allowedRoles = ["admin", "developer"];
const userRoles = (context.claims?.roles as string[]) || [];

const hasAccess = allowedRoles.some(role => userRoles.includes(role));

if (!hasAccess) {
  throw new Error("Insufficient permissions");
}

// Grant service access
context.claims = context.claims || {};
context.claims.service_access = context.claims.service_access || [];
(context.claims.service_access as string[]).push("my-service");

context;`,
  },
};

export default function NewActionPage() {
  const { tenantId, triggers } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";

  const [selectedTemplate, setSelectedTemplate] = useState<string>("");
  const [script, setScript] = useState<string>("// Your TypeScript code here\ncontext;");

  const handleTemplateSelect = (templateId: string) => {
    setSelectedTemplate(templateId);
    if (templateId && SCRIPT_TEMPLATES[templateId]) {
      setScript(SCRIPT_TEMPLATES[templateId].script);
    }
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="icon" asChild>
          <Link to={`/dashboard/tenants/${tenantId}/actions`}>
            <ArrowLeftIcon className="h-4 w-4" />
          </Link>
        </Button>
        <div>
          <h1 className="text-3xl font-bold">New Action</h1>
          <p className="text-muted-foreground mt-1">
            Create a new authentication flow action
          </p>
        </div>
      </div>

      {actionData?.error && (
        <div className="p-4 bg-destructive/10 text-destructive rounded-md">
          {actionData.error}
        </div>
      )}

      <Form method="post" className="space-y-6">
        {/* Basic Information */}
        <Card>
          <CardHeader>
            <CardTitle>Basic Information</CardTitle>
            <CardDescription>
              Configure the basic settings for your action
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="name">Name *</Label>
              <Input
                id="name"
                name="name"
                placeholder="my-action"
                required
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="description">Description</Label>
              <Input
                id="description"
                name="description"
                placeholder="What does this action do?"
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="trigger_id">Trigger *</Label>
              <Select name="trigger_id" required>
                <SelectTrigger>
                  <SelectValue placeholder="Select a trigger" />
                </SelectTrigger>
                <SelectContent>
                  {triggers.map((trigger) => (
                    <SelectItem key={trigger} value={trigger}>
                      {TRIGGER_LABELS[trigger]}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="execution_order">Execution Order</Label>
                <Input
                  id="execution_order"
                  name="execution_order"
                  type="number"
                  defaultValue="0"
                  min="0"
                />
                <p className="text-sm text-muted-foreground">
                  Lower numbers execute first
                </p>
              </div>

              <div className="space-y-2">
                <Label htmlFor="timeout_ms">Timeout (ms)</Label>
                <Input
                  id="timeout_ms"
                  name="timeout_ms"
                  type="number"
                  defaultValue="3000"
                  min="100"
                  max="30000"
                />
                <p className="text-sm text-muted-foreground">
                  Maximum execution time
                </p>
              </div>
            </div>

            <div className="flex items-center space-x-2">
              <Switch id="enabled" name="enabled" defaultChecked />
              <Label htmlFor="enabled">Enabled</Label>
            </div>

            <div className="flex items-center space-x-2">
              <Switch id="strict_mode" name="strict_mode" />
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
              Write TypeScript code to modify the authentication context
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {/* Template Selector */}
            <div className="space-y-2">
              <Label>Script Templates</Label>
              <Select value={selectedTemplate} onValueChange={handleTemplateSelect}>
                <SelectTrigger>
                  <SelectValue placeholder="Choose a template (optional)" />
                </SelectTrigger>
                <SelectContent>
                  {Object.entries(SCRIPT_TEMPLATES).map(([id, template]) => (
                    <SelectItem key={id} value={id}>
                      {template.name} - {template.description}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {/* Code Editor (Basic) */}
            <div className="space-y-2">
              <Label htmlFor="script">TypeScript Code *</Label>
              <Textarea
                id="script"
                name="script"
                value={script}
                onChange={(e) => setScript(e.target.value)}
                className="font-mono text-sm min-h-[400px]"
                placeholder="// Your TypeScript code here&#10;context;"
                required
              />
              <p className="text-sm text-muted-foreground">
                The <code>context</code> object is available globally. Modify it and return it.
              </p>
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

        {/* Actions */}
        <div className="flex gap-2">
          <Button type="submit" disabled={isSubmitting}>
            {isSubmitting ? "Creating..." : "Create Action"}
          </Button>
          <Button type="button" variant="outline" asChild>
            <Link to={`/dashboard/tenants/${tenantId}/actions`}>Cancel</Link>
          </Button>
        </div>
      </Form>
    </div>
  );
}
