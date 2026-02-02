import type { Route } from "./+types/auth.callback";
import { redirect } from "react-router";
import { commitSession } from "~/services/session.server";

export async function loader({ request }: Route.LoaderArgs) {
    const url = new URL(request.url);
    const code = url.searchParams.get("code");
    const error = url.searchParams.get("error");
    const errorDescription = url.searchParams.get("error_description");

    if (error) {
        console.error("Auth error:", error, errorDescription);
        return redirect(`/login?error=${error}`);
    }

    if (!code) {
        return redirect("/login");
    }

    // Initial code exchange - in a real app, you might verify state parameter here
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
