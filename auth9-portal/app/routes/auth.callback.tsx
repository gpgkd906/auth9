import type { LoaderFunctionArgs } from "react-router";
import { redirect } from "react-router";
import { commitSession, getOAuthState, clearOAuthStateCookie } from "~/services/session.server";
import { invitationApi } from "~/services/api";

function parseOAuthState(stateParam: string | null): { inviteToken?: string } {
    if (!stateParam) return {};
    try {
        const decoded = Buffer.from(stateParam, "base64url").toString("utf-8");
        const parsed = JSON.parse(decoded);
        if (parsed && typeof parsed === "object" && parsed.invite_token) {
            return { inviteToken: parsed.invite_token };
        }
    } catch {
        // Not a JSON state (plain UUID), ignore
    }
    return {};
}

export async function loader({ request }: LoaderFunctionArgs) {
    const url = new URL(request.url);
    const portalOrigin = url.origin;
    const code = url.searchParams.get("code");
    const state = url.searchParams.get("state");
    const error = url.searchParams.get("error");
    const errorDescription = url.searchParams.get("error_description");

    if (error) {
        console.error("Auth error:", error, errorDescription);
        return redirect(`/login?error=${error}`);
    }

    // Only authorization code flow is supported.
    if (!code) {
        return redirect("/login");
    }

    // Validate OAuth state to prevent Login CSRF
    const storedState = await getOAuthState(request);
    if (!storedState || storedState !== state) {
        console.error("OAuth state mismatch", { hasStored: !!storedState, hasReceived: !!state });
        return redirect("/login?error=state_mismatch");
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
                redirect_uri: `${portalOrigin}/auth/callback`,
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

        // Check if there's a pending invitation to accept
        const { inviteToken } = parseOAuthState(state);
        if (inviteToken) {
            try {
                await invitationApi.accept({ token: inviteToken });
            } catch (err) {
                console.error("Failed to auto-accept invitation:", err);
            }
        }

        return redirect("/dashboard", {
            headers: [
                ["Set-Cookie", await commitSession(session)],
                ["Set-Cookie", await clearOAuthStateCookie()],
            ],
        });

    } catch (err) {
        console.error("Callback error:", err);
        return redirect("/login?error=callback_exception");
    }
}
