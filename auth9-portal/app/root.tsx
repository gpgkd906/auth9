import {
  Links,
  Meta,
  Outlet,
  Scripts,
  ScrollRestoration,
  useRouteLoaderData,
  useRouteError,
  isRouteErrorResponse,
} from "react-router";
import type { LinksFunction, LoaderFunctionArgs, MetaFunction } from "react-router";
import { I18nProvider, type AppLocale } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { translate } from "~/i18n/translate";
import { ConfirmProvider } from "~/hooks/useConfirm";
import { useNonce } from "~/hooks/useNonce";
import { resolveLocale } from "~/services/locale.server";
import "./styles/tailwind.css";

export const links: LinksFunction = () => [
  { rel: "preconnect", href: "https://fonts.googleapis.com" },
  {
    rel: "preconnect",
    href: "https://fonts.gstatic.com",
    crossOrigin: "anonymous",
  },
  {
    rel: "stylesheet",
    href: "https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap",
  },
];

export async function loader({ request }: LoaderFunctionArgs) {
  const locale = await resolveLocale(request);
  return { locale };
}

export const meta: MetaFunction<typeof loader> = ({ matches }) => {
  const locale = resolveMetaLocale(matches);
  return buildMeta(locale, "common.meta.appTitle", "common.meta.appDescription");
};

export function Layout({ children }: { children: React.ReactNode }) {
  const nonce = useNonce();
  const rootData = useRouteLoaderData("root") as { locale?: AppLocale } | undefined;
  const locale = rootData?.locale ?? "zh-CN";
  return (
    <html lang={locale} className="h-full" data-theme="light" suppressHydrationWarning>
      <head>
        <meta charSet="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <Meta />
        <Links />
        {/* Blocking script to prevent theme flash on page load */}
        <script src="/theme-init.js" nonce={nonce} />
      </head>
      <body className="h-full antialiased">
        {children}
        <ScrollRestoration nonce={nonce} />
        <Scripts nonce={nonce} />
      </body>
    </html>
  );
}

export default function App() {
  const rootData = useRouteLoaderData("root") as { locale?: AppLocale } | undefined;
  const locale = (rootData?.locale ?? "zh-CN") as AppLocale;
  return (
    <I18nProvider locale={locale}>
      <ConfirmProvider>
        <Outlet />
      </ConfirmProvider>
    </I18nProvider>
  );
}

function readLocaleFromCookie(): AppLocale | null {
  if (typeof document === "undefined") return null;
  const match = document.cookie.split("; ").find((c) => c.startsWith("auth9_locale="));
  if (!match) return null;
  const raw = decodeURIComponent(match.split("=")[1] || "");
  if (raw === "en-US" || raw === "zh-CN" || raw === "ja") return raw;
  return null;
}

export function ErrorBoundary() {
  const error = useRouteError();
  const rootData = useRouteLoaderData("root") as { locale?: AppLocale } | undefined;
  const locale: AppLocale = rootData?.locale
    || readLocaleFromCookie()
    || (typeof document !== "undefined" && (document.documentElement.lang as AppLocale))
    || "zh-CN";

  if (isRouteErrorResponse(error)) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-[var(--bg-primary)]">
        <div className="text-center">
          <h1 className="text-6xl font-bold text-[var(--text-primary)]">{error.status}</h1>
          <p className="mt-4 text-xl text-[var(--text-secondary)]">
            {error.status === 404
              ? translate(locale, "common.errors.pageNotFound")
              : translate(locale, "common.errors.somethingWentWrong")}
          </p>
          <a
            href="/"
            className="mt-8 inline-block px-6 py-3 bg-apple-blue text-white rounded-apple font-medium hover:bg-blue-600 transition-colors"
          >
            {translate(locale, "common.errors.goBackHome")}
          </a>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-[var(--bg-primary)]">
      <div className="text-center">
        <h1 className="text-6xl font-bold text-[var(--text-primary)]">Error</h1>
        <p className="mt-4 text-xl text-[var(--text-secondary)]">
          {translate(locale, "common.errors.somethingWentWrong")}
        </p>
        <a
          href="/"
          className="mt-8 inline-block px-6 py-3 bg-apple-blue text-white rounded-apple font-medium hover:bg-blue-600 transition-colors"
        >
          {translate(locale, "common.errors.goBackHome")}
        </a>
      </div>
    </div>
  );
}
