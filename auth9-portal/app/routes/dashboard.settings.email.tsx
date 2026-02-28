import type { MetaFunction, ActionFunctionArgs, LoaderFunctionArgs } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation, useSubmit } from "react-router";
import { useState, useEffect } from "react";
import { CheckCircledIcon, CrossCircledIcon, ExclamationTriangleIcon } from "@radix-ui/react-icons";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Checkbox } from "~/components/ui/checkbox";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "~/components/ui/select";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "~/components/ui/dialog";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "~/components/ui/alert-dialog";
import { systemApi, type EmailProviderConfig } from "~/services/api";
import { getAccessToken } from "~/services/session.server";

// Helper to get provider display info
function getProviderInfo(config: EmailProviderConfig): { name: string; details: string } | null {
  switch (config.type) {
    case "smtp":
      return { name: "SMTP", details: `${config.host}:${config.port}` };
    case "ses":
      return { name: "AWS SES", details: `Region: ${config.region}` };
    case "oracle":
      return { name: "Oracle Email Delivery", details: config.smtp_endpoint };
    default:
      return null;
  }
}

const PROVIDER_LABELS: Record<string, string> = {
  none: "None (Email disabled)",
  smtp: "SMTP",
  ses: "AWS SES",
  oracle: "Oracle Email Delivery",
};

export const meta: MetaFunction = () => {
  return [{ title: "Email Settings - Auth9" }];
};

export async function loader({ request }: LoaderFunctionArgs) {
  const accessToken = await getAccessToken(request);
  try {
    const result = await systemApi.getEmailSettings(accessToken || undefined);
    // The API returns { data: { category, setting_key, value, ... } }
    // The actual config is in result.data.value
    const config = result.data.value as EmailProviderConfig;
    return { config, error: null };
  } catch {
    // If no config exists yet, return none
    return { config: { type: "none" } as EmailProviderConfig, error: null };
  }
}

export async function action({ request }: ActionFunctionArgs) {
  const accessToken = await getAccessToken(request);
  const formData = await request.formData();
  const intent = formData.get("intent");

  try {
    if (intent === "save") {
      const providerType = formData.get("provider_type") as string;

      let config: EmailProviderConfig;

      if (providerType === "none") {
        config = { type: "none" };
      } else if (providerType === "smtp") {
        config = {
          type: "smtp",
          host: formData.get("host") as string,
          port: parseInt(formData.get("port") as string, 10) || 587,
          username: (formData.get("username") as string) || undefined,
          password: (formData.get("password") as string) || undefined,
          use_tls: formData.get("use_tls") === "on",
          from_email: formData.get("from_email") as string,
          from_name: (formData.get("from_name") as string) || undefined,
        };
      } else if (providerType === "ses") {
        config = {
          type: "ses",
          region: formData.get("region") as string,
          access_key_id: (formData.get("access_key_id") as string) || undefined,
          secret_access_key: (formData.get("secret_access_key") as string) || undefined,
          from_email: formData.get("from_email") as string,
          from_name: (formData.get("from_name") as string) || undefined,
          configuration_set: (formData.get("configuration_set") as string) || undefined,
        };
      } else if (providerType === "oracle") {
        config = {
          type: "oracle",
          smtp_endpoint: formData.get("smtp_endpoint") as string,
          port: parseInt(formData.get("port") as string, 10) || 587,
          username: formData.get("username") as string,
          password: formData.get("password") as string,
          from_email: formData.get("from_email") as string,
          from_name: (formData.get("from_name") as string) || undefined,
        };
      } else {
        return Response.json({ error: "Invalid provider type" }, { status: 400 });
      }

      await systemApi.updateEmailSettings(config, accessToken || undefined);
      return { success: true, message: "Email settings saved successfully" };
    }

    if (intent === "test_connection") {
      const result = await systemApi.testEmailConnection(accessToken || undefined);
      if (result.success) {
        return { success: true, message: "Connection test successful" };
      } else {
        return Response.json({ error: result.message }, { status: 400 });
      }
    }

    if (intent === "send_test") {
      const toEmail = formData.get("test_email") as string;
      if (!toEmail || !toEmail.includes("@")) {
        return Response.json({ error: "Please enter a valid email address" }, { status: 400 });
      }
      const result = await systemApi.sendTestEmail(toEmail, accessToken || undefined);
      if (result.success) {
        return { success: true, message: `Test email sent to ${toEmail}` };
      } else {
        return Response.json({ error: result.message }, { status: 400 });
      }
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown error";
    return Response.json({ error: message }, { status: 400 });
  }

  return Response.json({ error: "Invalid intent" }, { status: 400 });
}

export default function EmailSettingsPage() {
  const { config } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const submit = useSubmit();

  const [providerType, setProviderType] = useState<string>(config.type);
  const [isTestEmailOpen, setIsTestEmailOpen] = useState(false);
  const [testEmail, setTestEmail] = useState("");
  const [pendingProviderType, setPendingProviderType] = useState<string | null>(null);

  const isSubmitting = navigation.state === "submitting";
  const currentIntent = navigation.formData?.get("intent");

  // Get current provider info for status display
  const currentProviderInfo = getProviderInfo(config);
  const isConfigured = config.type !== "none";

  // Reset provider type when config changes (after successful save)
  useEffect(() => {
    setProviderType(config.type);
  }, [config.type]);

  const handleProviderChange = (newType: string) => {
    // If switching from a configured provider to a different type, show warning
    if (isConfigured && newType !== config.type && newType !== "none") {
      setPendingProviderType(newType);
    } else {
      setProviderType(newType);
    }
  };

  const confirmProviderSwitch = () => {
    if (pendingProviderType) {
      setProviderType(pendingProviderType);
      setPendingProviderType(null);
    }
  };

  const cancelProviderSwitch = () => {
    setPendingProviderType(null);
  };

  const handleTestConnection = () => {
    submit({ intent: "test_connection" }, { method: "post" });
  };

  const handleSendTestEmail = () => {
    submit({ intent: "send_test", test_email: testEmail }, { method: "post" });
  };

  useEffect(() => {
    if (actionData && "success" in actionData && actionData.success && isTestEmailOpen) {
      setIsTestEmailOpen(false);
      setTestEmail("");
    }
  }, [actionData]);

  return (
    <div className="space-y-6">
      {actionData && "success" in actionData && actionData.success && (
        <div className="rounded-xl bg-[var(--accent-green)]/10 border border-[var(--accent-green)]/20 p-4 text-sm text-[var(--accent-green)]">
          {actionData.message}
        </div>
      )}

      {actionData && "error" in actionData && (
        <div className="rounded-xl bg-red-50 border border-red-200 p-4 text-sm text-red-700">
          {String(actionData.error)}
        </div>
      )}

      {/* Current Status Card */}
      <Card className={isConfigured ? "border-[var(--accent-green)]/20 bg-[var(--accent-green)]/5" : "border-yellow-200 bg-yellow-50/50"}>
        <CardContent className="pt-6">
          <div className="flex items-center gap-3">
            {isConfigured ? (
              <CheckCircledIcon className="h-5 w-5 text-[var(--accent-green)]" />
            ) : (
              <CrossCircledIcon className="h-5 w-5 text-yellow-600" />
            )}
            <div>
              <p className="text-sm font-medium text-[var(--text-primary)]">
                {isConfigured ? "Email Provider Active" : "Email Provider Not Configured"}
              </p>
              {currentProviderInfo ? (
                <p className="text-sm text-[var(--text-secondary)]">
                  Using <span className="font-medium">{currentProviderInfo.name}</span> ({currentProviderInfo.details})
                </p>
              ) : (
                <p className="text-sm text-[var(--text-secondary)]">
                  System emails are disabled. Configure a provider below to enable email functionality.
                </p>
              )}
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Info Banner */}
      <div className="rounded-xl bg-blue-50 border border-blue-200 p-4 text-sm text-blue-700 flex items-start gap-3">
        <ExclamationTriangleIcon className="h-5 w-5 flex-shrink-0 mt-0.5" />
        <div>
          <p className="font-medium">Single Provider Configuration</p>
          <p className="mt-1">
            The system supports one email provider at a time. Switching to a different provider type will replace the current configuration.
          </p>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Email Provider Configuration</CardTitle>
          <CardDescription>
            Select and configure your email provider. Sensitive fields like passwords are stored encrypted.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Form method="post" className="space-y-6">
            <input type="hidden" name="intent" value="save" />

            <div className="space-y-2">
              <Label htmlFor="provider_type">Provider Type</Label>
              <Select
                name="provider_type"
                value={providerType}
                onValueChange={handleProviderChange}
              >
                <SelectTrigger className="w-full max-w-xs">
                  <SelectValue placeholder="Select a provider" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">None (Email disabled)</SelectItem>
                  <SelectItem value="smtp">SMTP</SelectItem>
                  <SelectItem value="ses">AWS SES</SelectItem>
                  <SelectItem value="oracle">Oracle Email Delivery</SelectItem>
                </SelectContent>
              </Select>
              {providerType !== config.type && providerType !== "none" && config.type !== "none" && (
                <p className="text-xs text-amber-600 mt-1">
                  Saving will replace your current {PROVIDER_LABELS[config.type]} configuration.
                </p>
              )}
            </div>

            {/* SMTP Configuration */}
            {providerType === "smtp" && (
              <div className="space-y-4 border-t pt-4">
                <h3 className="text-sm font-medium text-[var(--text-primary)]">SMTP Configuration</h3>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div className="space-y-2">
                    <Label htmlFor="host">Server Host</Label>
                    <Input
                      id="host"
                      name="host"
                      placeholder="smtp.example.com"
                      defaultValue={config.type === "smtp" ? config.host : ""}
                      required
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="port">Port</Label>
                    <Input
                      id="port"
                      name="port"
                      type="number"
                      placeholder="587"
                      defaultValue={config.type === "smtp" ? config.port : 587}
                      required
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="username">Username</Label>
                    <Input
                      id="username"
                      name="username"
                      placeholder="username"
                      defaultValue={config.type === "smtp" ? config.username || "" : ""}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="password">Password</Label>
                    <Input
                      id="password"
                      name="password"
                      type="password"
                      placeholder={config.type === "smtp" && config.password ? "***" : "Enter password"}
                      defaultValue=""
                    />
                    {config.type === "smtp" && config.password && (
                      <p className="text-xs text-[var(--text-secondary)]">Leave blank to keep existing password</p>
                    )}
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="from_email">From Email</Label>
                    <Input
                      id="from_email"
                      name="from_email"
                      type="email"
                      placeholder="noreply@example.com"
                      defaultValue={config.type === "smtp" ? config.from_email : ""}
                      required
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="from_name">From Name</Label>
                    <Input
                      id="from_name"
                      name="from_name"
                      placeholder="Auth9"
                      defaultValue={config.type === "smtp" ? config.from_name || "" : ""}
                    />
                  </div>
                </div>
                <div className="flex items-center space-x-2">
                  <Checkbox
                    id="use_tls"
                    name="use_tls"
                    defaultChecked={config.type === "smtp" ? config.use_tls : true}
                  />
                  <Label htmlFor="use_tls" className="font-normal cursor-pointer">
                    Use TLS encryption
                  </Label>
                </div>
              </div>
            )}

            {/* AWS SES Configuration */}
            {providerType === "ses" && (
              <div className="space-y-4 border-t pt-4">
                <h3 className="text-sm font-medium text-[var(--text-primary)]">AWS SES Configuration</h3>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div className="space-y-2">
                    <Label htmlFor="region">AWS Region</Label>
                    <Input
                      id="region"
                      name="region"
                      placeholder="us-east-1"
                      defaultValue={config.type === "ses" ? config.region : ""}
                      required
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="access_key_id">Access Key ID</Label>
                    <Input
                      id="access_key_id"
                      name="access_key_id"
                      placeholder="AKIA..."
                      defaultValue={config.type === "ses" ? config.access_key_id || "" : ""}
                    />
                    <p className="text-xs text-[var(--text-secondary)]">Optional if using IAM role</p>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="secret_access_key">Secret Access Key</Label>
                    <Input
                      id="secret_access_key"
                      name="secret_access_key"
                      type="password"
                      placeholder={config.type === "ses" && config.secret_access_key ? "***" : "Enter secret key"}
                      defaultValue=""
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="configuration_set">Configuration Set</Label>
                    <Input
                      id="configuration_set"
                      name="configuration_set"
                      placeholder="Optional"
                      defaultValue={config.type === "ses" ? config.configuration_set || "" : ""}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="from_email">From Email</Label>
                    <Input
                      id="from_email"
                      name="from_email"
                      type="email"
                      placeholder="noreply@example.com"
                      defaultValue={config.type === "ses" ? config.from_email : ""}
                      required
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="from_name">From Name</Label>
                    <Input
                      id="from_name"
                      name="from_name"
                      placeholder="Auth9"
                      defaultValue={config.type === "ses" ? config.from_name || "" : ""}
                    />
                  </div>
                </div>
              </div>
            )}

            {/* Oracle Email Delivery Configuration */}
            {providerType === "oracle" && (
              <div className="space-y-4 border-t pt-4">
                <h3 className="text-sm font-medium text-[var(--text-primary)]">Oracle Email Delivery Configuration</h3>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div className="space-y-2">
                    <Label htmlFor="smtp_endpoint">SMTP Endpoint</Label>
                    <Input
                      id="smtp_endpoint"
                      name="smtp_endpoint"
                      placeholder="smtp.us-ashburn-1.oraclecloud.com"
                      defaultValue={config.type === "oracle" ? config.smtp_endpoint : ""}
                      required
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="port">Port</Label>
                    <Input
                      id="port"
                      name="port"
                      type="number"
                      placeholder="587"
                      defaultValue={config.type === "oracle" ? config.port : 587}
                      required
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="username">SMTP Username</Label>
                    <Input
                      id="username"
                      name="username"
                      placeholder="ocid1.user..."
                      defaultValue={config.type === "oracle" ? config.username : ""}
                      required
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="password">SMTP Password</Label>
                    <Input
                      id="password"
                      name="password"
                      type="password"
                      placeholder={config.type === "oracle" && config.password ? "***" : "Enter password"}
                      defaultValue=""
                      required={config.type !== "oracle"}
                    />
                    {config.type === "oracle" && config.password && (
                      <p className="text-xs text-[var(--text-secondary)]">Leave blank to keep existing password</p>
                    )}
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="from_email">From Email</Label>
                    <Input
                      id="from_email"
                      name="from_email"
                      type="email"
                      placeholder="noreply@example.com"
                      defaultValue={config.type === "oracle" ? config.from_email : ""}
                      required
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="from_name">From Name</Label>
                    <Input
                      id="from_name"
                      name="from_name"
                      placeholder="Auth9"
                      defaultValue={config.type === "oracle" ? config.from_name || "" : ""}
                    />
                  </div>
                </div>
              </div>
            )}

            {/* Action Buttons */}
            <div className="flex flex-wrap items-center gap-3 border-t pt-4">
              <Button type="submit" disabled={isSubmitting && currentIntent === "save"}>
                {isSubmitting && currentIntent === "save" ? "Saving..." : "Save Settings"}
              </Button>

              {providerType !== "none" && (
                <>
                  <Button
                    type="button"
                    variant="outline"
                    onClick={handleTestConnection}
                    disabled={isSubmitting}
                  >
                    {isSubmitting && currentIntent === "test_connection" ? "Testing..." : "Test Connection"}
                  </Button>

                  <Button
                    type="button"
                    variant="outline"
                    onClick={() => setIsTestEmailOpen(true)}
                    disabled={isSubmitting}
                  >
                    Send Test Email
                  </Button>
                </>
              )}
            </div>
          </Form>
        </CardContent>
      </Card>

      {/* Test Email Dialog */}
      <Dialog open={isTestEmailOpen} onOpenChange={setIsTestEmailOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Send Test Email</DialogTitle>
            <DialogDescription>
              Enter an email address to receive a test email and verify your configuration.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="test_email_input">Email Address</Label>
              <Input
                id="test_email_input"
                type="email"
                placeholder="your@email.com"
                value={testEmail}
                onChange={(e) => setTestEmail(e.target.value)}
              />
            </div>
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => setIsTestEmailOpen(false)}>
              Cancel
            </Button>
            <Button
              type="button"
              onClick={handleSendTestEmail}
              disabled={!testEmail.includes("@") || isSubmitting}
            >
              Send Test Email
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Provider Switch Confirmation Dialog */}
      <AlertDialog open={pendingProviderType !== null} onOpenChange={(open) => !open && cancelProviderSwitch()}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Switch Email Provider?</AlertDialogTitle>
            <AlertDialogDescription>
              You are about to switch from <strong>{PROVIDER_LABELS[config.type]}</strong> to{" "}
              <strong>{pendingProviderType ? PROVIDER_LABELS[pendingProviderType] : ""}</strong>.
              <br /><br />
              Your current configuration will be replaced when you save. This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel onClick={cancelProviderSwitch}>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={confirmProviderSwitch}>
              Switch Provider
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
