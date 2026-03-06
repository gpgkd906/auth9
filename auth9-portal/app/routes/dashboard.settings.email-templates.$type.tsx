import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { Form, Link, redirect, useActionData, useFetcher, useLoaderData, useNavigation } from "react-router";
import { useEffect, useState } from "react";
import { ArrowLeftIcon, EyeOpenIcon, PaperPlaneIcon, ResetIcon } from "@radix-ui/react-icons";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Textarea } from "~/components/ui/textarea";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "~/components/ui/alert-dialog";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "~/components/ui/dialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "~/components/ui/tabs";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { translate } from "~/i18n/translate";
import { resolveLocale } from "~/services/locale.server";
import {
  emailTemplateApi,
  type EmailTemplateContent,
  type EmailTemplateType,
  type RenderedEmailPreview,
} from "~/services/api";
import { getAccessToken } from "~/services/session.server";

export const meta: MetaFunction<typeof loader> = ({ data, matches }) => {
  const locale = resolveMetaLocale(matches);
  const name = data?.template?.metadata?.name || translate(locale, "settings.emailTemplateEditor.fallbackName");
  return buildMeta(locale, "settings.emailTemplateEditor.metaTitle", undefined, { name });
};

export async function loader({ params, request }: LoaderFunctionArgs) {
  const locale = await resolveLocale(request);
  const accessToken = await getAccessToken(request);
  const templateType = params.type as EmailTemplateType;

  if (!templateType) {
    throw new Response(translate(locale, "settings.emailTemplateEditor.typeRequired"), { status: 400 });
  }

  try {
    const result = await emailTemplateApi.get(templateType, accessToken || undefined);
    return { template: result.data, error: null };
  } catch (error) {
    const message = error instanceof Error ? error.message : translate(locale, "settings.emailTemplateEditor.loadFailed");
    throw new Response(message, { status: 404 });
  }
}

export async function action({ request, params }: ActionFunctionArgs) {
  const locale = await resolveLocale(request);
  const accessToken = await getAccessToken(request);
  const templateType = params.type as EmailTemplateType;
  const formData = await request.formData();
  const intent = formData.get("intent");

  try {
    if (intent === "save") {
      const content: EmailTemplateContent = {
        subject: formData.get("subject") as string,
        html_body: formData.get("html_body") as string,
        text_body: formData.get("text_body") as string,
      };

      await emailTemplateApi.update(templateType, content, accessToken || undefined);
      return { success: true, message: translate(locale, "settings.emailTemplateEditor.saved") };
    }

    if (intent === "reset") {
      await emailTemplateApi.reset(templateType, accessToken || undefined);
      return redirect(`/dashboard/settings/email-templates/${templateType}`);
    }

    if (intent === "preview") {
      const content: EmailTemplateContent = {
        subject: formData.get("subject") as string,
        html_body: formData.get("html_body") as string,
        text_body: formData.get("text_body") as string,
      };

      const result = await emailTemplateApi.preview(templateType, content, accessToken || undefined);
      return { preview: result.data };
    }

    if (intent === "sendTest") {
      const variablesJson = formData.get("variables") as string;
      const variables: Record<string, string> = variablesJson ? JSON.parse(variablesJson) : {};

      const result = await emailTemplateApi.sendTestEmail(
        templateType,
        {
          to_email: formData.get("to_email") as string,
          subject: formData.get("subject") as string,
          html_body: formData.get("html_body") as string,
          text_body: formData.get("text_body") as string,
          variables,
        },
        accessToken || undefined
      );

      return result.success
        ? { testEmailSuccess: true, testEmailMessage: result.message }
        : { testEmailSuccess: false, testEmailError: result.message };
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : translate(locale, "common.errors.unknown");
    return { error: message };
  }

  return { error: translate(locale, "settings.emailTemplateEditor.invalidIntent") };
}

export default function EmailTemplateEditorPage() {
  const { template } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const resetFetcher = useFetcher();
  const { t } = useI18n();

  const [subject, setSubject] = useState(template.content.subject);
  const [htmlBody, setHtmlBody] = useState(template.content.html_body);
  const [textBody, setTextBody] = useState(template.content.text_body);
  const [preview, setPreview] = useState<RenderedEmailPreview | null>(null);
  const [previewTab, setPreviewTab] = useState<"html" | "text">("html");
  const [sendTestDialogOpen, setSendTestDialogOpen] = useState(false);
  const [testEmailRecipient, setTestEmailRecipient] = useState("");
  const [testVariables, setTestVariables] = useState<Record<string, string>>(() => {
    const initial: Record<string, string> = {};
    for (const variable of template.metadata.variables) {
      initial[variable.name] = variable.example;
    }
    return initial;
  });

  const isSubmitting = navigation.state === "submitting";
  const currentIntent = navigation.formData?.get("intent");

  useEffect(() => {
    if (actionData && "preview" in actionData && actionData.preview) {
      setPreview(actionData.preview as RenderedEmailPreview);
    }
  }, [actionData]);

  useEffect(() => {
    if (actionData && "testEmailSuccess" in actionData && actionData.testEmailSuccess) {
      setSendTestDialogOpen(false);
    }
  }, [actionData]);

  useEffect(() => {
    setSubject(template.content.subject);
    setHtmlBody(template.content.html_body);
    setTextBody(template.content.text_body);
    const initial: Record<string, string> = {};
    for (const variable of template.metadata.variables) {
      initial[variable.name] = variable.example;
    }
    setTestVariables(initial);
  }, [template.content, template.metadata.variables]);

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Button asChild variant="ghost" size="sm">
            <Link to="/dashboard/settings/email-templates">
              <ArrowLeftIcon className="mr-1 h-4 w-4" />
              {t("settings.emailTemplateEditor.back")}
            </Link>
          </Button>
          <div>
            <h2 className="text-lg font-semibold text-[var(--text-primary)]">{template.metadata.name}</h2>
            <p className="text-sm text-[var(--text-secondary)]">{template.metadata.description}</p>
          </div>
        </div>
        {template.is_customized && (
          <AlertDialog>
            <AlertDialogTrigger asChild>
              <Button variant="outline" size="sm">
                <ResetIcon className="mr-1 h-4 w-4" />
                {t("settings.emailTemplateEditor.resetToDefault")}
              </Button>
            </AlertDialogTrigger>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>{t("settings.emailTemplateEditor.resetTitle")}</AlertDialogTitle>
                <AlertDialogDescription>{t("settings.emailTemplateEditor.resetDescription")}</AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>{t("common.buttons.cancel")}</AlertDialogCancel>
                <AlertDialogAction
                  onClick={() => {
                    resetFetcher.submit({ intent: "reset" }, { method: "post" });
                  }}
                >
                  {t("settings.emailTemplateEditor.resetConfirm")}
                </AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>
        )}
      </div>

      {actionData && "success" in actionData && actionData.success && (
        <div className="rounded-xl border border-[var(--accent-green)]/20 bg-[var(--accent-green)]/10 p-4 text-sm text-[var(--accent-green)]">
          {actionData.message}
        </div>
      )}
      {actionData && "testEmailSuccess" in actionData && actionData.testEmailSuccess && "testEmailMessage" in actionData && (
        <div className="rounded-xl border border-[var(--accent-green)]/20 bg-[var(--accent-green)]/10 p-4 text-sm text-[var(--accent-green)]">
          {actionData.testEmailMessage}
        </div>
      )}
      {actionData && "testEmailSuccess" in actionData && !actionData.testEmailSuccess && "testEmailError" in actionData && (
        <div className="rounded-xl border border-red-200 bg-red-50 p-4 text-sm text-red-700">{actionData.testEmailError}</div>
      )}
      {actionData && "error" in actionData && (
        <div className="rounded-xl border border-red-200 bg-red-50 p-4 text-sm text-red-700">{String(actionData.error)}</div>
      )}

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-3">
        <div className="space-y-6 lg:col-span-2">
          <Card>
            <CardHeader>
              <CardTitle>{t("settings.emailTemplateEditor.templateContent")}</CardTitle>
              <CardDescription>
                {t("settings.emailTemplateEditor.templateContentDescription", { syntax: "{{variable_name}}" })}
              </CardDescription>
            </CardHeader>
            <CardContent>
              <Form method="post" className="space-y-6">
                <input type="hidden" name="intent" value="save" />

                <div className="space-y-2">
                  <Label htmlFor="subject">{t("settings.emailTemplateEditor.subjectLine")}</Label>
                  <Input id="subject" name="subject" value={subject} onChange={(event) => setSubject(event.target.value)} placeholder={t("settings.emailTemplateEditor.subjectPlaceholder")} />
                </div>

                <div className="space-y-2">
                  <Label htmlFor="html_body">{t("settings.emailTemplateEditor.htmlBody")}</Label>
                  <Textarea id="html_body" name="html_body" value={htmlBody} onChange={(event) => setHtmlBody(event.target.value)} placeholder={t("settings.emailTemplateEditor.htmlPlaceholder")} className="min-h-[300px] font-mono text-sm" />
                </div>

                <div className="space-y-2">
                  <Label htmlFor="text_body">{t("settings.emailTemplateEditor.textBody")}</Label>
                  <Textarea id="text_body" name="text_body" value={textBody} onChange={(event) => setTextBody(event.target.value)} placeholder={t("settings.emailTemplateEditor.textPlaceholder")} className="min-h-[150px] font-mono text-sm" />
                  <p className="text-xs text-[var(--text-secondary)]">{t("settings.emailTemplateEditor.textBodyHint")}</p>
                </div>

                <div className="flex items-center gap-3 border-t pt-4">
                  <Button type="submit" disabled={isSubmitting && currentIntent === "save"}>
                    {isSubmitting && currentIntent === "save" ? t("settings.emailTemplateEditor.saving") : t("settings.emailTemplateEditor.saveTemplate")}
                  </Button>
                  <Button
                    type="submit"
                    name="intent"
                    value="preview"
                    variant="outline"
                    disabled={isSubmitting}
                    onClick={(event) => {
                      event.preventDefault();
                      const form = event.currentTarget.closest("form");
                      if (form) {
                        const intentInput = form.querySelector('input[name="intent"]') as HTMLInputElement | null;
                        if (intentInput) {
                          intentInput.value = "preview";
                        }
                        form.requestSubmit();
                        setTimeout(() => {
                          if (intentInput) {
                            intentInput.value = "save";
                          }
                        }, 100);
                      }
                    }}
                  >
                    <EyeOpenIcon className="mr-1 h-4 w-4" />
                    {isSubmitting && currentIntent === "preview" ? t("settings.emailTemplateEditor.loading") : t("settings.emailTemplateEditor.preview")}
                  </Button>
                  <Dialog open={sendTestDialogOpen} onOpenChange={setSendTestDialogOpen}>
                    <DialogTrigger asChild>
                      <Button type="button" variant="outline">
                        <PaperPlaneIcon className="mr-1 h-4 w-4" />
                        {t("settings.emailTemplateEditor.sendTestEmail")}
                      </Button>
                    </DialogTrigger>
                    <DialogContent className="sm:max-w-[500px]">
                      <DialogHeader>
                        <DialogTitle>{t("settings.emailTemplateEditor.sendTestTitle")}</DialogTitle>
                        <DialogDescription>{t("settings.emailTemplateEditor.sendTestDescription")}</DialogDescription>
                      </DialogHeader>
                      <Form method="post" className="space-y-4">
                        <input type="hidden" name="intent" value="sendTest" />
                        <input type="hidden" name="subject" value={subject} />
                        <input type="hidden" name="html_body" value={htmlBody} />
                        <input type="hidden" name="text_body" value={textBody} />
                        <input type="hidden" name="variables" value={JSON.stringify(testVariables)} />

                        <div className="space-y-2">
                          <Label htmlFor="to_email">{t("settings.emailTemplateEditor.recipientEmail")}</Label>
                          <Input
                            id="to_email"
                            name="to_email"
                            type="email"
                            value={testEmailRecipient}
                            onChange={(event) => setTestEmailRecipient(event.target.value)}
                            placeholder={t("settings.emailTemplateEditor.recipientPlaceholder")}
                            required
                          />
                        </div>

                        {template.metadata.variables.length > 0 && (
                          <div className="space-y-3">
                            <Label className="text-sm font-medium">{t("settings.emailTemplateEditor.templateVariables")}</Label>
                            <div className="max-h-[200px] space-y-2 overflow-y-auto">
                              {template.metadata.variables.map((variable) => (
                                <div key={variable.name} className="space-y-1">
                                  <Label htmlFor={`var_${variable.name}`} className="text-xs text-[var(--text-secondary)]">{variable.name}</Label>
                                  <Input
                                    id={`var_${variable.name}`}
                                    value={testVariables[variable.name] || ""}
                                    onChange={(event) =>
                                      setTestVariables((previous) => ({
                                        ...previous,
                                        [variable.name]: event.target.value,
                                      }))
                                    }
                                    placeholder={variable.example}
                                    className="text-sm"
                                  />
                                </div>
                              ))}
                            </div>
                          </div>
                        )}

                        <DialogFooter>
                          <Button type="button" variant="outline" onClick={() => setSendTestDialogOpen(false)}>
                            {t("common.buttons.cancel")}
                          </Button>
                          <Button type="submit" disabled={isSubmitting && currentIntent === "sendTest"}>
                            <PaperPlaneIcon className="mr-1 h-4 w-4" />
                            {isSubmitting && currentIntent === "sendTest" ? t("settings.emailTemplateEditor.sending") : t("settings.emailTemplateEditor.sendTestEmail")}
                          </Button>
                        </DialogFooter>
                      </Form>
                    </DialogContent>
                  </Dialog>
                </div>
              </Form>
            </CardContent>
          </Card>
        </div>

        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="text-base">{t("settings.emailTemplateEditor.availableVariables")}</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-3">
                {template.metadata.variables.map((variable) => (
                  <div key={variable.name} className="text-sm">
                    <code className="rounded bg-[var(--sidebar-item-hover)] px-1.5 py-0.5 text-xs font-mono">{`{{${variable.name}}}`}</code>
                    <p className="mt-0.5 text-xs text-[var(--text-secondary)]">{variable.description}</p>
                    <p className="text-xs text-[var(--text-tertiary)]">{t("settings.emailTemplateEditor.example", { value: variable.example })}</p>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>

          {preview && (
            <Card>
              <CardHeader>
                <CardTitle className="text-base">{t("settings.emailTemplateEditor.previewTitle")}</CardTitle>
                <CardDescription>{t("settings.emailTemplateEditor.previewDescription")}</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <Label className="text-xs text-[var(--text-secondary)]">{t("settings.emailTemplateEditor.subject")}</Label>
                  <p className="text-sm font-medium">{preview.subject}</p>
                </div>
                <Tabs value={previewTab} onValueChange={(value) => setPreviewTab(value as "html" | "text") }>
                  <TabsList className="grid w-full grid-cols-2">
                    <TabsTrigger value="html">{t("settings.emailTemplateEditor.htmlTab")}</TabsTrigger>
                    <TabsTrigger value="text">{t("settings.emailTemplateEditor.textTab")}</TabsTrigger>
                  </TabsList>
                  <TabsContent value="html" className="mt-2">
                    <div className="overflow-hidden rounded-md border bg-white">
                      <iframe srcDoc={preview.html_body} className="h-[300px] w-full" title={t("settings.emailTemplateEditor.previewFrameTitle")} sandbox="" />
                    </div>
                  </TabsContent>
                  <TabsContent value="text" className="mt-2">
                    <pre className="max-h-[300px] overflow-auto whitespace-pre-wrap rounded-md bg-[var(--sidebar-item-hover)] p-3 text-xs">{preview.text_body}</pre>
                  </TabsContent>
                </Tabs>
              </CardContent>
            </Card>
          )}
        </div>
      </div>
    </div>
  );
}
