import type { ActionFunctionArgs, LoaderFunctionArgs } from "react-router";
import { requireIdentityAuthWithUpdate } from "~/services/session.server";
import { redirect, Form, useActionData, useLoaderData, useNavigation } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { useFormatters } from "~/i18n/format";
import { useI18n } from "~/i18n";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";
import { mapApiError } from "~/lib/error-messages";
import { sessionApi, type SessionInfo } from "~/services/api";
import {
  DesktopIcon,
  MobileIcon,
  GlobeIcon,
  Cross2Icon,
  CheckCircledIcon,
} from "@radix-ui/react-icons";

export async function loader({ request }: LoaderFunctionArgs) {
  // Session API requires the identity token (with 'sid' claim), not the tenant access token
  const { session, headers } = await requireIdentityAuthWithUpdate(request);
  const identityToken = session.identityAccessToken || "";

  try {
    const response = await sessionApi.listMySessions(identityToken);
    const data = { sessions: response.data, error: undefined as string | undefined };
    if (headers) {
      return Response.json(data, { headers });
    }
    return data;
  } catch {
    const locale = await resolveLocale(request);
    return { sessions: [], error: translate(locale, "account.sessions.loadError") };
  }
}

export async function action({ request }: ActionFunctionArgs) {
  // Session API requires the identity token (with 'sid' claim), not the tenant access token
  const { session } = await requireIdentityAuthWithUpdate(request);
  const accessToken = session.identityAccessToken || "";

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
    const locale = await resolveLocale(request);
    const message = mapApiError(error, locale);
    return { error: message };
  }

  const locale = await resolveLocale(request);
  return { error: translate(locale, "account.sessions.invalidAction") };
}

function getDeviceIcon(deviceType?: string) {
  switch (deviceType) {
    case "mobile":
      return <MobileIcon className="h-5 w-5" />;
    case "tablet":
      return <MobileIcon className="h-5 w-5" />;
    case "unknown":
      return <GlobeIcon className="h-5 w-5" />;
    case "desktop":
      return <DesktopIcon className="h-5 w-5" />;
    default:
      return <GlobeIcon className="h-5 w-5" />;
  }
}

function getDeviceLabel(session: SessionInfo, t: ReturnType<typeof useI18n>["t"]) {
  if (session.device_name) return session.device_name;

  switch (session.device_type) {
    case "desktop":
      return "Desktop Browser";
    case "mobile":
    case "tablet":
      return "Mobile Device";
    default:
      return t("account.sessions.unknownDevice");
  }
}

function formatRelativeDate(dateString: string, t: ReturnType<typeof useI18n>["t"]) {
  const date = new Date(dateString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffMins < 1) return t("account.sessions.justNow");
  if (diffMins < 60) return t("account.sessions.minutesAgo", { count: diffMins });
  if (diffHours < 24) return t("account.sessions.hoursAgo", { count: diffHours });
  if (diffDays < 7) return t("account.sessions.daysAgo", { count: diffDays });
  return date.toLocaleDateString();
}

export default function AccountSessionsPage() {
  const { t } = useI18n();
  const formatters = useFormatters();
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
          <CardTitle>{t("account.sessions.currentTitle")}</CardTitle>
          <CardDescription>
            {t("account.sessions.currentDescription")}
          </CardDescription>
        </CardHeader>
        <CardContent>
          {currentSession ? (
            <div className="flex items-start gap-4 rounded-xl border border-[var(--accent-green)]/20 border-l-4 border-l-[var(--accent-green)] bg-[var(--accent-green)]/5 px-4 py-4">
              <div className="p-3 bg-green-100 text-[var(--accent-green)] rounded-full">
                {getDeviceIcon(currentSession.device_type)}
              </div>
              <div className="flex-1">
                <div className="flex items-center gap-2">
                  <span className="font-medium">
                    {getDeviceLabel(currentSession, t)}
                  </span>
                  <span className="inline-flex items-center gap-1 text-xs bg-green-100 text-[var(--accent-green)] px-2 py-0.5 rounded-full">
                    <CheckCircledIcon className="h-3 w-3" />
                    {t("account.sessions.current")}
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
                    {t("account.sessions.lastActive")}: {formatRelativeDate(currentSession.last_active_at, t)}
                  </div>
                </div>
              </div>
            </div>
          ) : (
            <p className="text-[var(--text-secondary)]">{t("account.sessions.unknownCurrent")}</p>
          )}
        </CardContent>
      </Card>

      {/* Other Sessions */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <div>
            <CardTitle>{t("account.sessions.otherTitle")}</CardTitle>
            <CardDescription>
              {t("account.sessions.otherDescription")}
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
                {t("account.sessions.signOutAll")}
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
              {t("account.sessions.noOtherSessions")}
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
                      {getDeviceLabel(session, t)}
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
                        {t("account.sessions.lastActive")}: {formatRelativeDate(session.last_active_at, t)}
                      </div>
                      <div className="text-xs text-[var(--text-tertiary)]">
                        {t("account.sessions.started")}: {formatters.date(session.created_at)}
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
                      {t("account.sessions.revoke")}
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
          <CardTitle>{t("account.sessions.securityTips")}</CardTitle>
        </CardHeader>
        <CardContent>
          <ul className="text-sm text-[var(--text-secondary)] space-y-2">
            <li>• {t("account.sessions.tips.unrecognized")}</li>
            <li>• {t("account.sessions.tips.sharedDevices")}</li>
            <li>• {t("account.sessions.tips.mfa")}</li>
            <li>• {t("account.sessions.tips.passwords")}</li>
          </ul>
        </CardContent>
      </Card>
    </div>
  );
}
