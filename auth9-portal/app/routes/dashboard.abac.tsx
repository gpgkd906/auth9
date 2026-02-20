import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation } from "react-router";
import { useMemo } from "react";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Textarea } from "~/components/ui/textarea";
import {
  abacApi,
  type AbacMode,
  type AbacPolicyDocument,
  type AbacPolicyListPayload,
  type AbacSimulationResult,
} from "~/services/api";
import { getAccessToken, getSession } from "~/services/session.server";

export const meta: MetaFunction = () => {
  return [{ title: "ABAC Policies - Auth9" }];
};

const SAMPLE_POLICY_JSON = JSON.stringify(
  {
    rules: [
      {
        id: "deny_non_work_hours",
        effect: "deny",
        actions: ["user_manage", "rbac_write"],
        resource_types: ["tenant"],
        priority: 100,
        condition: {
          not: {
            var: "env.hour",
            op: "time_between",
            value: "09:00-18:00",
          },
        },
      },
      {
        id: "allow_owner_admin",
        effect: "allow",
        actions: ["user_manage", "invitation_write", "rbac_write"],
        resource_types: ["tenant"],
        priority: 50,
        condition: {
          any: [
            { var: "subject.roles", op: "contains", value: "owner" },
            { var: "subject.roles", op: "contains", value: "admin" },
          ],
        },
      },
    ],
  },
  null,
  2
);

function parsePolicyJson(raw: FormDataEntryValue | null): AbacPolicyDocument {
  const text = (raw || "").toString().trim();
  if (!text) {
    return { rules: [] };
  }
  const parsed = JSON.parse(text) as AbacPolicyDocument;
  if (!parsed || typeof parsed !== "object" || !Array.isArray(parsed.rules)) {
    throw new Error("Policy JSON must be an object with a rules array");
  }
  return parsed;
}

function parseJsonObject(raw: FormDataEntryValue | null, fieldName: string): Record<string, unknown> {
  const text = (raw || "").toString().trim();
  if (!text) return {};
  const parsed = JSON.parse(text);
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error(`${fieldName} must be a JSON object`);
  }
  return parsed as Record<string, unknown>;
}

async function resolveTenantContext(request: Request): Promise<{ tenantId: string; accessToken: string }> {
  const session = await getSession(request);
  const tenantId = session?.activeTenantId;
  if (!tenantId) {
    throw new Error("No active tenant selected");
  }
  const accessToken = await getAccessToken(request);
  if (!accessToken) {
    throw new Error("Not authenticated");
  }
  return { tenantId, accessToken };
}

export async function loader({ request }: LoaderFunctionArgs) {
  const { tenantId, accessToken } = await resolveTenantContext(request);
  const data = await abacApi.listPolicies(tenantId, accessToken);
  return {
    tenantId,
    payload: data.data as AbacPolicyListPayload,
  };
}

export async function action({ request }: ActionFunctionArgs) {
  try {
    const { tenantId, accessToken } = await resolveTenantContext(request);
    const formData = await request.formData();
    const intent = (formData.get("intent") || "").toString();

    if (intent === "create_draft") {
      const policy = parsePolicyJson(formData.get("policy_json"));
      const change_note = (formData.get("change_note") || "").toString().trim();
      await abacApi.createDraft(
        tenantId,
        { policy, change_note: change_note || undefined },
        accessToken
      );
      return { success: true, intent };
    }

    if (intent === "update_draft") {
      const versionId = (formData.get("version_id") || "").toString();
      const policy = parsePolicyJson(formData.get("policy_json"));
      const change_note = (formData.get("change_note") || "").toString().trim();
      await abacApi.updateDraft(
        tenantId,
        versionId,
        { policy, change_note: change_note || undefined },
        accessToken
      );
      return { success: true, intent };
    }

    if (intent === "publish" || intent === "rollback") {
      const versionId = (formData.get("version_id") || "").toString();
      const mode = ((formData.get("mode") || "enforce").toString() as AbacMode);
      if (intent === "publish") {
        await abacApi.publish(tenantId, versionId, mode, accessToken);
      } else {
        await abacApi.rollback(tenantId, versionId, mode, accessToken);
      }
      return { success: true, intent };
    }

    if (intent === "simulate") {
      const actionName = (formData.get("sim_action") || "").toString().trim();
      const resourceType = (formData.get("sim_resource_type") || "").toString().trim();
      if (!actionName || !resourceType) {
        throw new Error("Simulation action and resource type are required");
      }

      const includeDraftPolicy = (formData.get("sim_with_policy") || "").toString() === "on";
      const policy = includeDraftPolicy ? parsePolicyJson(formData.get("policy_json")) : undefined;
      const subject = parseJsonObject(formData.get("sim_subject_json"), "subject");
      const resource = parseJsonObject(formData.get("sim_resource_json"), "resource");
      const requestObj = parseJsonObject(formData.get("sim_request_json"), "request");
      const env = parseJsonObject(formData.get("sim_env_json"), "env");

      const response = await abacApi.simulate(
        tenantId,
        {
          policy,
          simulation: {
            action: actionName,
            resource_type: resourceType,
            subject,
            resource,
            request: requestObj,
            env,
          },
        },
        accessToken
      );
      return { success: true, intent, simulation: response.data as AbacSimulationResult };
    }

    return Response.json({ error: "Invalid intent" }, { status: 400 });
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown error";
    return Response.json({ error: message }, { status: 400 });
  }
}

export default function AbacPoliciesPage() {
  const { tenantId, payload } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>() as
    | { success?: boolean; error?: string; simulation?: AbacSimulationResult }
    | undefined;
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";

  const publishedVersion = useMemo(() => {
    if (!payload.policy_set?.published_version_id) return null;
    return payload.versions.find((v) => v.id === payload.policy_set?.published_version_id) || null;
  }, [payload.policy_set?.published_version_id, payload.versions]);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-[24px] font-semibold text-[var(--text-primary)] tracking-tight">ABAC Policies</h1>
        <p className="text-sm text-[var(--text-secondary)]">
          Tenant {tenantId} 的属性策略管理（草稿、发布、回滚、模拟）
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>当前状态</CardTitle>
          <CardDescription>Policy mode 与当前发布版本</CardDescription>
        </CardHeader>
        <div className="px-6 pb-6 text-sm text-[var(--text-secondary)] space-y-2">
          <div>Mode: <span className="font-medium text-[var(--text-primary)]">{payload.policy_set?.mode || "disabled"}</span></div>
          <div>
            Published:{" "}
            <span className="font-medium text-[var(--text-primary)]">
              {publishedVersion ? `v${publishedVersion.version_no} (${publishedVersion.id})` : "None"}
            </span>
          </div>
        </div>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>创建草稿</CardTitle>
          <CardDescription>提交新的 ABAC policy draft 版本</CardDescription>
        </CardHeader>
        <div className="px-6 pb-6">
          <Form method="post" className="space-y-4">
            <input type="hidden" name="intent" value="create_draft" />
            <div className="space-y-1.5">
              <Label htmlFor="change_note">Change Note</Label>
              <Input id="change_note" name="change_note" placeholder="why this version is created" />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="policy_json">Policy JSON</Label>
              <Textarea id="policy_json" name="policy_json" className="min-h-[220px] font-mono text-xs" defaultValue={SAMPLE_POLICY_JSON} />
            </div>
            <Button type="submit" disabled={isSubmitting}>{isSubmitting ? "Submitting..." : "Create Draft"}</Button>
          </Form>
        </div>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>版本列表</CardTitle>
          <CardDescription>发布、回滚与草稿更新</CardDescription>
        </CardHeader>
        <div className="px-6 pb-6 space-y-4">
          {payload.versions.length === 0 && (
            <p className="text-sm text-[var(--text-secondary)]">No ABAC policy version yet.</p>
          )}
          {payload.versions.map((version) => (
            <div key={version.id} className="rounded-xl border border-[var(--glass-border-subtle)] bg-[var(--glass-bg)] p-4 space-y-3">
              <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-2">
                <div>
                  <div className="font-medium text-[var(--text-primary)]">v{version.version_no} · {version.status}</div>
                  <div className="text-xs text-[var(--text-tertiary)]">{version.id}</div>
                </div>
                <div className="text-xs text-[var(--text-tertiary)]">{version.created_at}</div>
              </div>
              <div className="text-sm text-[var(--text-secondary)]">{version.change_note || "No change note"}</div>

              <div className="flex flex-wrap gap-2">
                <Form method="post">
                  <input type="hidden" name="intent" value="publish" />
                  <input type="hidden" name="version_id" value={version.id} />
                  <input type="hidden" name="mode" value="enforce" />
                  <Button type="submit" size="sm" disabled={isSubmitting}>Publish (enforce)</Button>
                </Form>
                <Form method="post">
                  <input type="hidden" name="intent" value="publish" />
                  <input type="hidden" name="version_id" value={version.id} />
                  <input type="hidden" name="mode" value="shadow" />
                  <Button type="submit" size="sm" variant="outline" disabled={isSubmitting}>Publish (shadow)</Button>
                </Form>
                <Form method="post">
                  <input type="hidden" name="intent" value="rollback" />
                  <input type="hidden" name="version_id" value={version.id} />
                  <input type="hidden" name="mode" value="enforce" />
                  <Button type="submit" size="sm" variant="outline" disabled={isSubmitting}>Rollback to This</Button>
                </Form>
              </div>

              {version.status === "draft" && (
                <Form method="post" className="space-y-2">
                  <input type="hidden" name="intent" value="update_draft" />
                  <input type="hidden" name="version_id" value={version.id} />
                  <Label htmlFor={`change-note-${version.id}`}>Update Draft Change Note</Label>
                  <Input id={`change-note-${version.id}`} name="change_note" defaultValue={version.change_note || ""} />
                  <Label htmlFor={`policy-json-${version.id}`}>Draft Policy JSON</Label>
                  <Textarea id={`policy-json-${version.id}`} name="policy_json" className="min-h-[160px] font-mono text-xs" defaultValue={SAMPLE_POLICY_JSON} />
                  <Button type="submit" size="sm" variant="outline" disabled={isSubmitting}>Update Draft</Button>
                </Form>
              )}
            </div>
          ))}
        </div>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>策略模拟</CardTitle>
          <CardDescription>模拟单次决策并查看命中规则</CardDescription>
        </CardHeader>
        <div className="px-6 pb-6">
          <Form method="post" className="space-y-4">
            <input type="hidden" name="intent" value="simulate" />
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div className="space-y-1.5">
                <Label htmlFor="sim_action">Action</Label>
                <Input id="sim_action" name="sim_action" defaultValue="user_manage" />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="sim_resource_type">Resource Type</Label>
                <Input id="sim_resource_type" name="sim_resource_type" defaultValue="tenant" />
              </div>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="sim_subject_json">subject JSON</Label>
              <Textarea id="sim_subject_json" name="sim_subject_json" className="min-h-[90px] font-mono text-xs" defaultValue={JSON.stringify({ roles: ["admin"], email_domain: "example.com" }, null, 2)} />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="sim_resource_json">resource JSON</Label>
              <Textarea id="sim_resource_json" name="sim_resource_json" className="min-h-[90px] font-mono text-xs" defaultValue={JSON.stringify({ tenant_id: tenantId }, null, 2)} />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="sim_request_json">request JSON</Label>
              <Textarea id="sim_request_json" name="sim_request_json" className="min-h-[90px] font-mono text-xs" defaultValue={JSON.stringify({ ip: "127.0.0.1" }, null, 2)} />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="sim_env_json">env JSON</Label>
              <Textarea id="sim_env_json" name="sim_env_json" className="min-h-[90px] font-mono text-xs" defaultValue={JSON.stringify({ hour: 10 }, null, 2)} />
            </div>
            <div className="flex items-center gap-2">
              <input id="sim_with_policy" name="sim_with_policy" type="checkbox" className="h-4 w-4" />
              <Label htmlFor="sim_with_policy">Use Policy JSON below instead of published version</Label>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="sim_policy_json">Policy JSON (optional)</Label>
              <Textarea id="sim_policy_json" name="policy_json" className="min-h-[180px] font-mono text-xs" defaultValue={SAMPLE_POLICY_JSON} />
            </div>
            <Button type="submit" disabled={isSubmitting}>{isSubmitting ? "Simulating..." : "Run Simulation"}</Button>
          </Form>

          {actionData?.simulation && (
            <div className="mt-4 rounded-xl border border-[var(--glass-border-subtle)] bg-[var(--glass-bg)] p-4 text-sm">
              <div>
                Decision:{" "}
                <span className={actionData.simulation.decision === "deny" ? "text-[var(--accent-red)] font-semibold" : "text-[var(--accent-green)] font-semibold"}>
                  {actionData.simulation.decision}
                </span>
              </div>
              <div className="mt-2">Matched allow rules: {actionData.simulation.matched_allow_rule_ids.join(", ") || "None"}</div>
              <div>Matched deny rules: {actionData.simulation.matched_deny_rule_ids.join(", ") || "None"}</div>
            </div>
          )}
        </div>
      </Card>

      {actionData?.error && (
        <div className="rounded-lg border border-[var(--accent-red)]/30 bg-[var(--accent-red)]/10 px-4 py-3 text-sm text-[var(--accent-red)]">
          {actionData.error}
        </div>
      )}
    </div>
  );
}
