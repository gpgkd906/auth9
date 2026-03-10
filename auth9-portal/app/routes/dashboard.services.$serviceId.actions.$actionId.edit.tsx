import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { Form, Link, useActionData, useLoaderData, useNavigation } from "react-router";
import { redirect } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Textarea } from "~/components/ui/textarea";
import { Switch } from "~/components/ui/switch";
import { getAuth9Client, withService, getTriggers } from "~/lib/auth9-client";
import { FormattedDate } from "~/components/ui/formatted-date";
import { getAccessToken } from "~/services/session.server";
import { ArrowLeftIcon } from "@radix-ui/react-icons";
import { useState } from "react";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";
import { mapApiError } from "~/lib/error-messages";
import { getActionContextReference, getActionTriggerLabel } from "~/lib/service-actions";

export const meta: MetaFunction<typeof loader> = ({ data, matches }) => {
  const locale = resolveMetaLocale(matches);
  return buildMeta(locale, "serviceActions.editMetaTitle", undefined, {
    actionName: data?.action.name || translate(locale, "serviceActions.title"),
  });
};

export async function loader({ params, request }: LoaderFunctionArgs) {
  const { serviceId, actionId } = params;
  const locale = await resolveLocale(request);
  if (!serviceId || !actionId) throw new Error(translate(locale, "serviceActions.errors.serviceAndActionIdRequired"));
  const accessToken = await getAccessToken(request);

  const client = getAuth9Client(accessToken || undefined);
  const api = withService(client, serviceId);

  const [actionRes, triggersRes] = await Promise.all([api.actions.get(actionId), getTriggers(client)]);

  return { locale, serviceId, action: actionRes.data, triggers: triggersRes.data };
}

export async function action({ params, request }: ActionFunctionArgs) {
  const { serviceId, actionId } = params;
  const locale = await resolveLocale(request);
  if (!serviceId || !actionId) return Response.json({ error: translate(locale, "serviceActions.errors.idsRequired") }, { status: 400 });
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

    await api.actions.update(actionId, { name, description: description || undefined, script, enabled, strictMode, executionOrder, timeoutMs });
    return redirect(`/dashboard/services/${serviceId}/actions/${actionId}`);
  } catch (error) {
    const message = mapApiError(error, locale);
    return { error: message };
  }
}

export default function EditActionPage() {
  const { serviceId, action, locale } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const { t, i18n } = useI18n();
  const effectiveLocale = (locale || i18n.resolvedLanguage || "zh-CN") as "zh-CN" | "en-US";
  const isSubmitting = navigation.state === "submitting";
  const [script, setScript] = useState(action.script);

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="icon" asChild>
          <Link to={`/dashboard/services/${serviceId}/actions/${action.id}`} aria-label={t("common.buttons.back")}><ArrowLeftIcon className="h-4 w-4" /></Link>
        </Button>
        <div>
          <h1 className="text-3xl font-bold">{t("serviceActions.editTitle")}</h1>
          <p className="text-muted-foreground mt-1">{action.name}</p>
        </div>
      </div>

      {actionData && typeof actionData === "object" && "error" in actionData && <div className="p-4 bg-destructive/10 text-destructive rounded-md">{String(actionData.error)}</div>}

      <Form method="post" className="space-y-6">
        <Card>
          <CardHeader>
            <CardTitle>{t("serviceActions.basicInformation")}</CardTitle>
            <CardDescription>{t("serviceActions.updateBasicInformationDescription")}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2"><Label htmlFor="name">{t("serviceActions.name")} *</Label><Input id="name" name="name" defaultValue={action.name} required /></div>
            <div className="space-y-2"><Label htmlFor="description">{t("serviceActions.descriptionLabel")}</Label><Input id="description" name="description" defaultValue={action.description || ""} /></div>
            <div className="space-y-2">
              <Label>{t("serviceActions.trigger")}</Label>
              <div className="p-2 bg-muted rounded-md">{getActionTriggerLabel(effectiveLocale, action.triggerId)}</div>
              <p className="text-sm text-muted-foreground">{t("serviceActions.triggerImmutable")}</p>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2"><Label htmlFor="execution_order">{t("serviceActions.executionOrder")}</Label><Input id="execution_order" name="execution_order" type="number" defaultValue={action.executionOrder} min="0" /></div>
              <div className="space-y-2"><Label htmlFor="timeout_ms">{t("serviceActions.timeout")}</Label><Input id="timeout_ms" name="timeout_ms" type="number" defaultValue={action.timeoutMs} min="100" max="30000" /></div>
            </div>
            <div className="flex items-center space-x-2"><Switch id="enabled" name="enabled" defaultChecked={action.enabled} /><Label htmlFor="enabled">{t("serviceActions.enabled")}</Label></div>
            <div className="flex items-center space-x-2"><Switch id="strict_mode" name="strict_mode" defaultChecked={action.strictMode} /><Label htmlFor="strict_mode">{t("serviceActions.strictMode")}</Label><span className="text-sm text-muted-foreground">{t("serviceActions.strictModeHint")}</span></div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader><CardTitle>{t("serviceActions.script")}</CardTitle><CardDescription>{t("serviceActions.scriptEditDescription")}</CardDescription></CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2"><Label htmlFor="script">{t("serviceActions.scriptCode")} *</Label><Textarea id="script" name="script" value={script} onChange={(e) => setScript(e.target.value)} className="font-mono text-sm min-h-[400px]" required /></div>
            <div className="p-4 bg-muted rounded-md space-y-2"><div className="font-semibold text-sm">{t("serviceActions.contextStructure")}</div><pre className="text-xs overflow-x-auto">{getActionContextReference()}</pre></div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader><CardTitle>{t("serviceActions.statistics.executionStatistics")}</CardTitle></CardHeader>
          <CardContent>
            <div className="grid grid-cols-3 gap-4 text-sm">
              <div><div className="text-muted-foreground mb-1">{t("serviceActions.statistics.totalExecutions")}</div><div className="text-2xl font-bold">{action.executionCount.toLocaleString()}</div></div>
              <div><div className="text-muted-foreground mb-1">{t("serviceActions.statistics.errors")}</div><div className="text-2xl font-bold text-destructive">{action.errorCount.toLocaleString()}</div></div>
              <div><div className="text-muted-foreground mb-1">{t("serviceActions.lastExecuted")}</div><div className="text-sm font-semibold">{action.lastExecutedAt ? <FormattedDate date={action.lastExecutedAt} /> : t("serviceActions.never")}</div></div>
            </div>
            {action.lastError && <div className="mt-4 p-3 bg-destructive/10 rounded-md"><div className="text-sm font-medium text-destructive mb-1">{t("serviceActions.lastError")}</div><div className="text-sm text-muted-foreground">{action.lastError}</div></div>}
          </CardContent>
        </Card>

        <div className="flex gap-2">
          <Button type="submit" disabled={isSubmitting}>{isSubmitting ? t("serviceActions.saving") : t("serviceActions.saveChanges")}</Button>
          <Button type="button" variant="outline" asChild><Link to={`/dashboard/services/${serviceId}/actions/${action.id}`}>{t("common.buttons.cancel")}</Link></Button>
        </div>
      </Form>
    </div>
  );
}
