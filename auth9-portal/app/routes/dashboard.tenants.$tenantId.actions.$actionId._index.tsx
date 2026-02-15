import type { LoaderFunctionArgs, MetaFunction } from "react-router";
import { Link, useLoaderData } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Badge } from "~/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "~/components/ui/tabs";
import type { Action, ActionExecution, ActionStats } from "@auth9/core";
import { ActionTrigger } from "@auth9/core";
import { getAuth9Client, withTenant } from "~/lib/auth9-client";
import { getAccessToken } from "~/services/session.server";
import { ArrowLeftIcon, CheckCircledIcon, CrossCircledIcon, ClockIcon, CodeIcon, ActivityLogIcon } from "@radix-ui/react-icons";

export const meta: MetaFunction<typeof loader> = ({ data }) => {
  return [{ title: `${data?.action.name || "Action"} - Auth9` }];
};

export async function loader({ params, request }: LoaderFunctionArgs) {
  const { tenantId, actionId } = params;
  if (!tenantId || !actionId) throw new Error("Tenant ID and Action ID are required");
  const accessToken = await getAccessToken(request);

  const client = getAuth9Client(accessToken || undefined);
  const api = withTenant(client, tenantId);

  const [actionRes, logsRes, statsRes] = await Promise.all([
    api.actions.get(actionId),
    api.actions.logs({ actionId, limit: 50 }),
    api.actions.stats(actionId).catch(() => null),
  ]);

  return {
    tenantId,
    action: actionRes.data,
    logs: logsRes.data,
    stats: statsRes?.data || null,
  };
}

const TRIGGER_LABELS: Record<string, string> = {
  [ActionTrigger.PostLogin]: "Post Login",
  [ActionTrigger.PreUserRegistration]: "Pre Registration",
  [ActionTrigger.PostUserRegistration]: "Post Registration",
  [ActionTrigger.PostChangePassword]: "Post Password Change",
  [ActionTrigger.PostEmailVerification]: "Post Email Verification",
  [ActionTrigger.PreTokenRefresh]: "Pre Token Refresh",
};

export default function ActionDetailPage() {
  const { tenantId, action, logs, stats } = useLoaderData<typeof loader>();

  // Calculate success rate
  const successRate = stats && stats.executionCount > 0
    ? ((stats.executionCount - stats.errorCount) / stats.executionCount) * 100
    : 0;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="icon" asChild>
          <Link to={`/dashboard/tenants/${tenantId}/actions`}>
            <ArrowLeftIcon className="h-4 w-4" />
          </Link>
        </Button>
        <div className="flex-1">
          <div className="flex items-center gap-2 mb-1">
            <h1 className="text-3xl font-bold">{action.name}</h1>
            <Badge variant={action.enabled ? "default" : "secondary"}>
              {action.enabled ? "Enabled" : "Disabled"}
            </Badge>
            {action.strictMode && (
              <Badge variant="destructive">Strict Mode</Badge>
            )}
            <Badge variant="outline">{TRIGGER_LABELS[action.triggerId]}</Badge>
          </div>
          {action.description && (
            <p className="text-muted-foreground">{action.description}</p>
          )}
        </div>
        <div className="flex gap-2">
          <Button asChild variant="outline">
            <Link to={`/dashboard/tenants/${tenantId}/actions/${action.id}/edit`}>
              Edit
            </Link>
          </Button>
        </div>
      </div>

      {/* Statistics */}
      {stats && (
        <div className="grid grid-cols-4 gap-4">
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">Total Executions</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">{stats.executionCount.toLocaleString()}</div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">Success Rate</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold flex items-center gap-2">
                {successRate >= 95 ? (
                  <CheckCircledIcon className="h-5 w-5 text-green-500" />
                ) : (
                  <CrossCircledIcon className="h-5 w-5 text-red-500" />
                )}
                {successRate.toFixed(1)}%
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">Avg Duration</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold flex items-center gap-2">
                <ClockIcon className="h-5 w-5" />
                {stats.avgDurationMs}ms
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">Last 24h</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">{stats.last24hCount.toLocaleString()}</div>
              <p className="text-xs text-muted-foreground mt-1">executions</p>
            </CardContent>
          </Card>
        </div>
      )}

      {/* Tabs */}
      <Tabs defaultValue="script" className="space-y-4">
        <TabsList>
          <TabsTrigger value="script">
            <CodeIcon className="mr-2 h-4 w-4" />
            Script
          </TabsTrigger>
          <TabsTrigger value="logs">
            <ActivityLogIcon className="mr-2 h-4 w-4" />
            Execution Logs ({logs.length})
          </TabsTrigger>
        </TabsList>

        {/* Script Tab */}
        <TabsContent value="script">
          <Card>
            <CardHeader>
              <CardTitle>TypeScript Code</CardTitle>
              <CardDescription>
                This code is executed on every {TRIGGER_LABELS[action.triggerId]} event
              </CardDescription>
            </CardHeader>
            <CardContent>
              <pre className="p-4 bg-muted rounded-md overflow-x-auto">
                <code className="text-sm">{action.script}</code>
              </pre>

              <div className="grid grid-cols-2 gap-4 mt-4">
                <div>
                  <div className="text-sm font-medium mb-1">Execution Order</div>
                  <div className="text-2xl font-bold">{action.executionOrder}</div>
                </div>
                <div>
                  <div className="text-sm font-medium mb-1">Timeout</div>
                  <div className="text-2xl font-bold">{action.timeoutMs}ms</div>
                </div>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        {/* Logs Tab */}
        <TabsContent value="logs">
          <Card>
            <CardHeader>
              <CardTitle>Execution Logs</CardTitle>
              <CardDescription>
                Recent action executions (last 50)
              </CardDescription>
            </CardHeader>
            <CardContent>
              {logs.length === 0 ? (
                <div className="text-center py-8 text-muted-foreground">
                  No executions yet
                </div>
              ) : (
                <div className="space-y-2">
                  {logs.map((log) => (
                    <ExecutionLogCard key={log.id} log={log} />
                  ))}
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>

      {/* Metadata */}
      <Card>
        <CardHeader>
          <CardTitle>Metadata</CardTitle>
        </CardHeader>
        <CardContent className="grid grid-cols-2 gap-4 text-sm">
          <div>
            <div className="text-muted-foreground mb-1">Action ID</div>
            <code className="text-xs bg-muted px-2 py-1 rounded">{action.id}</code>
          </div>
          <div>
            <div className="text-muted-foreground mb-1">Tenant ID</div>
            <code className="text-xs bg-muted px-2 py-1 rounded">{action.tenantId}</code>
          </div>
          <div>
            <div className="text-muted-foreground mb-1">Created At</div>
            <div>{new Date(action.createdAt).toLocaleString()}</div>
          </div>
          <div>
            <div className="text-muted-foreground mb-1">Updated At</div>
            <div>{new Date(action.updatedAt).toLocaleString()}</div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

function ExecutionLogCard({ log }: { log: ActionExecution }) {
  return (
    <div
      className={`p-3 rounded-md border ${
        log.success ? "bg-green-50 border-green-200" : "bg-red-50 border-red-200"
      }`}
    >
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          {log.success ? (
            <CheckCircledIcon className="h-4 w-4 text-green-600" />
          ) : (
            <CrossCircledIcon className="h-4 w-4 text-red-600" />
          )}
          <span className="font-semibold text-sm">
            {log.success ? "Success" : "Failed"}
          </span>
        </div>
        <div className="flex items-center gap-4 text-xs text-muted-foreground">
          <span>{log.durationMs}ms</span>
          <span>{new Date(log.executedAt).toLocaleString()}</span>
        </div>
      </div>

      {log.errorMessage && (
        <div className="text-sm text-red-700 font-mono bg-white/50 p-2 rounded">
          {log.errorMessage}
        </div>
      )}

      {log.userId && (
        <div className="text-xs text-muted-foreground mt-2">
          User ID: <code className="bg-white/50 px-1 py-0.5 rounded">{log.userId}</code>
        </div>
      )}
    </div>
  );
}
