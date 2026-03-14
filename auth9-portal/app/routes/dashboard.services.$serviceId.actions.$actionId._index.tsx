import { useState } from "react";
import type { LoaderFunctionArgs, MetaFunction } from "react-router";
import { Link, useLoaderData } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Badge } from "~/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "~/components/ui/tabs";
import type { ActionExecution } from "@auth9/core";
import { getAuth9Client, withService } from "~/lib/auth9-client";
import { FormattedDate } from "~/components/ui/formatted-date";
import { getAccessToken } from "~/services/session.server";
import { ArrowLeftIcon, CheckCircledIcon, CrossCircledIcon, ClockIcon, CodeIcon, ActivityLogIcon, ChevronDownIcon, ChevronRightIcon } from "@radix-ui/react-icons";
import { CodeBlock } from "~/components/services/copyable-value";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";
import { getActionTriggerLabel } from "~/lib/service-actions";

export const meta: MetaFunction<typeof loader> = ({ data, matches }) => {
  const locale = resolveMetaLocale(matches);
  return buildMeta(locale, "serviceActions.detailMetaTitle", undefined, {
    actionName: data?.action.name || translate(locale, "serviceActions.title"),
  });
};

export async function loader({ params, request }: LoaderFunctionArgs) {
  const { serviceId, actionId } = params;
  const locale = await resolveLocale(request);
  if (!serviceId || !actionId) throw new Error(translate(locale, "serviceActions.errors.serviceAndActionIdRequired"));
  const accessToken = await getAccessToken(request);

  const client = getAuth9Client(accessToken || undefined);
  const api = withService(client, serviceId);

  const [actionRes, logsRes, statsRes] = await Promise.all([
    api.actions.get(actionId),
    api.actions.logs({ actionId, limit: 50 }),
    api.actions.stats(actionId).catch(() => null),
  ]);

  return {
    locale,
    serviceId,
    action: actionRes.data,
    logs: logsRes.data,
    stats: statsRes?.data || null,
  };
}

export default function ActionDetailPage() {
  const { serviceId, action, logs, stats, locale } = useLoaderData<typeof loader>();
  const { t, i18n } = useI18n();
  const effectiveLocale = (locale || i18n.resolvedLanguage || "zh-CN") as "zh-CN" | "en-US";
  const successRate = stats && stats.executionCount > 0 ? ((stats.executionCount - stats.errorCount) / stats.executionCount) * 100 : 0;

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="icon" asChild>
          <Link to={`/dashboard/services/${serviceId}/actions`} aria-label={t("common.buttons.back")}>
            <ArrowLeftIcon className="h-4 w-4" />
          </Link>
        </Button>
        <div className="flex-1">
          <div className="flex items-center gap-2 mb-1">
            <h1 className="text-3xl font-bold">{action.name}</h1>
            <Badge variant={action.enabled ? "default" : "secondary"}>{action.enabled ? t("serviceActions.enabled") : t("serviceActions.disabled")}</Badge>
            {action.strictMode && <Badge variant="destructive">{t("serviceActions.strictMode")}</Badge>}
            <Badge variant="outline">{getActionTriggerLabel(effectiveLocale, action.triggerId)}</Badge>
          </div>
          {action.description && <p className="text-muted-foreground">{action.description}</p>}
        </div>
        <div className="flex gap-2">
          <Button asChild variant="outline">
            <Link to={`/dashboard/services/${serviceId}/actions/${action.id}/edit`}>{t("serviceActions.edit")}</Link>
          </Button>
        </div>
      </div>

      {stats && (
        <div className="grid grid-cols-4 gap-4">
          <Card>
            <CardHeader className="pb-2"><CardTitle className="text-sm font-medium">{t("serviceActions.statistics.totalExecutions")}</CardTitle></CardHeader>
            <CardContent><div className="text-2xl font-bold">{stats.executionCount.toLocaleString()}</div></CardContent>
          </Card>
          <Card>
            <CardHeader className="pb-2"><CardTitle className="text-sm font-medium">{t("serviceActions.successRate")}</CardTitle></CardHeader>
            <CardContent>
              <div className="text-2xl font-bold flex items-center gap-2">
                {successRate >= 95 ? <CheckCircledIcon className="h-5 w-5 text-green-500" /> : <CrossCircledIcon className="h-5 w-5 text-red-500" />}
                {successRate.toFixed(1)}%
              </div>
            </CardContent>
          </Card>
          <Card>
            <CardHeader className="pb-2"><CardTitle className="text-sm font-medium">{t("serviceActions.statistics.avgDuration")}</CardTitle></CardHeader>
            <CardContent><div className="text-2xl font-bold flex items-center gap-2"><ClockIcon className="h-5 w-5" />{stats.avgDurationMs}ms</div></CardContent>
          </Card>
          <Card>
            <CardHeader className="pb-2"><CardTitle className="text-sm font-medium">{t("serviceActions.statistics.last24h")}</CardTitle></CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">{stats.last24hCount.toLocaleString()}</div>
              <p className="text-xs text-muted-foreground mt-1">{t("serviceActions.statistics.executionsSuffix")}</p>
            </CardContent>
          </Card>
        </div>
      )}

      <Tabs defaultValue="script" className="space-y-4">
        <TabsList>
          <TabsTrigger value="script"><CodeIcon className="mr-2 h-4 w-4" />{t("serviceActions.tabs.script")}</TabsTrigger>
          <TabsTrigger value="logs"><ActivityLogIcon className="mr-2 h-4 w-4" />{t("serviceActions.tabs.logs", { count: logs.length })}</TabsTrigger>
        </TabsList>

        <TabsContent value="script">
          <Card>
            <CardHeader>
              <CardTitle>{t("serviceActions.scriptCode")}</CardTitle>
              <CardDescription>{t("serviceActions.scriptExecutedOn", { trigger: getActionTriggerLabel(effectiveLocale, action.triggerId) })}</CardDescription>
            </CardHeader>
            <CardContent>
              <CodeBlock>{action.script}</CodeBlock>
              <div className="grid grid-cols-2 gap-4 mt-4">
                <div><div className="text-sm font-medium mb-1">{t("serviceActions.executionOrder")}</div><div className="text-2xl font-bold">{action.executionOrder}</div></div>
                <div><div className="text-sm font-medium mb-1">{t("serviceActions.timeout")}</div><div className="text-2xl font-bold">{action.timeoutMs}ms</div></div>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="logs">
          <Card>
            <CardHeader>
              <CardTitle>{t("serviceActions.executionLogs")}</CardTitle>
              <CardDescription>{t("serviceActions.executionLogsDescription")}</CardDescription>
            </CardHeader>
            <CardContent>
              {logs.length === 0 ? <div className="text-center py-8 text-muted-foreground">{t("serviceActions.noExecutions")}</div> : <div className="space-y-2">{logs.map((log) => <ExecutionLogCard key={log.id} log={log} locale={effectiveLocale} />)}</div>}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>

      <Card>
        <CardHeader><CardTitle>{t("serviceActions.metadata")}</CardTitle></CardHeader>
        <CardContent className="grid grid-cols-2 gap-4 text-sm">
          <div><div className="text-muted-foreground mb-1">{t("serviceActions.actionId")}</div><code className="text-xs bg-muted px-2 py-1 rounded">{action.id}</code></div>
          <div><div className="text-muted-foreground mb-1">{t("serviceActions.serviceId")}</div><code className="text-xs bg-muted px-2 py-1 rounded">{action.serviceId}</code></div>
          <div><div className="text-muted-foreground mb-1">{t("serviceActions.createdAt")}</div><div><FormattedDate date={action.createdAt} /></div></div>
          <div><div className="text-muted-foreground mb-1">{t("serviceActions.updatedAt")}</div><div><FormattedDate date={action.updatedAt} /></div></div>
        </CardContent>
      </Card>
    </div>
  );
}

function ExecutionLogCard({ log, locale }: { log: ActionExecution; locale: "zh-CN" | "en-US" }) {
  const [expanded, setExpanded] = useState(false);
  const { t } = useI18n();

  return (
    <div className={`rounded-md border ${log.success ? "bg-green-50 border-green-200" : "bg-red-50 border-red-200"}`}>
      <button type="button" className="w-full p-3 text-left cursor-pointer" onClick={() => setExpanded(!expanded)}>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            {expanded ? <ChevronDownIcon className="h-4 w-4 text-muted-foreground" /> : <ChevronRightIcon className="h-4 w-4 text-muted-foreground" />}
            {log.success ? <CheckCircledIcon className="h-4 w-4 text-green-600" /> : <CrossCircledIcon className="h-4 w-4 text-red-600" />}
            <span className="font-semibold text-sm">{log.success ? t("serviceActions.success") : t("serviceActions.failed")}</span>
            {log.errorMessage && !expanded && <span className="text-xs text-red-600 truncate max-w-[300px]"> - {log.errorMessage}</span>}
          </div>
          <div className="flex items-center gap-4 text-xs text-muted-foreground"><span>{log.durationMs}ms</span><FormattedDate date={log.executedAt} /></div>
        </div>
      </button>

      {expanded && (
        <div className="px-3 pb-3 space-y-2 border-t border-inherit pt-2">
          {log.errorMessage && (
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1">{t("serviceActions.errorMessage")}</div>
              <div className="text-sm text-red-700 font-mono bg-white/50 p-2 rounded whitespace-pre-wrap break-all">{log.errorMessage}</div>
            </div>
          )}

          <div className="grid grid-cols-2 gap-2 text-xs">
            <div><span className="text-muted-foreground">{t("serviceActions.executionId")}:</span> <code className="bg-white/50 px-1 py-0.5 rounded">{log.id}</code></div>
            <div><span className="text-muted-foreground">{t("serviceActions.triggerId")}:</span> <code className="bg-white/50 px-1 py-0.5 rounded">{getActionTriggerLabel(locale, log.triggerId)}</code></div>
            <div><span className="text-muted-foreground">{t("serviceActions.duration")}:</span> <span>{log.durationMs}ms</span></div>
            <div><span className="text-muted-foreground">{t("serviceActions.executedAt")}:</span> <FormattedDate date={log.executedAt} /></div>
            {log.userId && <div className="col-span-2"><span className="text-muted-foreground">{t("serviceActions.userId")}:</span> <code className="bg-white/50 px-1 py-0.5 rounded">{log.userId}</code></div>}
          </div>
        </div>
      )}
    </div>
  );
}
