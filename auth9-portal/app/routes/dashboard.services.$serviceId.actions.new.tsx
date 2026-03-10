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
import { getAuth9Client, withService, getTriggers } from "~/lib/auth9-client";
import { getAccessToken } from "~/services/session.server";
import { ArrowLeftIcon } from "@radix-ui/react-icons";
import { useState } from "react";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";
import { mapApiError } from "~/lib/error-messages";
import { getActionContextReference, getActionScriptTemplates, getActionTriggerLabel, getDefaultActionScript } from "~/lib/service-actions";

export const meta: MetaFunction = ({ matches }) => buildMeta(resolveMetaLocale(matches), "serviceActions.newMetaTitle");

export async function loader({ params, request }: LoaderFunctionArgs) {
  const { serviceId } = params;
  const locale = await resolveLocale(request);
  if (!serviceId) throw new Error(translate(locale, "serviceActions.errors.serviceIdRequired"));
  const accessToken = await getAccessToken(request);

  const client = getAuth9Client(accessToken || undefined);
  const triggersRes = await getTriggers(client);

  return { locale, serviceId, triggers: triggersRes.data };
}

export async function action({ params, request }: ActionFunctionArgs) {
  const { serviceId } = params;
  const locale = await resolveLocale(request);
  if (!serviceId) return Response.json({ error: translate(locale, "serviceActions.errors.serviceIdRequired") }, { status: 400 });
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
    const api = withService(client, serviceId);

    const result = await api.actions.create({ name, description: description || undefined, triggerId, script, enabled, strictMode, executionOrder, timeoutMs });
    return redirect(`/dashboard/services/${serviceId}/actions/${result.data.id}`);
  } catch (error) {
    const message = mapApiError(error, locale);
    return { error: message };
  }
}

export default function NewActionPage() {
  const { serviceId, triggers, locale } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const { t, i18n } = useI18n();
  const isSubmitting = navigation.state === "submitting";
  const effectiveLocale = (locale || i18n.resolvedLanguage || "zh-CN") as "zh-CN" | "en-US";

  const templates = getActionScriptTemplates(effectiveLocale);
  const [selectedTemplate, setSelectedTemplate] = useState<string>("");
  const [script, setScript] = useState<string>(getDefaultActionScript(effectiveLocale));

  const handleTemplateSelect = (templateId: string) => {
    setSelectedTemplate(templateId);
    if (templateId && templates[templateId as keyof typeof templates]) {
      setScript(templates[templateId as keyof typeof templates].script);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="icon" asChild>
          <Link to={`/dashboard/services/${serviceId}/actions`} aria-label={t("common.buttons.back")}>
            <ArrowLeftIcon className="h-4 w-4" />
          </Link>
        </Button>
        <div>
          <h1 className="text-3xl font-bold">{t("serviceActions.newTitle")}</h1>
          <p className="text-muted-foreground mt-1">{t("serviceActions.newDescription")}</p>
        </div>
      </div>

      {actionData?.error && <div className="p-4 bg-destructive/10 text-destructive rounded-md">{actionData.error}</div>}

      <Form method="post" className="space-y-6">
        <Card>
          <CardHeader>
            <CardTitle>{t("serviceActions.basicInformation")}</CardTitle>
            <CardDescription>{t("serviceActions.basicInformationDescription")}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="name">{t("serviceActions.name")} *</Label>
              <Input id="name" name="name" placeholder={t("serviceActions.namePlaceholder")} required />
            </div>

            <div className="space-y-2">
              <Label htmlFor="description">{t("serviceActions.descriptionLabel")}</Label>
              <Input id="description" name="description" placeholder={t("serviceActions.descriptionPlaceholder")} />
            </div>

            <div className="space-y-2">
              <Label htmlFor="trigger_id">{t("serviceActions.trigger")} *</Label>
              <Select name="trigger_id" required>
                <SelectTrigger>
                  <SelectValue placeholder={t("serviceActions.selectTrigger")} />
                </SelectTrigger>
                <SelectContent>
                  {triggers.map((trigger) => (
                    <SelectItem key={trigger} value={trigger}>
                      {getActionTriggerLabel(effectiveLocale, trigger)}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="execution_order">{t("serviceActions.executionOrder")}</Label>
                <Input id="execution_order" name="execution_order" type="number" defaultValue="0" min="0" />
                <p className="text-sm text-muted-foreground">{t("serviceActions.executionOrderHint")}</p>
              </div>

              <div className="space-y-2">
                <Label htmlFor="timeout_ms">{t("serviceActions.timeout")}</Label>
                <Input id="timeout_ms" name="timeout_ms" type="number" defaultValue="3000" min="100" max="30000" />
                <p className="text-sm text-muted-foreground">{t("serviceActions.timeoutHint")}</p>
              </div>
            </div>

            <div className="flex items-center space-x-2">
              <Switch id="enabled" name="enabled" defaultChecked />
              <Label htmlFor="enabled">{t("serviceActions.enabled")}</Label>
            </div>

            <div className="flex items-center space-x-2">
              <Switch id="strict_mode" name="strict_mode" />
              <Label htmlFor="strict_mode">{t("serviceActions.strictMode")}</Label>
              <span className="text-sm text-muted-foreground">{t("serviceActions.strictModeHint")}</span>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{t("serviceActions.script")}</CardTitle>
            <CardDescription>{t("serviceActions.scriptDescription")}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label>{t("serviceActions.scriptTemplates")}</Label>
              <Select value={selectedTemplate} onValueChange={handleTemplateSelect}>
                <SelectTrigger>
                  <SelectValue placeholder={t("serviceActions.chooseTemplate")} />
                </SelectTrigger>
                <SelectContent>
                  {Object.entries(templates).map(([id, template]) => (
                    <SelectItem key={id} value={id}>
                      {template.name} - {template.description}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <Label htmlFor="script">{t("serviceActions.scriptCode")} *</Label>
              <Textarea
                id="script"
                name="script"
                value={script}
                onChange={(e) => setScript(e.target.value)}
                className="font-mono text-sm min-h-[400px]"
                placeholder={t("serviceActions.scriptPlaceholder")}
                required
              />
              <p className="text-sm text-muted-foreground">{t("serviceActions.scriptHelp")}</p>
            </div>

            <div className="p-4 bg-muted rounded-md space-y-2">
              <div className="font-semibold text-sm">{t("serviceActions.contextStructure")}</div>
              <pre className="text-xs overflow-x-auto">{getActionContextReference()}</pre>
            </div>
          </CardContent>
        </Card>

        <div className="flex gap-2">
          <Button type="submit" disabled={isSubmitting}>{isSubmitting ? t("serviceActions.creating") : t("serviceActions.createAction")}</Button>
          <Button type="button" variant="outline" asChild>
            <Link to={`/dashboard/services/${serviceId}/actions`}>{t("common.buttons.cancel")}</Link>
          </Button>
        </div>
      </Form>
    </div>
  );
}
