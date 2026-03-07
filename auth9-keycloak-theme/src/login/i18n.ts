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
      selectOtpDevice: "Select OTP Device",
      orContinueWith: "Or continue with",
      alreadyHaveAccount: "Already have an account?",
      lightMode: "Light mode",
      darkMode: "Dark mode",
    },
    ja: {
      backToLogin: "← ログインに戻る",
      selectOtpDevice: "OTPデバイスを選択",
      orContinueWith: "または以下で続ける",
      alreadyHaveAccount: "アカウントをお持ちですか？",
      lightMode: "ライトモード",
      darkMode: "ダークモード",
    },
    "zh-CN": {
      backToLogin: "← 返回登录",
      selectOtpDevice: "选择验证设备",
      orContinueWith: "或使用以下方式继续",
      alreadyHaveAccount: "已有账户？",
      lightMode: "浅色模式",
      darkMode: "深色模式",
    },
  })
  .build();

type I18n = typeof ofTypeI18n;

export { useI18n, type I18n };
