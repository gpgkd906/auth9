import type { ActionFunctionArgs, LoaderFunctionArgs } from "react-router";
import { getAccessToken } from "~/services/session.server";
import { redirect } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { sessionApi, type SessionInfo } from "~/services/api";
import {
  DesktopIcon,
  MobileIcon,
  GlobeIcon,
  Cross2Icon,
  CheckCircledIcon,
} from "@radix-ui/react-icons";

export async function loader({ request }: LoaderFunctionArgs) {
  const accessToken = await getAccessToken(request);
  if (!accessToken) {
    throw redirect("/login");
  }

  try {
    const response = await sessionApi.listMySessions(accessToken);
    return { sessions: response.data };
  } catch {
    return { sessions: [], error: "Failed to load sessions" };
  }
}

export async function action({ request }: ActionFunctionArgs) {
  const accessToken = await getAccessToken(request);
  if (!accessToken) {
    return { error: "Not authenticated" };
  }

  const formData = await request.formData();
  const intent = formData.get("intent");

  try {
    if (intent === "revoke") {
      const sessionId = formData.get("sessionId") as string;
      await sessionApi.revokeSession(sessionId, accessToken);
      return redirect("/dashboard/account/sessions");
    }

    if (intent === "revoke_all") {
      await sessionApi.revokeOtherSessions(accessToken);
      return redirect("/dashboard/account/sessions");
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : "Operation failed";
    return { error: message };
  }

  return { error: "Invalid action" };
}

function getDeviceIcon(deviceType?: string) {
  switch (deviceType) {
    case "mobile":
      return <MobileIcon className="h-5 w-5" />;
    case "tablet":
      return <MobileIcon className="h-5 w-5" />;
    case "desktop":
    default:
      return <DesktopIcon className="h-5 w-5" />;
  }
}

function formatDate(dateString: string) {
  const date = new Date(dateString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffMins < 1) return "Just now";
  if (diffMins < 60) return `${diffMins} minutes ago`;
  if (diffHours < 24) return `${diffHours} hours ago`;
  if (diffDays < 7) return `${diffDays} days ago`;
  return date.toLocaleDateString();
}

export default function AccountSessionsPage() {
  const { sessions, error: loadError } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();

  const isSubmitting = navigation.state === "submitting";
  const currentSession = sessions.find((s: SessionInfo) => s.is_current);
  const otherSessions = sessions.filter((s: SessionInfo) => !s.is_current);

  return (
    <div className="space-y-6">
      {/* Current Session */}
      <Card>
        <CardHeader>
          <CardTitle>Current Session</CardTitle>
          <CardDescription>
            This is the device you are currently using.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {currentSession ? (
            <div className="flex items-start gap-4">
              <div className="p-3 bg-green-100 text-[var(--accent-green)] rounded-full">
                {getDeviceIcon(currentSession.device_type)}
              </div>
              <div className="flex-1">
                <div className="flex items-center gap-2">
                  <span className="font-medium">
                    {currentSession.device_name || "Unknown Device"}
                  </span>
                  <span className="inline-flex items-center gap-1 text-xs bg-green-100 text-[var(--accent-green)] px-2 py-0.5 rounded-full">
                    <CheckCircledIcon className="h-3 w-3" />
                    Current
                  </span>
                </div>
                <div className="text-sm text-[var(--text-secondary)] mt-1 space-y-0.5">
                  {currentSession.ip_address && (
                    <div className="flex items-center gap-1">
                      <GlobeIcon className="h-3 w-3" />
                      {currentSession.ip_address}
                      {currentSession.location && ` • ${currentSession.location} `}
                    </div>
                  )}
                  <div>
                    Last active: {formatDate(currentSession.last_active_at)}
                  </div>
                </div>
              </div>
            </div>
          ) : (
            <p className="text-[var(--text-secondary)]">Unable to identify current session</p>
          )}
        </CardContent>
      </Card>

      {/* Other Sessions */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <div>
            <CardTitle>Other Sessions</CardTitle>
            <CardDescription>
              Devices where you are currently signed in.
            </CardDescription>
          </div>
          {otherSessions.length > 0 && (
            <Form method="post">
              <input type="hidden" name="intent" value="revoke_all" />
              <Button
                type="submit"
                variant="destructive"
                size="sm"
                disabled={isSubmitting}
              >
                Sign out all
              </Button>
            </Form>
          )}
        </CardHeader>
        <CardContent>
          {loadError && (
            <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md mb-4">
              {loadError}
            </div>
          )}

          {actionData?.error && (
            <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md mb-4">
              {actionData.error}
            </div>
          )}

          {otherSessions.length === 0 ? (
            <p className="text-[var(--text-secondary)] text-center py-8">
              No other active sessions
            </p>
          ) : (
            <div className="divide-y">
              {otherSessions.map((session: SessionInfo) => (
                <div
                  key={session.id}
                  className="flex items-start gap-4 py-4 first:pt-0 last:pb-0"
                >
                  <div className="p-3 bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)] rounded-full">
                    {getDeviceIcon(session.device_type)}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="font-medium truncate">
                      {session.device_name || "Unknown Device"}
                    </div>
                    <div className="text-sm text-[var(--text-secondary)] mt-1 space-y-0.5">
                      {session.ip_address && (
                        <div className="flex items-center gap-1">
                          <GlobeIcon className="h-3 w-3" />
                          {session.ip_address}
                          {session.location && ` • ${session.location} `}
                        </div>
                      )}
                      <div>
                        Last active: {formatDate(session.last_active_at)}
                      </div>
                      <div className="text-xs text-[var(--text-tertiary)]">
                        Started: {new Date(session.created_at).toLocaleDateString()}
                      </div>
                    </div>
                  </div>
                  <Form method="post">
                    <input type="hidden" name="intent" value="revoke" />
                    <input type="hidden" name="sessionId" value={session.id} />
                    <Button
                      type="submit"
                      variant="ghost"
                      size="sm"
                      className="text-[var(--accent-red)] hover:text-[var(--accent-red)] hover:bg-[var(--accent-red)]/10"
                      disabled={isSubmitting}
                    >
                      <Cross2Icon className="h-4 w-4 mr-1" />
                      Revoke
                    </Button>
                  </Form>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Session Security Tips */}
      <Card>
        <CardHeader>
          <CardTitle>Security Tips</CardTitle>
        </CardHeader>
        <CardContent>
          <ul className="text-sm text-[var(--text-secondary)] space-y-2">
            <li>• Sign out of sessions you do not recognize</li>
            <li>• Do not stay signed in on shared or public devices</li>
            <li>• Enable two-factor authentication for extra security</li>
            <li>• Use unique, strong passwords for each account</li>
          </ul>
        </CardContent>
      </Card>
    </div>
  );
}
