import type { LoaderFunctionArgs } from "react-router";
import { redirect } from "react-router";
import { commitSession } from "~/services/session.server";

export async function loader({ request }: LoaderFunctionArgs) {
    const url = new URL(request.url);
    const code = url.searchParams.get("code");
    const accessToken = url.searchParams.get("access_token");
    const expiresIn = url.searchParams.get("expires_in");
    const error = url.searchParams.get("error");
    const errorDescription = url.searchParams.get("error_description");

    if (error) {
        console.error("Auth error:", error, errorDescription);
        return redirect(`/login?error=${error}`);
    }

    // Handle implicit flow (access_token returned directly)
    if (accessToken) {
        const session = {
            accessToken: accessToken,
            refreshToken: undefined,
            idToken: undefined,
            expiresAt: Date.now() + (parseInt(expiresIn || "3600", 10) * 1000),
        };

        return redirect("/dashboard", {
            headers: {
                "Set-Cookie": await commitSession(session),
            },
        });
    }

    // Handle authorization code flow
    if (!code) {
        return redirect("/login");
    }

    try {
        const tokenUrl = `${process.env.AUTH9_CORE_URL || "http://localhost:8080"}/api/v1/auth/token`;

        // Authorization Code Exchange
        const response = await fetch(tokenUrl, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({
                grant_type: "authorization_code",
                client_id: process.env.AUTH9_PORTAL_CLIENT_ID || "auth9-portal",
                code,
                redirect_uri: `${process.env.AUTH9_PORTAL_URL || "http://localhost:3000"}/auth/callback`,
            }),
        });

        if (!response.ok) {
            const body = await response.text();
            console.error("Failed to exchange token:", response.status, body);
            return redirect("/login?error=token_exchange_failed");
        }

        const data = await response.json();

        // Store tokens in session cookie
        const session = {
            accessToken: data.access_token,
            refreshToken: data.refresh_token,
            idToken: data.id_token,
            expiresAt: Date.now() + (data.expires_in * 1000),
        };

        return redirect("/dashboard", {
            headers: {
                "Set-Cookie": await commitSession(session),
            },
        });

    } catch (err) {
        console.error("Callback error:", err);
        return redirect("/login?error=callback_exception");
    }
}
