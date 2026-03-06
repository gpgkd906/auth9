import js from "@eslint/js";
import tseslint from "typescript-eslint";
import reactPlugin from "eslint-plugin-react";
import reactHooksPlugin from "eslint-plugin-react-hooks";
import globals from "globals";

const UI_TEXT_PATTERN = /[A-Za-z\u4E00-\u9FFF]/;
const UI_ATTRIBUTE_NAMES = new Set(["placeholder", "title", "aria-label", "aria-description", "alt"]);
const UI_OBJECT_KEYS = new Set(["title", "description", "label", "confirmLabel", "cancelLabel", "placeholder", "alt"]);

function isMeaningfulText(value) {
  return typeof value === "string" && UI_TEXT_PATTERN.test(value.trim());
}

function getStaticString(node) {
  if (!node) return null;
  if (node.type === "Literal" && typeof node.value === "string") {
    return node.value;
  }
  if (node.type === "TemplateLiteral" && node.expressions.length === 0) {
    return node.quasis.map((quasi) => quasi.value.cooked ?? "").join("");
  }
  return null;
}

const noBareUiStringsRule = {
  meta: {
    type: "problem",
    docs: {
      description: "Disallow hardcoded user-visible strings outside i18n resources",
    },
    messages: {
      bareUiString: "User-visible string should come from i18n, not a hardcoded literal.",
    },
    schema: [],
  },
  create(context) {
    return {
      JSXAttribute(node) {
        if (!UI_ATTRIBUTE_NAMES.has(node.name.name)) return;
        if (!node.value) return;
        if (node.value.type === "Literal" && isMeaningfulText(node.value.value)) {
          context.report({ node: node.value, messageId: "bareUiString" });
          return;
        }
        if (node.value.type === "JSXExpressionContainer") {
          const text = getStaticString(node.value.expression);
          if (isMeaningfulText(text)) {
            context.report({ node: node.value.expression, messageId: "bareUiString" });
          }
        }
      },
      Property(node) {
        if (node.parent?.type !== "ObjectExpression") return;
        if (node.parent.parent?.type !== "CallExpression") return;
        if (node.parent.parent.callee.type !== "Identifier" || node.parent.parent.callee.name !== "confirm") return;
        if (node.computed || node.kind !== "init") return;
        if (node.key.type !== "Identifier" && node.key.type !== "Literal") return;
        const keyName = node.key.type === "Identifier" ? node.key.name : node.key.value;
        if (!UI_OBJECT_KEYS.has(keyName)) return;
        const text = getStaticString(node.value);
        if (isMeaningfulText(text)) {
          context.report({ node: node.value, messageId: "bareUiString" });
        }
      },
    };
  },
};

export default tseslint.config(
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    ignores: [
      "node_modules/**",
      "build/**",
      ".cache/**",
      ".react-router/**",
      "public/**",
      "coverage/**",
      "*.d.ts",
    ],
  },
  {
    files: ["**/*.{js,jsx,ts,tsx}"],
    plugins: {
      react: reactPlugin,
      "react-hooks": reactHooksPlugin,
    },
    languageOptions: {
      ecmaVersion: "latest",
      sourceType: "module",
      globals: {
        ...globals.browser,
        ...globals.node,
        ...globals.es2021,
      },
      parserOptions: {
        ecmaFeatures: {
          jsx: true,
        },
      },
    },
    settings: {
      react: {
        version: "detect",
      },
      formComponents: ["Form"],
      linkComponents: [
        { name: "Link", linkAttribute: "to" },
        { name: "NavLink", linkAttribute: "to" },
      ],
    },
    rules: {
      ...reactPlugin.configs.recommended.rules,
      ...reactPlugin.configs["jsx-runtime"].rules,
      ...reactHooksPlugin.configs.recommended.rules,
      "react/prop-types": "off",
    },
  },
  {
    files: ["app/root.tsx", "app/routes/**/*.tsx", "app/components/**/*.tsx"],
    ignores: ["app/components/ui/**", "app/i18n/**"],
    plugins: {
      "auth9-i18n": {
        rules: {
          "no-bare-ui-strings": noBareUiStringsRule,
        },
      },
    },
    rules: {
      "auth9-i18n/no-bare-ui-strings": "error",
    },
  },
  {
    files: ["tests/**/*.ts", "tests/**/*.tsx"],
    rules: {
      "@typescript-eslint/no-explicit-any": "warn",
    },
  }
);
