import {
  Links,
  Meta,
  Outlet,
  Scripts,
  ScrollRestoration,
  useRouteError,
  isRouteErrorResponse,
} from "react-router";
import type { LinksFunction, MetaFunction } from "react-router";
import { ConfirmProvider } from "~/hooks/useConfirm";
import { useNonce } from "~/hooks/useNonce";
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

export const meta: MetaFunction = () => {
  return [
    { title: "Auth9 - Identity Management" },
    { name: "description", content: "Modern identity and access management" },
  ];
};

export function Layout({ children }: { children: React.ReactNode }) {
  const nonce = useNonce();
  return (
    <html lang="en" className="h-full">
      <head>
        <meta charSet="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <Meta />
        <Links />
        {/* External script to prevent theme flash on page load */}
        <script src="/theme-init.js" />
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
  return (
    <ConfirmProvider>
      <Outlet />
    </ConfirmProvider>
  );
}

export function ErrorBoundary() {
  const error = useRouteError();

  if (isRouteErrorResponse(error)) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="text-center">
          <h1 className="text-6xl font-bold text-gray-900">{error.status}</h1>
          <p className="mt-4 text-xl text-gray-600">
            {error.status === 404 ? "Page not found" : "Something went wrong"}
          </p>
          <a
            href="/"
            className="mt-8 inline-block px-6 py-3 bg-apple-blue text-white rounded-apple font-medium hover:bg-blue-600 transition-colors"
          >
            Go back home
          </a>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen flex items-center justify-center">
      <div className="text-center">
        <h1 className="text-6xl font-bold text-gray-900">Error</h1>
        <p className="mt-4 text-xl text-gray-600">
          Something went wrong. Please try again.
        </p>
        <a
          href="/"
          className="mt-8 inline-block px-6 py-3 bg-apple-blue text-white rounded-apple font-medium hover:bg-blue-600 transition-colors"
        >
          Go back home
        </a>
      </div>
    </div>
  );
}
