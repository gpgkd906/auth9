import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation, useSubmit } from "react-router";
import { useEffect, useState } from "react";
import { CheckCircledIcon, CrossCircledIcon, ExclamationTriangleIcon } from "@radix-ui/react-icons";
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from "~/components/ui/alert-dialog";
import { Checkbox } from "~/components/ui/checkbox";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "~/components/ui/dialog";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "~/components/ui/select";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { translate } from "~/i18n/translate";
import { resolveLocale } from "~/services/locale.server";
import { getAccessToken } from "~/services/session.server";
import { systemApi, type EmailProviderConfig } from "~/services/api";

function getProviderLabel(locale: string, type: string) {
  switch (type) {
    case "none":
      return translate(locale as never, "settings.emailSettings.none");
    case "smtp":
      return translate(locale as never, "settings.emailSettings.smtp");
    case "ses":
      return translate(locale as never, "settings.emailSettings.ses");
    case "oracle":
      return translate(locale as never, "settings.emailSettings.oracle");
    default:
      return type;
  }
}

function getProviderInfo(locale: string, config: EmailProviderConfig): { name: string; details: string } | null {
  switch (config.type) {
    case "smtp":
      return { name: getProviderLabel(locale, "smtp"), details: `${config.host}:${config.port}` };
    case "ses":
      return { name: getProviderLabel(locale, "ses"), details: `Region: ${config.region}` };
    case "oracle":
      return { name: getProviderLabel(locale, "oracle"), details: config.smtp_endpoint };
    default:
      return null;
  }
}

export const meta: MetaFunction = ({ matches }) => buildMeta(resolveMetaLocale(matches), "settings.emailSettings.metaTitle");

export async function loader({ request }: LoaderFunctionArgs) {
  const accessToken = await getAccessToken(request);
  try {
    const result = await systemApi.getEmailSettings(accessToken || undefined);
    const config = result.data.value as EmailProviderConfig;
    return { config, error: null };
  } catch {
    return { config: { type: "none" } as EmailProviderConfig, error: null };
  }
}

export async function action({ request }: ActionFunctionArgs) {
  const locale = await resolveLocale(request);
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
        return Response.json({ error: translate(locale, "settings.emailSettings.invalidProviderType") }, { status: 400 });
      }

      await systemApi.updateEmailSettings(config, accessToken || undefined);
      return { success: true, message: translate(locale, "settings.emailSettings.saved") };
    }

    if (intent === "test_connection") {
      const result = await systemApi.testEmailConnection(accessToken || undefined);
      if (result.success) {
        return { success: true, message: translate(locale, "settings.emailSettings.connectionTestSuccess") };
      }
      return Response.json({ error: result.message }, { status: 400 });
    }

    if (intent === "send_test") {
      const toEmail = formData.get("test_email") as string;
      if (!toEmail || !toEmail.includes("@")) {
        return Response.json({ error: translate(locale, "settings.emailSettings.invalidEmail") }, { status: 400 });
      }
      const result = await systemApi.sendTestEmail(toEmail, accessToken || undefined);
      if (result.success) {
        return { success: true, message: `Test email sent to ${toEmail}` };
      }
      return Response.json({ error: result.message }, { status: 400 });
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : translate(locale, "common.errors.unknown");
    return Response.json({ error: message }, { status: 400 });
  }

  return Response.json({ error: translate(locale, "settings.emailSettings.invalidIntent") }, { status: 400 });
}

export default function EmailSettingsPage() {
  const { config } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const submit = useSubmit();
  const { t, i18n } = useI18n();

  const [providerType, setProviderType] = useState<string>(config.type);
  const [isTestEmailOpen, setIsTestEmailOpen] = useState(false);
  const [testEmail, setTestEmail] = useState("");
  const [pendingProviderType, setPendingProviderType] = useState<string | null>(null);

  const isSubmitting = navigation.state === "submitting";
  const currentIntent = navigation.formData?.get("intent");
  const currentProviderInfo = getProviderInfo(i18n.language, config);
  const isConfigured = config.type !== "none";

  useEffect(() => {
    setProviderType(config.type);
  }, [config.type]);

  const handleProviderChange = (nextType: string) => {
    if (isConfigured && nextType !== config.type && nextType !== "none") {
      setPendingProviderType(nextType);
    } else {
      setProviderType(nextType);
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
      const isSendTest = actionData.message && String(actionData.message).startsWith("Test email sent");
      if (!isSendTest) {
        setIsTestEmailOpen(false);
        setTestEmail("");
      }
    }
  }, [actionData, isTestEmailOpen]);

  return (
    <div className="space-y-6">
      {actionData && "success" in actionData && actionData.success && (
        <div className="rounded-xl border border-[var(--accent-green)]/20 bg-[var(--accent-green)]/10 p-4 text-sm text-[var(--accent-green)]">{actionData.message}</div>
      )}

      {actionData && "error" in actionData && (
        <div className="rounded-xl border border-red-200 bg-red-50 p-4 text-sm text-red-700">{String(actionData.error)}</div>
      )}

      <Card className={isConfigured ? "border-[var(--accent-green)]/20 bg-[var(--accent-green)]/5" : "border-yellow-200 bg-yellow-50/50"}>
        <CardContent className="pt-6">
          <div className="flex items-center gap-3">
            {isConfigured ? <CheckCircledIcon className="h-5 w-5 text-[var(--accent-green)]" /> : <CrossCircledIcon className="h-5 w-5 text-yellow-600" />}
            <div>
              <p className="text-sm font-medium text-[var(--text-primary)]">{isConfigured ? t("settings.emailSettings.statusConfigured") : t("settings.emailSettings.statusNotConfigured")}</p>
              {currentProviderInfo ? (
                <p className="text-sm text-[var(--text-secondary)]">{t("settings.emailSettings.statusUsing", { name: currentProviderInfo.name, details: currentProviderInfo.details })}</p>
              ) : (
                <p className="text-sm text-[var(--text-secondary)]">{t("settings.emailSettings.statusDisabled")}</p>
              )}
            </div>
          </div>
        </CardContent>
      </Card>

      <div className="flex items-start gap-3 rounded-xl border border-blue-200 bg-blue-50 p-4 text-sm text-blue-700">
        <ExclamationTriangleIcon className="mt-0.5 h-5 w-5 flex-shrink-0" />
        <div>
          <p className="font-medium">{t("settings.emailSettings.infoTitle")}</p>
          <p className="mt-1">{t("settings.emailSettings.infoDescription")}</p>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>{t("settings.emailSettings.title")}</CardTitle>
          <CardDescription>{t("settings.emailSettings.description")}</CardDescription>
        </CardHeader>
        <CardContent>
          <Form method="post" className="space-y-6">
            <input type="hidden" name="intent" value="save" />

            <div className="space-y-2">
              <Label htmlFor="provider_type">{t("settings.emailSettings.providerType")}</Label>
              <Select name="provider_type" value={providerType} onValueChange={handleProviderChange}>
                <SelectTrigger className="w-full max-w-xs">
                  <SelectValue placeholder={t("settings.emailSettings.selectProvider")} />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">{t("settings.emailSettings.none")}</SelectItem>
                  <SelectItem value="smtp">{t("settings.emailSettings.smtp")}</SelectItem>
                  <SelectItem value="ses">{t("settings.emailSettings.ses")}</SelectItem>
                  <SelectItem value="oracle">{t("settings.emailSettings.oracle")}</SelectItem>
                </SelectContent>
              </Select>
              {providerType !== config.type && providerType !== "none" && config.type !== "none" && (
                <p className="mt-1 text-xs text-amber-600">{t("settings.emailSettings.replacementWarning", { provider: getProviderLabel(i18n.language, config.type) })}</p>
              )}
            </div>

            {providerType === "smtp" && (
              <div className="space-y-4 border-t pt-4">
                <h3 className="text-sm font-medium text-[var(--text-primary)]">{t("settings.emailSettings.smtpConfiguration")}</h3>
                <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
                  <div className="space-y-2"><Label htmlFor="host">{t("settings.emailSettings.serverHost")}</Label><Input id="host" name="host" placeholder={t("settings.emailSettings.serverHostPlaceholder")} defaultValue={config.type === "smtp" ? config.host : ""} required /></div>
                  <div className="space-y-2"><Label htmlFor="port">{t("settings.emailSettings.port")}</Label><Input id="port" name="port" type="number" placeholder={t("settings.emailSettings.portPlaceholder")} defaultValue={config.type === "smtp" ? config.port : 587} required /></div>
                  <div className="space-y-2"><Label htmlFor="username">{t("settings.emailSettings.username")}</Label><Input id="username" name="username" placeholder={t("settings.emailSettings.usernamePlaceholder")} defaultValue={config.type === "smtp" ? config.username || "" : ""} /></div>
                  <div className="space-y-2"><Label htmlFor="password">{t("settings.emailSettings.password")}</Label><Input id="password" name="password" type="password" placeholder={config.type === "smtp" && config.password ? "***" : t("settings.emailSettings.passwordPlaceholder")} defaultValue="" />{config.type === "smtp" && config.password && <p className="text-xs text-[var(--text-secondary)]">{t("settings.emailSettings.leavePasswordBlank")}</p>}</div>
                  <div className="space-y-2"><Label htmlFor="from_email">{t("settings.emailSettings.fromEmail")}</Label><Input id="from_email" name="from_email" type="email" placeholder={t("settings.emailSettings.fromEmailPlaceholder")} defaultValue={config.type === "smtp" ? config.from_email : ""} required /></div>
                  <div className="space-y-2"><Label htmlFor="from_name">{t("settings.emailSettings.fromName")}</Label><Input id="from_name" name="from_name" placeholder={t("settings.emailSettings.fromNamePlaceholder")} defaultValue={config.type === "smtp" ? config.from_name || "" : ""} /></div>
                </div>
                <div className="flex items-center space-x-2"><Checkbox id="use_tls" name="use_tls" defaultChecked={config.type === "smtp" ? config.use_tls : true} /><Label htmlFor="use_tls" className="cursor-pointer font-normal">{t("settings.emailSettings.useTls")}</Label></div>
              </div>
            )}

            {providerType === "ses" && (
              <div className="space-y-4 border-t pt-4">
                <h3 className="text-sm font-medium text-[var(--text-primary)]">{t("settings.emailSettings.sesConfiguration")}</h3>
                <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
                  <div className="space-y-2"><Label htmlFor="region">{t("settings.emailSettings.awsRegion")}</Label><Input id="region" name="region" placeholder={t("settings.emailSettings.awsRegionPlaceholder")} defaultValue={config.type === "ses" ? config.region : ""} required /></div>
                  <div className="space-y-2"><Label htmlFor="access_key_id">{t("settings.emailSettings.accessKeyId")}</Label><Input id="access_key_id" name="access_key_id" placeholder={t("settings.emailSettings.accessKeyIdPlaceholder")} defaultValue={config.type === "ses" ? config.access_key_id || "" : ""} /><p className="text-xs text-[var(--text-secondary)]">{t("settings.emailSettings.optionalIfIam")}</p></div>
                  <div className="space-y-2"><Label htmlFor="secret_access_key">{t("settings.emailSettings.secretAccessKey")}</Label><Input id="secret_access_key" name="secret_access_key" type="password" placeholder={config.type === "ses" && config.secret_access_key ? "***" : t("settings.emailSettings.secretAccessKeyPlaceholder")} defaultValue="" /></div>
                  <div className="space-y-2"><Label htmlFor="configuration_set">{t("settings.emailSettings.configurationSet")}</Label><Input id="configuration_set" name="configuration_set" placeholder={t("settings.emailSettings.configurationSetPlaceholder")} defaultValue={config.type === "ses" ? config.configuration_set || "" : ""} /></div>
                  <div className="space-y-2"><Label htmlFor="from_email">{t("settings.emailSettings.fromEmail")}</Label><Input id="from_email" name="from_email" type="email" placeholder={t("settings.emailSettings.fromEmailPlaceholder")} defaultValue={config.type === "ses" ? config.from_email : ""} required /></div>
                  <div className="space-y-2"><Label htmlFor="from_name">{t("settings.emailSettings.fromName")}</Label><Input id="from_name" name="from_name" placeholder={t("settings.emailSettings.fromNamePlaceholder")} defaultValue={config.type === "ses" ? config.from_name || "" : ""} /></div>
                </div>
              </div>
            )}

            {providerType === "oracle" && (
              <div className="space-y-4 border-t pt-4">
                <h3 className="text-sm font-medium text-[var(--text-primary)]">{t("settings.emailSettings.oracleConfiguration")}</h3>
                <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
                  <div className="space-y-2"><Label htmlFor="smtp_endpoint">{t("settings.emailSettings.smtpEndpoint")}</Label><Input id="smtp_endpoint" name="smtp_endpoint" placeholder={t("settings.emailSettings.smtpEndpointPlaceholder")} defaultValue={config.type === "oracle" ? config.smtp_endpoint : ""} required /></div>
                  <div className="space-y-2"><Label htmlFor="port">{t("settings.emailSettings.port")}</Label><Input id="port" name="port" type="number" placeholder={t("settings.emailSettings.portPlaceholder")} defaultValue={config.type === "oracle" ? config.port : 587} required /></div>
                  <div className="space-y-2"><Label htmlFor="username">{t("settings.emailSettings.smtpUsername")}</Label><Input id="username" name="username" placeholder={t("settings.emailSettings.smtpUsernamePlaceholder")} defaultValue={config.type === "oracle" ? config.username : ""} required /></div>
                  <div className="space-y-2"><Label htmlFor="password">{t("settings.emailSettings.smtpPassword")}</Label><Input id="password" name="password" type="password" placeholder={config.type === "oracle" && config.password ? "***" : t("settings.emailSettings.passwordPlaceholder")} defaultValue="" required={config.type !== "oracle"} />{config.type === "oracle" && config.password && <p className="text-xs text-[var(--text-secondary)]">{t("settings.emailSettings.leavePasswordBlank")}</p>}</div>
                  <div className="space-y-2"><Label htmlFor="from_email">{t("settings.emailSettings.fromEmail")}</Label><Input id="from_email" name="from_email" type="email" placeholder={t("settings.emailSettings.fromEmailPlaceholder")} defaultValue={config.type === "oracle" ? config.from_email : ""} required /></div>
                  <div className="space-y-2"><Label htmlFor="from_name">{t("settings.emailSettings.fromName")}</Label><Input id="from_name" name="from_name" placeholder={t("settings.emailSettings.fromNamePlaceholder")} defaultValue={config.type === "oracle" ? config.from_name || "" : ""} /></div>
                </div>
              </div>
            )}

            <div className="flex flex-wrap items-center gap-3 border-t pt-4">
              <Button type="submit" disabled={isSubmitting && currentIntent === "save"}>{isSubmitting && currentIntent === "save" ? t("settings.emailSettings.saving") : t("settings.emailSettings.saveSettings")}</Button>
              {providerType !== "none" && (
                <>
                  <Button type="button" variant="outline" onClick={handleTestConnection} disabled={isSubmitting}>{isSubmitting && currentIntent === "test_connection" ? t("settings.emailSettings.testing") : t("settings.emailSettings.testConnection")}</Button>
                  <Button type="button" variant="outline" onClick={() => setIsTestEmailOpen(true)} disabled={isSubmitting}>{t("settings.emailSettings.sendTestEmail")}</Button>
                </>
              )}
            </div>
          </Form>
        </CardContent>
      </Card>

      <Dialog open={isTestEmailOpen} onOpenChange={setIsTestEmailOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("settings.emailSettings.dialogTitle")}</DialogTitle>
            <DialogDescription>{t("settings.emailSettings.dialogDescription")}</DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            {actionData && "success" in actionData && actionData.success && String(actionData.message).startsWith("Test email sent") && (
              <div className="rounded-lg border border-[var(--accent-green)]/20 bg-[var(--accent-green)]/10 p-3 text-sm text-[var(--accent-green)]">{actionData.message}</div>
            )}
            {actionData && "error" in actionData && isTestEmailOpen && (
              <div className="rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-700">{String(actionData.error)}</div>
            )}
            <div className="space-y-2">
              <Label htmlFor="test_email_input">{t("settings.emailSettings.emailAddress")}</Label>
              <Input id="test_email_input" type="email" placeholder={t("settings.emailSettings.testEmailPlaceholder")} value={testEmail} onChange={(event) => setTestEmail(event.target.value)} />
            </div>
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => setIsTestEmailOpen(false)}>{t("common.buttons.cancel")}</Button>
            <Button type="button" onClick={handleSendTestEmail} disabled={!testEmail.includes("@") || isSubmitting}>{t("settings.emailSettings.sendTestEmail")}</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <AlertDialog open={pendingProviderType !== null} onOpenChange={(open) => !open && cancelProviderSwitch()}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("settings.emailSettings.switchProviderTitle")}</AlertDialogTitle>
            <AlertDialogDescription>
              {t("settings.emailSettings.switchProviderDescription", {
                current: getProviderLabel(i18n.language, config.type),
                next: pendingProviderType ? getProviderLabel(i18n.language, pendingProviderType) : "",
              })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel onClick={cancelProviderSwitch}>{t("common.buttons.cancel")}</AlertDialogCancel>
            <AlertDialogAction onClick={confirmProviderSwitch}>{t("settings.emailSettings.switchProvider")}</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
