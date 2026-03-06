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
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { translate } from "~/i18n/translate";
import { resolveLocale } from "~/services/locale.server";

export const meta: MetaFunction = ({ matches }) => buildMeta(resolveMetaLocale(matches), "abacPage.metaTitle");

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
    throw new Error("abacPage.policyJsonRulesRequired");
  }
  return parsed;
}

function parseJsonObject(raw: FormDataEntryValue | null, fieldName: string): Record<string, unknown> {
  const text = (raw || "").toString().trim();
  if (!text) return {};
  const parsed = JSON.parse(text);
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error(`abacPage.jsonObjectRequired:${fieldName}`);
  }
  return parsed as Record<string, unknown>;
}

function toTranslatedMessage(locale: string, error: unknown) {
  if (error instanceof Error) {
    if (error.message.startsWith("abacPage.jsonObjectRequired:")) {
      const field = error.message.split(":")[1] || "field";
      return translate(locale as never, "abacPage.jsonObjectRequired", { field });
    }
    if (error.message.startsWith("abacPage.")) {
      return translate(locale as never, error.message as never);
    }
    return error.message;
  }
  return translate(locale as never, "abacPage.unknownError");
}

async function resolveTenantContext(request: Request, locale: string): Promise<{ tenantId: string; accessToken: string }> {
  const session = await getSession(request);
  const tenantId = session?.activeTenantId;
  if (!tenantId) {
    throw new Error(translate(locale as never, "abacPage.noActiveTenant"));
  }
  const accessToken = await getAccessToken(request);
  if (!accessToken) {
    throw new Error(translate(locale as never, "abacPage.notAuthenticated"));
  }
  return { tenantId, accessToken };
}

export async function loader({ request }: LoaderFunctionArgs) {
  const locale = await resolveLocale(request);
  const { tenantId, accessToken } = await resolveTenantContext(request, locale);
  const data = await abacApi.listPolicies(tenantId, accessToken);
  return {
    tenantId,
    payload: data.data as AbacPolicyListPayload,
  };
}

export async function action({ request }: ActionFunctionArgs) {
  try {
    const locale = await resolveLocale(request);
    const { tenantId, accessToken } = await resolveTenantContext(request, locale);
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
        throw new Error("abacPage.simulationFieldsRequired");
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

    return Response.json({ error: translate(locale, "abacPage.invalidIntent") }, { status: 400 });
  } catch (error) {
    const locale = await resolveLocale(request);
    const message = toTranslatedMessage(locale, error);
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
  const { t } = useI18n();

  const publishedVersion = useMemo(() => {
    if (!payload.policy_set?.published_version_id) return null;
    return payload.versions.find((v) => v.id === payload.policy_set?.published_version_id) || null;
  }, [payload.policy_set?.published_version_id, payload.versions]);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-[24px] font-semibold text-[var(--text-primary)] tracking-tight">{t("abacPage.title")}</h1>
        <p className="text-sm text-[var(--text-secondary)]">
          {t("abacPage.description", { tenantId })}
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>{t("abacPage.currentStatus")}</CardTitle>
          <CardDescription>{t("abacPage.currentStatusDescription")}</CardDescription>
        </CardHeader>
        <div className="px-6 pb-6 text-sm text-[var(--text-secondary)] space-y-2">
          <div>{t("abacPage.mode")}: <span className="font-medium text-[var(--text-primary)]">{payload.policy_set?.mode || t("abacPage.disabled")}</span></div>
          <div>
            {t("abacPage.published")}:{" "}
            <span className="font-medium text-[var(--text-primary)]">
              {publishedVersion ? `v${publishedVersion.version_no} (${publishedVersion.id})` : t("abacPage.none")}
            </span>
          </div>
        </div>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("abacPage.createDraftTitle")}</CardTitle>
          <CardDescription>{t("abacPage.createDraftDescription")}</CardDescription>
        </CardHeader>
        <div className="px-6 pb-6">
          <Form method="post" className="space-y-4">
            <input type="hidden" name="intent" value="create_draft" />
            <div className="space-y-1.5">
              <Label htmlFor="change_note">{t("abacPage.changeNote")}</Label>
              <Input id="change_note" name="change_note" placeholder={t("abacPage.changeNotePlaceholder")} />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="policy_json">{t("abacPage.policyJson")}</Label>
              <Textarea id="policy_json" name="policy_json" className="min-h-[220px] font-mono text-xs" defaultValue={SAMPLE_POLICY_JSON} />
            </div>
            <Button type="submit" disabled={isSubmitting}>{isSubmitting ? t("abacPage.submitting") : t("abacPage.createDraft")}</Button>
          </Form>
        </div>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("abacPage.versionsTitle")}</CardTitle>
          <CardDescription>{t("abacPage.versionsDescription")}</CardDescription>
        </CardHeader>
        <div className="px-6 pb-6 space-y-4">
          {payload.versions.length === 0 && (
            <p className="text-sm text-[var(--text-secondary)]">{t("abacPage.noVersions")}</p>
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
              <div className="text-sm text-[var(--text-secondary)]">{version.change_note || t("abacPage.noChangeNote")}</div>

              <div className="flex flex-wrap gap-2">
                <Form method="post">
                  <input type="hidden" name="intent" value="publish" />
                  <input type="hidden" name="version_id" value={version.id} />
                  <input type="hidden" name="mode" value="enforce" />
                  <Button type="submit" size="sm" disabled={isSubmitting}>{t("abacPage.publishEnforce")}</Button>
                </Form>
                <Form method="post">
                  <input type="hidden" name="intent" value="publish" />
                  <input type="hidden" name="version_id" value={version.id} />
                  <input type="hidden" name="mode" value="shadow" />
                  <Button type="submit" size="sm" variant="outline" disabled={isSubmitting}>{t("abacPage.publishShadow")}</Button>
                </Form>
                <Form method="post">
                  <input type="hidden" name="intent" value="rollback" />
                  <input type="hidden" name="version_id" value={version.id} />
                  <input type="hidden" name="mode" value="enforce" />
                  <Button type="submit" size="sm" variant="outline" disabled={isSubmitting}>{t("abacPage.rollback")}</Button>
                </Form>
              </div>

              {version.status === "draft" && (
                <Form method="post" className="space-y-2">
                  <input type="hidden" name="intent" value="update_draft" />
                  <input type="hidden" name="version_id" value={version.id} />
                  <Label htmlFor={`change-note-${version.id}`}>{t("abacPage.updateDraftChangeNote")}</Label>
                  <Input id={`change-note-${version.id}`} name="change_note" defaultValue={version.change_note || ""} />
                  <Label htmlFor={`policy-json-${version.id}`}>{t("abacPage.draftPolicyJson")}</Label>
                  <Textarea id={`policy-json-${version.id}`} name="policy_json" className="min-h-[160px] font-mono text-xs" defaultValue={SAMPLE_POLICY_JSON} />
                  <Button type="submit" size="sm" variant="outline" disabled={isSubmitting}>{t("abacPage.updateDraft")}</Button>
                </Form>
              )}
            </div>
          ))}
        </div>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("abacPage.simulationTitle")}</CardTitle>
          <CardDescription>{t("abacPage.simulationDescription")}</CardDescription>
        </CardHeader>
        <div className="px-6 pb-6">
          <Form method="post" className="space-y-4">
            <input type="hidden" name="intent" value="simulate" />
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div className="space-y-1.5">
                <Label htmlFor="sim_action">{t("abacPage.action")}</Label>
                <Input id="sim_action" name="sim_action" defaultValue="user_manage" />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="sim_resource_type">{t("abacPage.resourceType")}</Label>
                <Input id="sim_resource_type" name="sim_resource_type" defaultValue="tenant" />
              </div>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="sim_subject_json">{t("abacPage.subjectJson")}</Label>
              <Textarea id="sim_subject_json" name="sim_subject_json" className="min-h-[90px] font-mono text-xs" defaultValue={JSON.stringify({ roles: ["admin"], email_domain: "example.com" }, null, 2)} />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="sim_resource_json">{t("abacPage.resourceJson")}</Label>
              <Textarea id="sim_resource_json" name="sim_resource_json" className="min-h-[90px] font-mono text-xs" defaultValue={JSON.stringify({ tenant_id: tenantId }, null, 2)} />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="sim_request_json">{t("abacPage.requestJson")}</Label>
              <Textarea id="sim_request_json" name="sim_request_json" className="min-h-[90px] font-mono text-xs" defaultValue={JSON.stringify({ ip: "127.0.0.1" }, null, 2)} />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="sim_env_json">{t("abacPage.envJson")}</Label>
              <Textarea id="sim_env_json" name="sim_env_json" className="min-h-[90px] font-mono text-xs" defaultValue={JSON.stringify({ hour: 10 }, null, 2)} />
            </div>
            <div className="flex items-center gap-2">
              <input id="sim_with_policy" name="sim_with_policy" type="checkbox" className="h-4 w-4" />
              <Label htmlFor="sim_with_policy">{t("abacPage.usePolicyJson")}</Label>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="sim_policy_json">{t("abacPage.optionalPolicyJson")}</Label>
              <Textarea id="sim_policy_json" name="policy_json" className="min-h-[180px] font-mono text-xs" defaultValue={SAMPLE_POLICY_JSON} />
            </div>
            <Button type="submit" disabled={isSubmitting}>{isSubmitting ? t("abacPage.simulating") : t("abacPage.runSimulation")}</Button>
          </Form>

          {actionData?.simulation && (
            <div className="mt-4 rounded-xl border border-[var(--glass-border-subtle)] bg-[var(--glass-bg)] p-4 text-sm">
              <div>
                {t("abacPage.decision")}:{" "}
                <span className={actionData.simulation.decision === "deny" ? "text-[var(--accent-red)] font-semibold" : "text-[var(--accent-green)] font-semibold"}>
                  {actionData.simulation.decision}
                </span>
              </div>
              <div className="mt-2">{t("abacPage.matchedAllowRules")}: {actionData.simulation.matched_allow_rule_ids.join(", ") || t("abacPage.none")}</div>
              <div>{t("abacPage.matchedDenyRules")}: {actionData.simulation.matched_deny_rule_ids.join(", ") || t("abacPage.none")}</div>
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
