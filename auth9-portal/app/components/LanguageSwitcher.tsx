import { useRevalidator } from "react-router";
import { useLocale, useI18n, type AppLocale } from "~/i18n";

export function LanguageSwitcher() {
  const { t } = useI18n();
  const { locale, setLocale } = useLocale();
  const revalidator = useRevalidator();

  const handleChange = async (event: React.ChangeEvent<HTMLSelectElement>) => {
    const nextLocale = event.target.value as AppLocale;
    if (nextLocale === locale) return;
    await setLocale(nextLocale);
    revalidator.revalidate();
  };

  return (
    <label className="inline-flex items-center gap-2 text-sm text-[var(--text-secondary)]">
      <span className="sr-only">{t("common.language.switcherLabel")}</span>
      <select
        aria-label={t("common.language.switcherLabel")}
        value={locale}
        onChange={handleChange}
        className="rounded-lg border border-[var(--glass-border-subtle)] bg-[var(--glass-bg)] px-3 py-2 text-sm text-[var(--text-primary)] outline-none"
      >
        <option value="zh-CN">{t("common.language.zhCN")}</option>
        <option value="en-US">{t("common.language.enUS")}</option>
        <option value="ja">{t("common.language.ja")}</option>
      </select>
    </label>
  );
}
