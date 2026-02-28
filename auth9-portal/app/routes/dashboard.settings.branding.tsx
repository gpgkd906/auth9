import type { MetaFunction, ActionFunctionArgs, LoaderFunctionArgs } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation } from "react-router";
import { useState, useEffect } from "react";
import { CheckCircledIcon, ResetIcon } from "@radix-ui/react-icons";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Textarea } from "~/components/ui/textarea";
import { brandingApi, type BrandingConfig } from "~/services/api";
import { getAccessToken } from "~/services/session.server";

// Default branding values (should match backend defaults)
const DEFAULT_BRANDING: BrandingConfig = {
  primary_color: "#007AFF",
  secondary_color: "#5856D6",
  background_color: "#F5F5F7",
  text_color: "#1D1D1F",
  allow_registration: false,
};

export const meta: MetaFunction = () => {
  return [{ title: "Login Branding - Auth9" }];
};

export async function loader({ request }: LoaderFunctionArgs) {
  const accessToken = await getAccessToken(request);
  try {
    const result = await brandingApi.get(accessToken || undefined);
    return { config: result.data, error: null };
  } catch {
    // If no config exists yet, return defaults
    return { config: DEFAULT_BRANDING, error: null };
  }
}

export async function action({ request }: ActionFunctionArgs) {
  const accessToken = await getAccessToken(request);
  const formData = await request.formData();
  const intent = formData.get("intent");

  try {
    if (intent === "save") {
      const config: BrandingConfig = {
        logo_url: (formData.get("logo_url") as string) || undefined,
        primary_color: formData.get("primary_color") as string,
        secondary_color: formData.get("secondary_color") as string,
        background_color: formData.get("background_color") as string,
        text_color: formData.get("text_color") as string,
        custom_css: (formData.get("custom_css") as string) || undefined,
        company_name: (formData.get("company_name") as string) || undefined,
        favicon_url: (formData.get("favicon_url") as string) || undefined,
        allow_registration: formData.get("allow_registration") === "true",
      };

      await brandingApi.update(config, accessToken || undefined);
      return { success: true, message: "Branding settings saved successfully" };
    }

    if (intent === "reset") {
      await brandingApi.update(DEFAULT_BRANDING, accessToken || undefined);
      return { success: true, message: "Branding reset to defaults", reset: true };
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown error";
    return Response.json({ error: message }, { status: 400 });
  }

  return Response.json({ error: "Invalid intent" }, { status: 400 });
}

// Color picker component
function ColorPicker({
  id,
  label,
  value,
  onChange,
  defaultValue,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
  defaultValue: string;
}) {
  return (
    <div className="space-y-2">
      <Label htmlFor={id}>{label}</Label>
      <div className="flex items-center gap-2">
        <label
          htmlFor={`${id}_picker`}
          className="w-10 h-10 rounded-md border border-gray-300 shadow-sm cursor-pointer block"
          style={{ backgroundColor: value }}
        >
          <span className="sr-only">Choose {label}</span>
        </label>
        <input
          type="color"
          id={`${id}_picker`}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          className="sr-only"
        />
        <Input
          id={id}
          name={id}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={defaultValue}
          className="font-mono uppercase"
          maxLength={7}
        />
      </div>
    </div>
  );
}

export default function BrandingSettingsPage() {
  const { config } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();

  // Local state for form fields
  const [logoUrl, setLogoUrl] = useState(config.logo_url || "");
  const [primaryColor, setPrimaryColor] = useState(config.primary_color);
  const [secondaryColor, setSecondaryColor] = useState(config.secondary_color);
  const [backgroundColor, setBackgroundColor] = useState(config.background_color);
  const [textColor, setTextColor] = useState(config.text_color);
  const [customCss, setCustomCss] = useState(config.custom_css || "");
  const [companyName, setCompanyName] = useState(config.company_name || "");
  const [faviconUrl, setFaviconUrl] = useState(config.favicon_url || "");
  const [allowRegistration, setAllowRegistration] = useState(config.allow_registration ?? false);

  const isSubmitting = navigation.state === "submitting";
  const currentIntent = navigation.formData?.get("intent");

  // Reset form when config changes (after reset action)
  useEffect(() => {
    if (actionData && "reset" in actionData && actionData.reset) {
      setLogoUrl("");
      setPrimaryColor(DEFAULT_BRANDING.primary_color);
      setSecondaryColor(DEFAULT_BRANDING.secondary_color);
      setBackgroundColor(DEFAULT_BRANDING.background_color);
      setTextColor(DEFAULT_BRANDING.text_color);
      setCustomCss("");
      setCompanyName("");
      setFaviconUrl("");
      setAllowRegistration(false);
    }
  }, [actionData]);

  // Check if current config matches defaults
  const isDefault =
    primaryColor === DEFAULT_BRANDING.primary_color &&
    secondaryColor === DEFAULT_BRANDING.secondary_color &&
    backgroundColor === DEFAULT_BRANDING.background_color &&
    textColor === DEFAULT_BRANDING.text_color &&
    !logoUrl &&
    !customCss &&
    !companyName &&
    !faviconUrl &&
    !allowRegistration;

  return (
    <div className="space-y-6">
      {actionData && "success" in actionData && actionData.success && (
        <div className="rounded-xl bg-[var(--accent-green)]/10 border border-[var(--accent-green)]/20 p-4 text-sm text-[var(--accent-green)] flex items-center gap-2">
          <CheckCircledIcon className="h-4 w-4" />
          {actionData.message}
        </div>
      )}

      {actionData && "error" in actionData && (
        <div className="rounded-xl bg-red-50 border border-red-200 p-4 text-sm text-red-700">
          {String(actionData.error)}
        </div>
      )}

      <Card>
        <CardHeader>
          <CardTitle>Login Page Branding</CardTitle>
          <CardDescription>
            Customize the appearance of your login pages. Changes will be applied to all Keycloak login forms.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Form method="post" className="space-y-6">

            {/* Company Identity */}
            <div className="space-y-2">
              <h3 className="text-sm font-medium text-[var(--text-primary)] border-b pb-2">Company Identity</h3>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label htmlFor="company_name">Company Name</Label>
                  <Input
                    id="company_name"
                    name="company_name"
                    placeholder="Your Company Name"
                    value={companyName}
                    onChange={(e) => setCompanyName(e.target.value)}
                    maxLength={100}
                  />
                  <p className="text-xs text-[var(--text-secondary)]">Displayed on the login page</p>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="logo_url">Logo URL</Label>
                  <Input
                    id="logo_url"
                    name="logo_url"
                    type="url"
                    placeholder="https://example.com/logo.png"
                    value={logoUrl}
                    onChange={(e) => setLogoUrl(e.target.value)}
                  />
                  <p className="text-xs text-[var(--text-secondary)]">Recommended size: 200x50 pixels</p>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="favicon_url">Favicon URL</Label>
                  <Input
                    id="favicon_url"
                    name="favicon_url"
                    type="url"
                    placeholder="https://example.com/favicon.ico"
                    value={faviconUrl}
                    onChange={(e) => setFaviconUrl(e.target.value)}
                  />
                  <p className="text-xs text-[var(--text-secondary)]">Browser tab icon (ICO or PNG)</p>
                </div>
              </div>

              {/* Logo Preview */}
              {logoUrl && (
                <div className="mt-4 p-4 bg-[var(--sidebar-item-hover)] rounded-lg min-w-[200px]">
                  <p className="text-sm text-[var(--text-secondary)] mb-2">Logo Preview:</p>
                  <img
                    src={logoUrl}
                    alt="Logo preview"
                    className="max-h-16 object-contain"
                    onError={(e) => {
                      (e.target as HTMLImageElement).style.display = "none";
                    }}
                  />
                </div>
              )}
            </div>

            {/* Login Options */}
            <div className="space-y-2">
              <h3 className="text-sm font-medium text-[var(--text-primary)] border-b pb-2">Login Options</h3>
              <div className="space-y-1">
                <div className="flex items-center justify-between min-h-[48px]">
                  <Label htmlFor="allow_registration">Allow Registration</Label>
                  <label htmlFor="allow_registration" className="relative inline-flex items-center cursor-pointer shrink-0">
                    <span className="sr-only">Toggle allow registration</span>
                    <input
                      type="checkbox"
                      id="allow_registration"
                      name="allow_registration"
                      value="true"
                      checked={allowRegistration}
                      onChange={(e) => setAllowRegistration(e.target.checked)}
                      className="sr-only peer"
                    />
                    <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600"></div>
                  </label>
                </div>
                <p className="text-xs text-[var(--text-secondary)]">
                  Show &quot;Create account&quot; link on the login page
                </p>
              </div>
            </div>

            {/* Colors */}
            <div className="space-y-2">
              <h3 className="text-sm font-medium text-[var(--text-primary)] border-b pb-2">Colors</h3>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <ColorPicker
                  id="primary_color"
                  label="Primary Color"
                  value={primaryColor}
                  onChange={setPrimaryColor}
                  defaultValue={DEFAULT_BRANDING.primary_color}
                />
                <ColorPicker
                  id="secondary_color"
                  label="Secondary Color"
                  value={secondaryColor}
                  onChange={setSecondaryColor}
                  defaultValue={DEFAULT_BRANDING.secondary_color}
                />
                <ColorPicker
                  id="background_color"
                  label="Background Color"
                  value={backgroundColor}
                  onChange={setBackgroundColor}
                  defaultValue={DEFAULT_BRANDING.background_color}
                />
                <ColorPicker
                  id="text_color"
                  label="Text Color"
                  value={textColor}
                  onChange={setTextColor}
                  defaultValue={DEFAULT_BRANDING.text_color}
                />
              </div>
            </div>

            {/* Preview */}
            <div className="space-y-2">
              <h3 className="text-sm font-medium text-[var(--text-primary)] border-b pb-2">Preview</h3>
              <div
                className="rounded-lg p-0 sm:p-8 flex items-center justify-center min-h-[300px]"
                style={{ backgroundColor }}
              >
                <div className="w-full">
                  <div className="w-full max-w-none sm:max-w-sm bg-white rounded-xl shadow-lg p-6">
                    {logoUrl ? (
                      <img src={logoUrl} alt="Logo" className="h-10 mx-auto mb-4 object-contain" />
                    ) : companyName ? (
                      <h2
                        className="text-xl font-semibold text-center mb-4"
                        style={{ color: primaryColor }}
                      >
                        {companyName}
                      </h2>
                    ) : (
                      <div
                        className="h-10 w-32 mx-auto mb-4 rounded"
                        style={{ backgroundColor: primaryColor, opacity: 0.2 }}
                      />
                    )}

                    <div className="space-y-4">
                      <div>
                        <span
                          className="block text-sm font-medium mb-1"
                          style={{ color: textColor }}
                        >
                          Email
                        </span>
                        <div
                          className="w-full h-10 rounded-md border"
                          style={{ borderColor: secondaryColor }}
                        />
                      </div>
                      <div>
                        <span
                          className="block text-sm font-medium mb-1"
                          style={{ color: textColor }}
                        >
                          Password
                        </span>
                        <div
                          className="w-full h-10 rounded-md border"
                          style={{ borderColor: secondaryColor }}
                        />
                      </div>
                      <button
                        type="button"
                        className="w-full h-10 rounded-md text-white font-medium"
                        style={{ backgroundColor: primaryColor }}
                      >
                        Sign In
                      </button>
                      <p className="text-center text-sm" style={{ color: secondaryColor }}>
                        Forgot password?
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* Custom CSS */}
            <div className="space-y-2">
              <h3 className="text-sm font-medium text-[var(--text-primary)] border-b pb-2">
                Custom CSS
                <span className="font-normal text-[var(--text-secondary)] ml-2">(Advanced)</span>
              </h3>
              <div className="space-y-2">
                <Textarea
                  id="custom_css"
                  name="custom_css"
                  placeholder={`.login-form {\n  border-radius: 16px;\n  box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);\n}`}
                  value={customCss}
                  onChange={(e) => setCustomCss(e.target.value)}
                  className="font-mono text-sm min-h-[120px]"
                />
                <p className="text-xs text-[var(--text-secondary)]">
                  Add custom CSS rules to further customize the login page. Maximum 50KB.
                </p>
              </div>
            </div>

            {/* Action Buttons */}
            <div className="flex flex-wrap items-center gap-3 border-t pt-4 md:static sticky bottom-0 bg-[var(--surface-primary)] pb-4 -mb-4 z-10">
              <Button type="submit" name="intent" value="save" disabled={isSubmitting && currentIntent === "save"}>
                {isSubmitting && currentIntent === "save" ? "Saving..." : "Save Changes"}
              </Button>

              <Button
                type="submit"
                name="intent"
                value="reset"
                variant="outline"
                disabled={isSubmitting || isDefault}
              >
                <ResetIcon className="h-4 w-4 mr-2" />
                {isSubmitting && currentIntent === "reset" ? "Resetting..." : "Reset to Defaults"}
              </Button>
            </div>
          </Form>
        </CardContent>
      </Card>
    </div>
  );
}
