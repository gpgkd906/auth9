import type { MetaFunction } from "@remix-run/node";
import { Link } from "@remix-run/react";
import { Button } from "~/components/ui/button";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";

export const meta: MetaFunction = () => {
  return [
    { title: "Auth9 - Modern Identity Management" },
    { name: "description", content: "Secure, scalable identity and access management" },
  ];
};

export default function Index() {
  return (
    <div className="min-h-screen bg-gradient-to-b from-gray-50 to-white">
      {/* Header */}
      <header className="fixed top-0 left-0 right-0 z-50 bg-white/80 backdrop-blur-apple border-b border-gray-200/50">
        <div className="max-w-7xl mx-auto px-6 h-16 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-apple bg-apple-blue flex items-center justify-center">
              <span className="text-white font-bold text-lg">A</span>
            </div>
            <span className="text-xl font-semibold">Auth9</span>
          </div>
          <nav className="flex items-center gap-6">
            <Link to="/login" className="text-gray-600 hover:text-gray-900 font-medium">
              Sign In
            </Link>
            <Button asChild>
              <Link to="/register">Get Started</Link>
            </Button>
          </nav>
        </div>
      </header>

      {/* Hero */}
      <main className="pt-32 pb-20 px-6">
        <div className="max-w-4xl mx-auto text-center">
          <h1 className="text-5xl md:text-6xl font-bold tracking-tight text-gray-900">
            Identity Management
            <br />
            <span className="text-gradient">Made Simple</span>
          </h1>
          <p className="mt-6 text-xl text-gray-600 max-w-2xl mx-auto">
            Secure, scalable authentication and authorization for your applications.
            Multi-tenant, SSO, and dynamic RBAC out of the box.
          </p>
          <div className="mt-10 flex items-center justify-center gap-4">
            <Button size="lg" asChild>
              <Link to="/register">Start Free Trial</Link>
            </Button>
            <Button size="lg" variant="outline" asChild>
              <Link to="/docs">Read Documentation</Link>
            </Button>
          </div>
        </div>

        {/* Features */}
        <div className="mt-32 max-w-6xl mx-auto grid md:grid-cols-3 gap-6">
          <Card>
            <CardHeader>
              <div className="w-12 h-12 rounded-apple-lg bg-blue-50 flex items-center justify-center mb-4">
                <svg className="w-6 h-6 text-apple-blue" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
                </svg>
              </div>
              <CardTitle>Single Sign-On</CardTitle>
              <CardDescription>
                One login for all your applications. Supports OIDC, SAML, and more.
              </CardDescription>
            </CardHeader>
          </Card>

          <Card>
            <CardHeader>
              <div className="w-12 h-12 rounded-apple-lg bg-purple-50 flex items-center justify-center mb-4">
                <svg className="w-6 h-6 text-apple-purple" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
                </svg>
              </div>
              <CardTitle>Multi-Tenant</CardTitle>
              <CardDescription>
                Built for SaaS. Isolate tenants with flexible configuration per organization.
              </CardDescription>
            </CardHeader>
          </Card>

          <Card>
            <CardHeader>
              <div className="w-12 h-12 rounded-apple-lg bg-green-50 flex items-center justify-center mb-4">
                <svg className="w-6 h-6 text-apple-green" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
                </svg>
              </div>
              <CardTitle>Dynamic RBAC</CardTitle>
              <CardDescription>
                Fine-grained access control with roles, permissions, and inheritance.
              </CardDescription>
            </CardHeader>
          </Card>
        </div>
      </main>

      {/* Footer */}
      <footer className="border-t border-gray-200 py-12 px-6">
        <div className="max-w-6xl mx-auto flex items-center justify-between">
          <p className="text-gray-500 text-sm">
            © 2024 Auth9. All rights reserved.
          </p>
          <div className="flex items-center gap-6">
            <Link to="/privacy" className="text-gray-500 hover:text-gray-900 text-sm">
              Privacy
            </Link>
            <Link to="/terms" className="text-gray-500 hover:text-gray-900 text-sm">
              Terms
            </Link>
          </div>
        </div>
      </footer>
    </div>
  );
}
