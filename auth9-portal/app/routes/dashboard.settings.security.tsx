import type { ActionFunctionArgs, LoaderFunctionArgs } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation } from "react-router";
import { useEffect, useState } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Switch } from "~/components/ui/switch";
import { passwordApi, tenantApi, type PasswordPolicy, type Tenant } from "~/services/api";
import { getAccessToken } from "~/services/session.server";

export async function loader({ request }: LoaderFunctionArgs) {
  // Load tenants for password policy management (admin only)
  const accessToken = await getAccessToken(request);
  const tenantsResponse = await tenantApi.list(1, 100, undefined, accessToken || undefined);
  return { tenants: tenantsResponse.data };
}

export async function action({ request }: ActionFunctionArgs) {
  const formData = await request.formData();
  const intent = formData.get("intent");

  try {
    if (intent === "update_policy") {
      const tenantId = formData.get("tenantId") as string;
      const policy: Partial<PasswordPolicy> = {
        min_length: parseInt(formData.get("minLength") as string) || 8,
        require_uppercase: formData.get("requireUppercase") === "true",
        require_lowercase: formData.get("requireLowercase") === "true",
        require_numbers: formData.get("requireNumbers") === "true",
        require_symbols: formData.get("requireSymbols") === "true",
        max_age_days: parseInt(formData.get("maxAgeDays") as string) || 0,
        history_count: parseInt(formData.get("historyCount") as string) || 0,
        lockout_threshold: parseInt(formData.get("lockoutThreshold") as string) || 0,
        lockout_duration_mins: parseInt(formData.get("lockoutDurationMins") as string) || 15,
      };

      await passwordApi.updatePasswordPolicy(tenantId, policy);
      return { success: true, message: "Password policy updated" };
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : "Operation failed";
    return { error: message };
  }

  return { error: "Invalid action" };
}

export default function SecuritySettingsPage() {
  const { tenants } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();

  const [selectedTenant, setSelectedTenant] = useState<string>("");
  const [policy, setPolicy] = useState<PasswordPolicy | null>(null);
  const [loadingPolicy, setLoadingPolicy] = useState(false);

  const isSubmitting = navigation.state === "submitting";

  // Load policy when tenant is selected
  useEffect(() => {
    if (selectedTenant) {
      setLoadingPolicy(true);
      passwordApi
        .getPasswordPolicy(selectedTenant)
        .then((res) => setPolicy(res.data))
        .catch(() => setPolicy(null))
        .finally(() => setLoadingPolicy(false));
    } else {
      setPolicy(null);
    }
  }, [selectedTenant]);

  return (
    <div className="space-y-6">
      {/* Password Policy Section (Admin only) */}
      <Card>
        <CardHeader>
          <CardTitle>Password Policy</CardTitle>
          <CardDescription>
            Configure password requirements for tenant users.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            <div className="space-y-2 max-w-xs">
              <Label htmlFor="tenantSelect">Select Tenant</Label>
              <select
                id="tenantSelect"
                value={selectedTenant}
                onChange={(e) => setSelectedTenant(e.target.value)}
                className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
              >
                <option value="">Select a tenant...</option>
                {tenants.map((tenant: Tenant) => (
                  <option key={tenant.id} value={tenant.id}>
                    {tenant.name}
                  </option>
                ))}
              </select>
            </div>

            {loadingPolicy && (
              <p className="text-sm text-[var(--text-secondary)]">Loading policy...</p>
            )}

            {selectedTenant && policy && (
              <Form method="post" className="space-y-6">
                <input type="hidden" name="intent" value="update_policy" />
                <input type="hidden" name="tenantId" value={selectedTenant} />

                <div className="grid gap-6 md:grid-cols-2">
                  <div className="space-y-2">
                    <Label htmlFor="minLength">Minimum length</Label>
                    <Input
                      id="minLength"
                      name="minLength"
                      type="number"
                      min={6}
                      max={128}
                      defaultValue={policy.min_length}
                    />
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="maxAgeDays">Password expiry (days)</Label>
                    <Input
                      id="maxAgeDays"
                      name="maxAgeDays"
                      type="number"
                      min={0}
                      max={365}
                      defaultValue={policy.max_age_days}
                    />
                    <p className="text-xs text-[var(--text-secondary)]">0 = never expires</p>
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="historyCount">Password history</Label>
                    <Input
                      id="historyCount"
                      name="historyCount"
                      type="number"
                      min={0}
                      max={24}
                      defaultValue={policy.history_count}
                    />
                    <p className="text-xs text-[var(--text-secondary)]">Previous passwords to remember</p>
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="lockoutThreshold">Lockout after</Label>
                    <Input
                      id="lockoutThreshold"
                      name="lockoutThreshold"
                      type="number"
                      min={0}
                      max={100}
                      defaultValue={policy.lockout_threshold}
                    />
                    <p className="text-xs text-[var(--text-secondary)]">Failed attempts (0 = disabled)</p>
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="lockoutDurationMins">Lockout duration (mins)</Label>
                    <Input
                      id="lockoutDurationMins"
                      name="lockoutDurationMins"
                      type="number"
                      min={1}
                      max={1440}
                      defaultValue={policy.lockout_duration_mins}
                    />
                  </div>
                </div>

                <div className="space-y-4">
                  <h4 className="text-sm font-medium">Character requirements</h4>
                  <div className="grid gap-4 md:grid-cols-2">
                    <div className="flex items-center justify-between">
                      <Label htmlFor="requireUppercase">Require uppercase</Label>
                      <input
                        type="hidden"
                        name="requireUppercase"
                        value={policy.require_uppercase ? "true" : "false"}
                      />
                      <Switch
                        id="requireUppercase"
                        defaultChecked={policy.require_uppercase}
                        onCheckedChange={(checked: boolean) => {
                          const input = document.querySelector(
                            'input[name="requireUppercase"]'
                          ) as HTMLInputElement;
                          if (input) input.value = checked ? "true" : "false";
                        }}
                      />
                    </div>

                    <div className="flex items-center justify-between">
                      <Label htmlFor="requireLowercase">Require lowercase</Label>
                      <input
                        type="hidden"
                        name="requireLowercase"
                        value={policy.require_lowercase ? "true" : "false"}
                      />
                      <Switch
                        id="requireLowercase"
                        defaultChecked={policy.require_lowercase}
                        onCheckedChange={(checked: boolean) => {
                          const input = document.querySelector(
                            'input[name="requireLowercase"]'
                          ) as HTMLInputElement;
                          if (input) input.value = checked ? "true" : "false";
                        }}
                      />
                    </div>

                    <div className="flex items-center justify-between">
                      <Label htmlFor="requireNumbers">Require numbers</Label>
                      <input
                        type="hidden"
                        name="requireNumbers"
                        value={policy.require_numbers ? "true" : "false"}
                      />
                      <Switch
                        id="requireNumbers"
                        defaultChecked={policy.require_numbers}
                        onCheckedChange={(checked: boolean) => {
                          const input = document.querySelector(
                            'input[name="requireNumbers"]'
                          ) as HTMLInputElement;
                          if (input) input.value = checked ? "true" : "false";
                        }}
                      />
                    </div>

                    <div className="flex items-center justify-between">
                      <Label htmlFor="requireSymbols">Require symbols</Label>
                      <input
                        type="hidden"
                        name="requireSymbols"
                        value={policy.require_symbols ? "true" : "false"}
                      />
                      <Switch
                        id="requireSymbols"
                        defaultChecked={policy.require_symbols}
                        onCheckedChange={(checked: boolean) => {
                          const input = document.querySelector(
                            'input[name="requireSymbols"]'
                          ) as HTMLInputElement;
                          if (input) input.value = checked ? "true" : "false";
                        }}
                      />
                    </div>
                  </div>
                </div>

                {actionData?.error && (
                  <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md">
                    {actionData.error}
                  </div>
                )}

                {actionData?.success && (
                  <div className="text-sm text-[var(--accent-green)] bg-[var(--accent-green)]/10 p-3 rounded-md">
                    {actionData.message}
                  </div>
                )}

                <Button type="submit" disabled={isSubmitting}>
                  {isSubmitting ? "Saving..." : "Save policy"}
                </Button>
              </Form>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
