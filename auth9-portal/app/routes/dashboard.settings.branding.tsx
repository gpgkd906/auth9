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
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";
import { mapApiError } from "~/lib/error-messages";

const DEFAULT_BRANDING: BrandingConfig = {
  primary_color: "#007AFF",
  secondary_color: "#5856D6",
  background_color: "#F5F5F7",
  text_color: "#1D1D1F",
  allow_registration: false,
};

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "settings.brandingPage.metaTitle");
};

export async function loader({ request }: LoaderFunctionArgs) {
  const accessToken = await getAccessToken(request);
  try {
    const result = await brandingApi.get(accessToken || undefined);
    return { config: result.data, error: null };
  } catch {
    return { config: DEFAULT_BRANDING, error: null };
  }
}

export async function action({ request }: ActionFunctionArgs) {
  const locale = await resolveLocale(request);
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
      return { success: true, message: translate(locale, "settings.brandingPage.successSaved") };
    }

    if (intent === "reset") {
      await brandingApi.update(DEFAULT_BRANDING, accessToken || undefined);
      return { success: true, message: translate(locale, "settings.brandingPage.successReset"), reset: true };
    }
  } catch (error) {
    const message = mapApiError(error, locale);
    return Response.json({ error: message }, { status: 400 });
  }

  return Response.json({ error: translate(locale, "tenants.errors.invalidIntent") }, { status: 400 });
}

function ColorPicker({
  id,
  label,
  value,
  onChange,
  defaultValue,
  srLabel,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
  defaultValue: string;
  srLabel: string;
}) {
  return (
    <div className="space-y-2">
      <Label htmlFor={id}>{label}</Label>
      <div className="flex items-center gap-2">
        <label htmlFor={`${id}_picker`} className="w-10 h-10 rounded-md border border-gray-300 shadow-sm cursor-pointer block" style={{ backgroundColor: value }}>
          <span className="sr-only">{srLabel}</span>
        </label>
        <input type="color" id={`${id}_picker`} value={value} onChange={(e) => onChange(e.target.value)} className="sr-only" />
        <Input id={id} name={id} value={value} onChange={(e) => onChange(e.target.value)} placeholder={defaultValue} className="font-mono uppercase" maxLength={7} />
      </div>
    </div>
  );
}

export default function BrandingSettingsPage() {
  const { config } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const { t } = useI18n();

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
        <div className="rounded-xl bg-red-50 border border-red-200 p-4 text-sm text-red-700">{String(actionData.error)}</div>
      )}

      <Card>
        <CardHeader>
          <CardTitle>{t("settings.brandingPage.title")}</CardTitle>
          <CardDescription>{t("settings.brandingPage.description")}</CardDescription>
        </CardHeader>
        <CardContent>
          <Form method="post" className="space-y-6 pb-24 md:pb-0">
            <div className="space-y-2">
              <h3 className="text-sm font-medium text-[var(--text-primary)] border-b pb-2">{t("settings.brandingPage.companyIdentity")}</h3>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label htmlFor="company_name">{t("settings.brandingPage.companyName")}</Label>
                  <Input id="company_name" name="company_name" placeholder={t("settings.brandingPage.companyNamePlaceholder")} value={companyName} onChange={(e) => setCompanyName(e.target.value)} maxLength={100} />
                  <p className="text-xs text-[var(--text-secondary)]">{t("settings.brandingPage.companyNameHint")}</p>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="logo_url">{t("settings.brandingPage.logoUrl")}</Label>
                  <Input id="logo_url" name="logo_url" type="url" placeholder={t("settings.brandingPage.logoUrlPlaceholder")} value={logoUrl} onChange={(e) => setLogoUrl(e.target.value)} />
                  <p className="text-xs text-[var(--text-secondary)]">{t("settings.brandingPage.logoUrlHint")}</p>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="favicon_url">{t("settings.brandingPage.faviconUrl")}</Label>
                  <Input id="favicon_url" name="favicon_url" type="url" placeholder={t("settings.brandingPage.faviconUrlPlaceholder")} value={faviconUrl} onChange={(e) => setFaviconUrl(e.target.value)} />
                  <p className="text-xs text-[var(--text-secondary)]">{t("settings.brandingPage.faviconUrlHint")}</p>
                </div>
              </div>

              {logoUrl && (
                <div className="mt-4 p-4 bg-[var(--sidebar-item-hover)] rounded-lg min-w-[200px]">
                  <p className="text-sm text-[var(--text-secondary)] mb-2">{t("settings.brandingPage.logoPreview")}</p>
                  <img src={logoUrl} alt={t("settings.brandingPage.logoPreviewAlt")} className="max-h-16 object-contain" onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }} />
                </div>
              )}
            </div>

            <div className="space-y-2">
              <h3 className="text-sm font-medium text-[var(--text-primary)] border-b pb-2">{t("settings.brandingPage.loginOptions")}</h3>
              <div className="space-y-1">
                <div className="flex items-center justify-between min-h-[48px]">
                  <Label htmlFor="allow_registration">{t("settings.brandingPage.allowRegistration")}</Label>
                  <label htmlFor="allow_registration" className="relative inline-flex items-center cursor-pointer shrink-0">
                    <span className="sr-only">{t("settings.brandingPage.toggleAllowRegistration")}</span>
                    <input type="checkbox" id="allow_registration" name="allow_registration" value="true" checked={allowRegistration} onChange={(e) => setAllowRegistration(e.target.checked)} className="sr-only peer" />
                    <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600"></div>
                  </label>
                </div>
                <p className="text-xs text-[var(--text-secondary)]">{t("settings.brandingPage.allowRegistrationHint")}</p>
              </div>
            </div>

            <div className="space-y-2">
              <h3 className="text-sm font-medium text-[var(--text-primary)] border-b pb-2">{t("settings.brandingPage.colors")}</h3>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <ColorPicker id="primary_color" label={t("services.branding.primaryColor")} value={primaryColor} onChange={setPrimaryColor} defaultValue={DEFAULT_BRANDING.primary_color} srLabel={t("services.branding.chooseColor", { label: t("services.branding.primaryColor") })} />
                <ColorPicker id="secondary_color" label={t("services.branding.secondaryColor")} value={secondaryColor} onChange={setSecondaryColor} defaultValue={DEFAULT_BRANDING.secondary_color} srLabel={t("services.branding.chooseColor", { label: t("services.branding.secondaryColor") })} />
                <ColorPicker id="background_color" label={t("services.branding.backgroundColor")} value={backgroundColor} onChange={setBackgroundColor} defaultValue={DEFAULT_BRANDING.background_color} srLabel={t("services.branding.chooseColor", { label: t("services.branding.backgroundColor") })} />
                <ColorPicker id="text_color" label={t("services.branding.textColor")} value={textColor} onChange={setTextColor} defaultValue={DEFAULT_BRANDING.text_color} srLabel={t("services.branding.chooseColor", { label: t("services.branding.textColor") })} />
              </div>
            </div>

            <div className="space-y-2">
              <h3 className="text-sm font-medium text-[var(--text-primary)] border-b pb-2">{t("settings.brandingPage.preview")}</h3>
              <div className="rounded-lg p-0 sm:p-8 flex items-center justify-center min-h-[300px]" style={{ backgroundColor }}>
                <div className="w-full">
                  <div className="w-full max-w-none sm:max-w-sm bg-white rounded-xl shadow-lg p-6">
                    {logoUrl ? (
                      <img src={logoUrl} alt={t("settings.brandingPage.previewLogoAlt")} className="h-10 mx-auto mb-4 object-contain" />
                    ) : companyName ? (
                      <h2 className="text-xl font-semibold text-center mb-4" style={{ color: primaryColor }}>{companyName}</h2>
                    ) : (
                      <div className="h-10 w-32 mx-auto mb-4 rounded" style={{ backgroundColor: primaryColor, opacity: 0.2 }} />
                    )}

                    <div className="space-y-4">
                      <div>
                        <span className="block text-sm font-medium mb-1" style={{ color: textColor }}>{t("settings.brandingPage.previewEmail")}</span>
                        <div className="w-full h-10 rounded-md border" style={{ borderColor: secondaryColor }} />
                      </div>
                      <div>
                        <span className="block text-sm font-medium mb-1" style={{ color: textColor }}>{t("settings.brandingPage.previewPassword")}</span>
                        <div className="w-full h-10 rounded-md border" style={{ borderColor: secondaryColor }} />
                      </div>
                      <button type="button" className="w-full h-10 rounded-md text-white font-medium" style={{ backgroundColor: primaryColor }}>{t("settings.brandingPage.previewSignIn")}</button>
                      <p className="text-center text-sm" style={{ color: secondaryColor }}>{t("settings.brandingPage.previewForgotPassword")}</p>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div className="space-y-2">
              <h3 className="text-sm font-medium text-[var(--text-primary)] border-b pb-2">{t("settings.brandingPage.customCss")}<span className="font-normal text-[var(--text-secondary)] ml-2">({t("settings.brandingPage.advanced")})</span></h3>
              <div className="space-y-2">
                <Textarea id="custom_css" name="custom_css" placeholder={t("settings.brandingPage.customCssPlaceholder")} value={customCss} onChange={(e) => setCustomCss(e.target.value)} className="font-mono text-sm min-h-[120px]" />
                <p className="text-xs text-[var(--text-secondary)]">{t("settings.brandingPage.customCssHint")}</p>
              </div>
            </div>

            <div className="fixed inset-x-0 bottom-0 z-20 border-t border-[var(--glass-border-subtle)] bg-[var(--bg-secondary)]/95 px-6 py-4 backdrop-blur md:static md:inset-auto md:z-auto md:border-0 md:bg-transparent md:px-0 md:py-0 md:backdrop-blur-0">
              <div className="flex flex-wrap items-center gap-3">
                <Button type="submit" name="intent" value="save" disabled={isSubmitting && currentIntent === "save"}>{isSubmitting && currentIntent === "save" ? t("settings.brandingPage.saving") : t("settings.brandingPage.saveChanges")}</Button>
                <Button type="submit" name="intent" value="reset" variant="outline" disabled={isSubmitting || isDefault}>
                  <ResetIcon className="h-4 w-4 mr-2" />
                  {isSubmitting && currentIntent === "reset" ? t("settings.brandingPage.resetting") : t("settings.brandingPage.resetToDefaults")}
                </Button>
              </div>
            </div>
          </Form>
        </CardContent>
      </Card>
    </div>
  );
}
