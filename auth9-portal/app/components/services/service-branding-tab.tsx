import { CheckCircledIcon, ResetIcon } from "@radix-ui/react-icons";
import { useEffect, useState } from "react";
import { Form } from "react-router";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Textarea } from "~/components/ui/textarea";
import { useI18n } from "~/i18n";
import type { BrandingConfig } from "~/services/api";

const DEFAULT_BRANDING: BrandingConfig = {
  primary_color: "#007AFF",
  secondary_color: "#5856D6",
  background_color: "#F5F5F7",
  text_color: "#1D1D1F",
  allow_registration: false,
};

function ColorPicker({
  defaultValue,
  id,
  label,
  onChange,
  value,
}: {
  defaultValue: string;
  id: string;
  label: string;
  onChange: (value: string) => void;
  value: string;
}) {
  const { t } = useI18n();

  return (
    <div className="space-y-2">
      <Label htmlFor={id}>{label}</Label>
      <div className="flex items-center gap-2">
        <label
          htmlFor={`${id}_picker`}
          className="block h-10 w-10 cursor-pointer rounded-md border border-gray-300 shadow-sm"
          style={{ backgroundColor: value }}
        >
          <span className="sr-only">{t("services.branding.chooseColor", { label })}</span>
        </label>
        <input
          type="color"
          id={`${id}_picker`}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          className="sr-only"
        />
        <Input
          id={id}
          name={id}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          placeholder={defaultValue}
          className="font-mono uppercase"
          maxLength={7}
        />
      </div>
    </div>
  );
}

interface ServiceBrandingTabProps {
  branding: BrandingConfig | null;
  currentIntent: string | null;
  deleteSucceeded: boolean;
  isSubmitting: boolean;
  updateSucceeded: boolean;
}

export function ServiceBrandingTab({
  branding,
  currentIntent,
  deleteSucceeded,
  isSubmitting,
  updateSucceeded,
}: ServiceBrandingTabProps) {
  const { t } = useI18n();
  const [isCustomizing, setIsCustomizing] = useState(Boolean(branding));
  const [logoUrl, setLogoUrl] = useState(branding?.logo_url || "");
  const [primaryColor, setPrimaryColor] = useState(branding?.primary_color || DEFAULT_BRANDING.primary_color);
  const [secondaryColor, setSecondaryColor] = useState(
    branding?.secondary_color || DEFAULT_BRANDING.secondary_color
  );
  const [backgroundColor, setBackgroundColor] = useState(
    branding?.background_color || DEFAULT_BRANDING.background_color
  );
  const [textColor, setTextColor] = useState(branding?.text_color || DEFAULT_BRANDING.text_color);
  const [customCss, setCustomCss] = useState(branding?.custom_css || "");
  const [companyName, setCompanyName] = useState(branding?.company_name || "");
  const [faviconUrl, setFaviconUrl] = useState(branding?.favicon_url || "");
  const [allowRegistration, setAllowRegistration] = useState(branding?.allow_registration ?? false);

  const resetToDefault = () => {
    setLogoUrl("");
    setPrimaryColor(DEFAULT_BRANDING.primary_color);
    setSecondaryColor(DEFAULT_BRANDING.secondary_color);
    setBackgroundColor(DEFAULT_BRANDING.background_color);
    setTextColor(DEFAULT_BRANDING.text_color);
    setCustomCss("");
    setCompanyName("");
    setFaviconUrl("");
    setAllowRegistration(false);
  };

  useEffect(() => {
    setIsCustomizing(Boolean(branding));
    setLogoUrl(branding?.logo_url || "");
    setPrimaryColor(branding?.primary_color || DEFAULT_BRANDING.primary_color);
    setSecondaryColor(branding?.secondary_color || DEFAULT_BRANDING.secondary_color);
    setBackgroundColor(branding?.background_color || DEFAULT_BRANDING.background_color);
    setTextColor(branding?.text_color || DEFAULT_BRANDING.text_color);
    setCustomCss(branding?.custom_css || "");
    setCompanyName(branding?.company_name || "");
    setFaviconUrl(branding?.favicon_url || "");
    setAllowRegistration(branding?.allow_registration ?? false);
  }, [branding]);

  useEffect(() => {
    if (deleteSucceeded) {
      setIsCustomizing(false);
      resetToDefault();
    }
  }, [deleteSucceeded]);

  if (!isCustomizing) {
    return (
      <Card>
        <CardContent className="py-12">
          <div className="text-center">
            <h3 className="mb-2 text-lg font-semibold">{t("services.branding.systemDefaultTitle")}</h3>
            <p className="mb-4 text-[var(--text-secondary)]">{t("services.branding.systemDefaultDescription")}</p>
            <Button onClick={() => setIsCustomizing(true)}>{t("services.branding.customize")}</Button>
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-4">
      {updateSucceeded && (
        <div className="flex items-center gap-2 rounded-xl border border-[var(--accent-green)]/20 bg-[var(--accent-green)]/10 p-4 text-sm text-[var(--accent-green)]">
          <CheckCircledIcon className="h-4 w-4" />
          {t("services.detail.brandingSaved")}
        </div>
      )}

      <Card>
        <CardHeader>
          <CardTitle>{t("services.branding.title")}</CardTitle>
          <CardDescription>{t("services.branding.description")}</CardDescription>
        </CardHeader>
        <CardContent>
          <Form method="post" className="space-y-6">
            <div className="space-y-2">
              <h3 className="border-b pb-2 text-sm font-medium text-[var(--text-primary)]">
                {t("services.branding.companyIdentity")}
              </h3>
              <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="company_name">{t("services.branding.companyName")}</Label>
                  <Input
                    id="company_name"
                    name="company_name"
                    placeholder={t("services.branding.companyNamePlaceholder")}
                    value={companyName}
                    onChange={(event) => setCompanyName(event.target.value)}
                    maxLength={100}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="logo_url">{t("services.branding.logoUrl")}</Label>
                  <Input
                    id="logo_url"
                    name="logo_url"
                    type="url"
                    placeholder={t("services.branding.logoUrlPlaceholder")}
                    value={logoUrl}
                    onChange={(event) => setLogoUrl(event.target.value)}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="favicon_url">{t("services.branding.faviconUrl")}</Label>
                  <Input
                    id="favicon_url"
                    name="favicon_url"
                    type="url"
                    placeholder={t("services.branding.faviconUrlPlaceholder")}
                    value={faviconUrl}
                    onChange={(event) => setFaviconUrl(event.target.value)}
                  />
                </div>
              </div>
            </div>

            <div className="space-y-2">
              <h3 className="border-b pb-2 text-sm font-medium text-[var(--text-primary)]">
                {t("services.branding.loginOptions")}
              </h3>
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label htmlFor="allow_registration">{t("services.branding.allowRegistration")}</Label>
                  <p className="text-xs text-[var(--text-secondary)]">
                    {t("services.branding.allowRegistrationHint")}
                  </p>
                </div>
                <label htmlFor="allow_registration" className="relative inline-flex cursor-pointer items-center">
                  <span className="sr-only">{t("services.branding.toggleAllowRegistration")}</span>
                  <input
                    type="checkbox"
                    id="allow_registration"
                    name="allow_registration"
                    value="true"
                    checked={allowRegistration}
                    onChange={(event) => setAllowRegistration(event.target.checked)}
                    className="peer sr-only"
                  />
                  <div className="peer h-6 w-11 rounded-full bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 peer-checked:bg-blue-600 peer-checked:after:translate-x-full peer-checked:after:border-white after:absolute after:top-[2px] after:left-[2px] after:h-5 after:w-5 after:rounded-full after:border after:border-gray-300 after:bg-white after:transition-all after:content-['']"></div>
                </label>
              </div>
            </div>

            <div className="space-y-2">
              <h3 className="border-b pb-2 text-sm font-medium text-[var(--text-primary)]">
                {t("services.branding.colors")}
              </h3>
              <div className="grid grid-cols-2 gap-4">
                <ColorPicker
                  id="primary_color"
                  label={t("services.branding.primaryColor")}
                  value={primaryColor}
                  onChange={setPrimaryColor}
                  defaultValue={DEFAULT_BRANDING.primary_color}
                />
                <ColorPicker
                  id="secondary_color"
                  label={t("services.branding.secondaryColor")}
                  value={secondaryColor}
                  onChange={setSecondaryColor}
                  defaultValue={DEFAULT_BRANDING.secondary_color}
                />
                <ColorPicker
                  id="background_color"
                  label={t("services.branding.backgroundColor")}
                  value={backgroundColor}
                  onChange={setBackgroundColor}
                  defaultValue={DEFAULT_BRANDING.background_color}
                />
                <ColorPicker
                  id="text_color"
                  label={t("services.branding.textColor")}
                  value={textColor}
                  onChange={setTextColor}
                  defaultValue={DEFAULT_BRANDING.text_color}
                />
              </div>
            </div>

            <div className="space-y-2">
              <h3 className="border-b pb-2 text-sm font-medium text-[var(--text-primary)]">
                {t("services.branding.customCss")}
                <span className="ml-2 font-normal text-[var(--text-secondary)]">
                  ({t("services.branding.advanced")})
                </span>
              </h3>
              <Textarea
                id="custom_css"
                name="custom_css"
                placeholder={t("services.branding.customCssPlaceholder")}
                value={customCss}
                onChange={(event) => setCustomCss(event.target.value)}
                className="min-h-[120px] font-mono text-sm"
              />
            </div>

            <div className="flex flex-wrap items-center gap-3 border-t pt-4">
              <Button
                type="submit"
                name="intent"
                value="update_branding"
                disabled={isSubmitting && currentIntent === "update_branding"}
              >
                {isSubmitting && currentIntent === "update_branding"
                  ? t("services.detail.saving")
                  : t("services.branding.saveBranding")}
              </Button>

              {branding ? (
                <Button type="submit" name="intent" value="delete_branding" variant="destructive" disabled={isSubmitting}>
                  <ResetIcon className="mr-2 h-4 w-4" />
                  {t("services.branding.resetToDefault")}
                </Button>
              ) : (
                <Button
                  type="button"
                  variant="destructive"
                  disabled={isSubmitting}
                  onClick={() => {
                    resetToDefault();
                    setIsCustomizing(false);
                  }}
                >
                  <ResetIcon className="mr-2 h-4 w-4" />
                  {t("services.branding.resetToDefault")}
                </Button>
              )}
            </div>
          </Form>
        </CardContent>
      </Card>
    </div>
  );
}
