import { i18nBuilder } from "keycloakify/login";
import type { ThemeName } from "../kc.gen";

/**
 * i18n configuration for auth9 theme.
 * Add custom translations here.
 */
const { useI18n, ofTypeI18n } = i18nBuilder
  .withThemeName<ThemeName>()
  .withCustomTranslations({
    en: {
      backToLogin: "← Back to Login",
    },
    ja: {
      backToLogin: "← ログインに戻る",
    },
    "zh-CN": {
      backToLogin: "← 返回登录",
    },
  })
  .build();

type I18n = typeof ofTypeI18n;

export { useI18n, type I18n };
