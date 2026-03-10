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
      configTotpSubtitle: "Set up your authenticator app to secure your account",
      configTotpDevicePlaceholder: "e.g. My Phone",
      selectAuthenticatorSubtitle: "Choose how you want to verify your identity",
      resetOtpDescription: "Select the authenticator device to reset",
    },
    ja: {
      backToLogin: "← ログインに戻る",
      selectOtpDevice: "OTPデバイスを選択",
      orContinueWith: "または以下で続ける",
      alreadyHaveAccount: "アカウントをお持ちですか？",
      lightMode: "ライトモード",
      darkMode: "ダークモード",
      configTotpSubtitle: "認証アプリを設定してアカウントを保護します",
      configTotpDevicePlaceholder: "例：マイフォン",
      selectAuthenticatorSubtitle: "本人確認の方法を選択してください",
      resetOtpDescription: "リセットする認証デバイスを選択してください",
    },
    "zh-CN": {
      backToLogin: "← 返回登录",
      selectOtpDevice: "选择验证设备",
      orContinueWith: "或使用以下方式继续",
      alreadyHaveAccount: "已有账户？",
      lightMode: "浅色模式",
      darkMode: "深色模式",
      configTotpSubtitle: "设置验证器应用以保护您的账户",
      configTotpDevicePlaceholder: "例：我的手机",
      selectAuthenticatorSubtitle: "选择您的身份验证方式",
      resetOtpDescription: "选择要重置的验证设备",
    },
  })
  .build();

type I18n = typeof ofTypeI18n;

export { useI18n, type I18n };
