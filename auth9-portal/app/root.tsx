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
  return (
    <html lang="en" className="h-full">
      <head>
        <meta charSet="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <Meta />
        <Links />
        {/* Inline script to prevent theme flash on page load */}
        <script
          dangerouslySetInnerHTML={{
            __html: `
              (function() {
                try {
                  var theme = localStorage.getItem('auth9-theme');
                  if (theme === 'dark' || (!theme && window.matchMedia('(prefers-color-scheme: dark)').matches)) {
                    document.documentElement.setAttribute('data-theme', 'dark');
                  }
                } catch (e) {}
              })();
            `,
          }}
        />
      </head>
      <body className="h-full antialiased">
        {children}
        <ScrollRestoration />
        <Scripts />
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
          <p className="mt-4 text-xl text-gray-600">{error.statusText}</p>
          {error.data && (
            <p className="mt-2 text-gray-500">{error.data}</p>
          )}
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
