import { ActionTrigger } from "@auth9/core";
import type { AppLocale } from "~/i18n";
import { translate } from "~/i18n/translate";

const triggerKeyMap: Record<string, string> = {
  [ActionTrigger.PostLogin]: "serviceActions.triggers.postLogin",
  [ActionTrigger.PreUserRegistration]: "serviceActions.triggers.preUserRegistration",
  [ActionTrigger.PostUserRegistration]: "serviceActions.triggers.postUserRegistration",
  [ActionTrigger.PostChangePassword]: "serviceActions.triggers.postChangePassword", // pragma: allowlist secret
  [ActionTrigger.PostEmailVerification]: "serviceActions.triggers.postEmailVerification",
  [ActionTrigger.PreTokenRefresh]: "serviceActions.triggers.preTokenRefresh",
};

export function getActionTriggerLabel(locale: AppLocale, triggerId: string) {
  return translate(locale, triggerKeyMap[triggerId] || "serviceActions.triggers.unknown");
}

export function getActionTriggerLabelFromT(t: (key: string, values?: Record<string, string | number | boolean>) => string, triggerId: string) {
  return t(triggerKeyMap[triggerId] || "serviceActions.triggers.unknown");
}

export function getDefaultActionScript(locale: AppLocale) {
  return locale === "zh-CN"
    ? "// 在这里编写 TypeScript 代码\ncontext;"
    : "// Your TypeScript code here\ncontext;";
}

export function getActionContextReference() {
  return `interface ActionContext {
  user: {
    id: string;
    email: string;
    display_name?: string;
    mfa_enabled: boolean;
  };
  tenant: {
    id: string;
    slug: string;
    name: string;
  };
  request: {
    ip?: string;
    user_agent?: string;
    timestamp: string;
  };
  claims?: Record<string, unknown>;
}`;
}

export function getActionScriptTemplates(locale: AppLocale) {
  if (locale === "zh-CN") {
    return {
      "add-claims": {
        name: "添加自定义声明",
        description: "向用户令牌添加自定义 claims",
        script: `// 向令牌添加自定义声明
context.claims = context.claims || {};
context.claims.department = "engineering";
context.claims.tier = "premium";

// 返回修改后的上下文
context;`,
      },
      "block-domain": {
        name: "阻止邮箱域名",
        description: "阻止特定邮箱域名的用户",
        script: `// 阻止特定邮箱域名
const blockedDomains = ["@competitor.com", "@spam.com"];
if (blockedDomains.some(domain => context.user.email.endsWith(domain))) {
  throw new Error("Email domain not allowed");
}

context;`,
      },
      "require-mfa": {
        name: "条件 MFA",
        description: "对特定 IP 范围要求 MFA",
        script: `// 对特定 IP 范围要求 MFA
if (context.request.ip?.startsWith("203.")) {
  context.claims = context.claims || {};
  context.claims.require_mfa = true;
}

context;`,
      },
      "service-access": {
        name: "服务访问控制",
        description: "根据角色授予服务访问权限",
        script: `// 检查用户角色并授予服务访问权限
const allowedRoles = ["admin", "developer"];
const userRoles = (context.claims?.roles as string[]) || [];

const hasAccess = allowedRoles.some(role => userRoles.includes(role));

if (!hasAccess) {
  throw new Error("Insufficient permissions");
}

// 授予服务访问权限
context.claims = context.claims || {};
context.claims.service_access = context.claims.service_access || [];
(context.claims.service_access as string[]).push("my-service");

context;`,
      },
    };
  }

  return {
    "add-claims": {
      name: "Add Custom Claims",
      description: "Add custom claims to the user's token",
      script: `// Add custom claims to token
context.claims = context.claims || {};
context.claims.department = "engineering";
context.claims.tier = "premium";

// Return modified context
context;`,
    },
    "block-domain": {
      name: "Block Email Domain",
      description: "Block users from specific email domains",
      script: `// Block specific email domains
const blockedDomains = ["@competitor.com", "@spam.com"];
if (blockedDomains.some(domain => context.user.email.endsWith(domain))) {
  throw new Error("Email domain not allowed");
}

context;`,
    },
    "require-mfa": {
      name: "Conditional MFA",
      description: "Require MFA for specific IP ranges",
      script: `// Require MFA for specific IP ranges
if (context.request.ip?.startsWith("203.")) {
  context.claims = context.claims || {};
  context.claims.require_mfa = true;
}

context;`,
    },
    "service-access": {
      name: "Service Access Control",
      description: "Grant access to services based on roles",
      script: `// Check user roles and grant service access
const allowedRoles = ["admin", "developer"];
const userRoles = (context.claims?.roles as string[]) || [];

const hasAccess = allowedRoles.some(role => userRoles.includes(role));

if (!hasAccess) {
  throw new Error("Insufficient permissions");
}

// Grant service access
context.claims = context.claims || {};
context.claims.service_access = context.claims.service_access || [];
(context.claims.service_access as string[]).push("my-service");

context;`,
    },
  };
}
