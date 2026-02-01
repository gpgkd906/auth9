import type { MetaFunction, LoaderFunctionArgs, ActionFunctionArgs } from "react-router";
import { redirect, Form, Link, useActionData, useLoaderData, useNavigation } from "react-router";
import { useState, useEffect } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
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
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "~/components/ui/dialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "~/components/ui/tabs";
import { ArrowLeftIcon, ResetIcon, EyeOpenIcon, PaperPlaneIcon } from "@radix-ui/react-icons";
import {
  emailTemplateApi,
  type EmailTemplateType,
  type EmailTemplateContent,
  type RenderedEmailPreview,
} from "~/services/api";

export const meta: MetaFunction<typeof loader> = ({ data }) => {
  const name = data?.template?.metadata?.name || "Template";
  return [{ title: `Edit ${name} - Email Templates - Auth9` }];
};

export async function loader({ params }: LoaderFunctionArgs) {
  const templateType = params.type as EmailTemplateType;

  if (!templateType) {
    throw new Response("Template type required", { status: 400 });
  }

  try {
    const result = await emailTemplateApi.get(templateType);
    return { template: result.data, error: null };
  } catch (error) {
    const message = error instanceof Error ? error.message : "Failed to load template";
    throw new Response(message, { status: 404 });
  }
}

export async function action({ request, params }: ActionFunctionArgs) {
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

      await emailTemplateApi.update(templateType, content);
      return { success: true, message: "Template saved successfully" };
    }

    if (intent === "reset") {
      await emailTemplateApi.reset(templateType);
      return redirect(`/dashboard/settings/email-templates/${templateType}`);
    }

    if (intent === "preview") {
      const content: EmailTemplateContent = {
        subject: formData.get("subject") as string,
        html_body: formData.get("html_body") as string,
        text_body: formData.get("text_body") as string,
      };

      const result = await emailTemplateApi.preview(templateType, content);
      return { preview: result.data };
    }

    if (intent === "sendTest") {
      const toEmail = formData.get("to_email") as string;
      const subject = formData.get("subject") as string;
      const htmlBody = formData.get("html_body") as string;
      const textBody = formData.get("text_body") as string;
      const variablesJson = formData.get("variables") as string;

      const variables: Record<string, string> = variablesJson
        ? JSON.parse(variablesJson)
        : {};

      const result = await emailTemplateApi.sendTestEmail(templateType, {
        to_email: toEmail,
        subject,
        html_body: htmlBody,
        text_body: textBody,
        variables,
      });

      if (result.success) {
        return {
          testEmailSuccess: true,
          testEmailMessage: result.message,
        };
      } else {
        return {
          testEmailSuccess: false,
          testEmailError: result.message,
        };
      }
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown error";
    return Response.json({ error: message }, { status: 400 });
  }

  return Response.json({ error: "Invalid intent" }, { status: 400 });
}

export default function EmailTemplateEditorPage() {
  const { template } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();

  const [subject, setSubject] = useState(template.content.subject);
  const [htmlBody, setHtmlBody] = useState(template.content.html_body);
  const [textBody, setTextBody] = useState(template.content.text_body);
  const [preview, setPreview] = useState<RenderedEmailPreview | null>(null);
  const [previewTab, setPreviewTab] = useState<"html" | "text">("html");

  // Send test email dialog state
  const [sendTestDialogOpen, setSendTestDialogOpen] = useState(false);
  const [testEmailRecipient, setTestEmailRecipient] = useState("");
  const [testVariables, setTestVariables] = useState<Record<string, string>>(() => {
    // Initialize with example values from template variables
    const initial: Record<string, string> = {};
    for (const variable of template.metadata.variables) {
      initial[variable.name] = variable.example;
    }
    return initial;
  });

  const isSubmitting = navigation.state === "submitting";
  const currentIntent = navigation.formData?.get("intent");

  // Update preview from action data
  useEffect(() => {
    if (actionData && "preview" in actionData && actionData.preview) {
      setPreview(actionData.preview as RenderedEmailPreview);
    }
  }, [actionData]);

  // Close send test dialog on success and reset form
  useEffect(() => {
    if (actionData && "testEmailSuccess" in actionData && actionData.testEmailSuccess) {
      setSendTestDialogOpen(false);
    }
  }, [actionData]);

  // Reset form when template changes (e.g., after reset)
  useEffect(() => {
    setSubject(template.content.subject);
    setHtmlBody(template.content.html_body);
    setTextBody(template.content.text_body);
    // Reset test variables to new example values
    const initial: Record<string, string> = {};
    for (const variable of template.metadata.variables) {
      initial[variable.name] = variable.example;
    }
    setTestVariables(initial);
  }, [template.content, template.metadata.variables]);

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Button asChild variant="ghost" size="sm">
            <Link to="/dashboard/settings/email-templates">
              <ArrowLeftIcon className="h-4 w-4 mr-1" />
              Back
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
                <ResetIcon className="h-4 w-4 mr-1" />
                Reset to Default
              </Button>
            </AlertDialogTrigger>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>Reset Template?</AlertDialogTitle>
                <AlertDialogDescription>
                  This will restore the default template content. Your customizations will be lost.
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>Cancel</AlertDialogCancel>
                <Form method="post">
                  <input type="hidden" name="intent" value="reset" />
                  <AlertDialogAction type="submit">Reset Template</AlertDialogAction>
                </Form>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>
        )}
      </div>

      {/* Success/Error Messages */}
      {actionData && "success" in actionData && actionData.success && (
        <div className="rounded-xl bg-[var(--accent-green)]/10 border border-[var(--accent-green)]/20 p-4 text-sm text-[var(--accent-green)]">
          {actionData.message}
        </div>
      )}
      {actionData && "testEmailSuccess" in actionData && actionData.testEmailSuccess && "testEmailMessage" in actionData && (
        <div className="rounded-xl bg-[var(--accent-green)]/10 border border-[var(--accent-green)]/20 p-4 text-sm text-[var(--accent-green)]">
          {actionData.testEmailMessage}
        </div>
      )}
      {actionData && "testEmailSuccess" in actionData && !actionData.testEmailSuccess && "testEmailError" in actionData && (
        <div className="rounded-xl bg-red-50 border border-red-200 p-4 text-sm text-red-700">
          {actionData.testEmailError}
        </div>
      )}
      {actionData && "error" in actionData && (
        <div className="rounded-xl bg-red-50 border border-red-200 p-4 text-sm text-red-700">
          {String(actionData.error)}
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Editor */}
        <div className="lg:col-span-2 space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>Template Content</CardTitle>
              <CardDescription>
                Edit the subject line and body content. Use {"{{variable_name}}"} syntax for dynamic values.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <Form method="post" className="space-y-6">
                <input type="hidden" name="intent" value="save" />

                <div className="space-y-2">
                  <Label htmlFor="subject">Subject Line</Label>
                  <Input
                    id="subject"
                    name="subject"
                    value={subject}
                    onChange={(e) => setSubject(e.target.value)}
                    placeholder="Enter email subject..."
                  />
                </div>

                <div className="space-y-2">
                  <Label htmlFor="html_body">HTML Body</Label>
                  <Textarea
                    id="html_body"
                    name="html_body"
                    value={htmlBody}
                    onChange={(e) => setHtmlBody(e.target.value)}
                    placeholder="Enter HTML content..."
                    className="font-mono text-sm min-h-[300px]"
                  />
                </div>

                <div className="space-y-2">
                  <Label htmlFor="text_body">Plain Text Body</Label>
                  <Textarea
                    id="text_body"
                    name="text_body"
                    value={textBody}
                    onChange={(e) => setTextBody(e.target.value)}
                    placeholder="Enter plain text content..."
                    className="font-mono text-sm min-h-[150px]"
                  />
                  <p className="text-xs text-[var(--text-secondary)]">
                    Shown to recipients whose email clients don&apos;t support HTML
                  </p>
                </div>

                <div className="flex items-center gap-3 pt-4 border-t">
                  <Button type="submit" disabled={isSubmitting && currentIntent === "save"}>
                    {isSubmitting && currentIntent === "save" ? "Saving..." : "Save Template"}
                  </Button>
                  <Button
                    type="submit"
                    name="intent"
                    value="preview"
                    variant="outline"
                    disabled={isSubmitting}
                    onClick={(e) => {
                      // Prevent default form submission, we need to change intent
                      e.preventDefault();
                      const form = e.currentTarget.closest("form");
                      if (form) {
                        const intentInput = form.querySelector('input[name="intent"]') as HTMLInputElement;
                        if (intentInput) {
                          intentInput.value = "preview";
                        }
                        form.requestSubmit();
                        // Reset intent for next submission
                        setTimeout(() => {
                          if (intentInput) {
                            intentInput.value = "save";
                          }
                        }, 100);
                      }
                    }}
                  >
                    <EyeOpenIcon className="h-4 w-4 mr-1" />
                    {isSubmitting && currentIntent === "preview" ? "Loading..." : "Preview"}
                  </Button>
                  <Dialog open={sendTestDialogOpen} onOpenChange={setSendTestDialogOpen}>
                    <DialogTrigger asChild>
                      <Button type="button" variant="outline">
                        <PaperPlaneIcon className="h-4 w-4 mr-1" />
                        Send Test Email
                      </Button>
                    </DialogTrigger>
                    <DialogContent className="sm:max-w-[500px]">
                      <DialogHeader>
                        <DialogTitle>Send Test Email</DialogTitle>
                        <DialogDescription>
                          Send a test email using the current template content with custom variable values.
                        </DialogDescription>
                      </DialogHeader>
                      <Form method="post" className="space-y-4">
                        <input type="hidden" name="intent" value="sendTest" />
                        <input type="hidden" name="subject" value={subject} />
                        <input type="hidden" name="html_body" value={htmlBody} />
                        <input type="hidden" name="text_body" value={textBody} />
                        <input type="hidden" name="variables" value={JSON.stringify(testVariables)} />

                        <div className="space-y-2">
                          <Label htmlFor="to_email">Recipient Email</Label>
                          <Input
                            id="to_email"
                            name="to_email"
                            type="email"
                            value={testEmailRecipient}
                            onChange={(e) => setTestEmailRecipient(e.target.value)}
                            placeholder="recipient@example.com"
                            required
                          />
                        </div>

                        {template.metadata.variables.length > 0 && (
                          <div className="space-y-3">
                            <Label className="text-sm font-medium">Template Variables</Label>
                            <div className="space-y-2 max-h-[200px] overflow-y-auto">
                              {template.metadata.variables.map((variable) => (
                                <div key={variable.name} className="space-y-1">
                                  <Label htmlFor={`var_${variable.name}`} className="text-xs text-[var(--text-secondary)]">
                                    {variable.name}
                                  </Label>
                                  <Input
                                    id={`var_${variable.name}`}
                                    value={testVariables[variable.name] || ""}
                                    onChange={(e) =>
                                      setTestVariables((prev) => ({
                                        ...prev,
                                        [variable.name]: e.target.value,
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
                          <Button
                            type="button"
                            variant="outline"
                            onClick={() => setSendTestDialogOpen(false)}
                          >
                            Cancel
                          </Button>
                          <Button
                            type="submit"
                            disabled={isSubmitting && currentIntent === "sendTest"}
                          >
                            <PaperPlaneIcon className="h-4 w-4 mr-1" />
                            {isSubmitting && currentIntent === "sendTest" ? "Sending..." : "Send Test Email"}
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

        {/* Sidebar */}
        <div className="space-y-6">
          {/* Variables */}
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Available Variables</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-3">
                {template.metadata.variables.map((variable) => (
                  <div key={variable.name} className="text-sm">
                    <code className="bg-[var(--sidebar-item-hover)] px-1.5 py-0.5 rounded text-xs font-mono">
                      {`{{${variable.name}}}`}
                    </code>
                    <p className="text-[var(--text-secondary)] mt-0.5 text-xs">{variable.description}</p>
                    <p className="text-[var(--text-tertiary)] text-xs">Example: {variable.example}</p>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>

          {/* Preview */}
          {preview && (
            <Card>
              <CardHeader>
                <CardTitle className="text-base">Preview</CardTitle>
                <CardDescription>
                  Preview with sample data
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <Label className="text-xs text-[var(--text-secondary)]">Subject</Label>
                  <p className="text-sm font-medium">{preview.subject}</p>
                </div>
                <Tabs value={previewTab} onValueChange={(v) => setPreviewTab(v as "html" | "text")}>
                  <TabsList className="grid w-full grid-cols-2">
                    <TabsTrigger value="html">HTML</TabsTrigger>
                    <TabsTrigger value="text">Text</TabsTrigger>
                  </TabsList>
                  <TabsContent value="html" className="mt-2">
                    <div className="border rounded-md overflow-hidden bg-white">
                      <iframe
                        srcDoc={preview.html_body}
                        className="w-full h-[300px]"
                        title="Email preview"
                        sandbox=""
                      />
                    </div>
                  </TabsContent>
                  <TabsContent value="text" className="mt-2">
                    <pre className="text-xs bg-[var(--sidebar-item-hover)] p-3 rounded-md overflow-auto max-h-[300px] whitespace-pre-wrap">
                      {preview.text_body}
                    </pre>
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
