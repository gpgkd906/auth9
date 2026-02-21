import type { LoaderFunctionArgs, MetaFunction } from "react-router";
import { Link, useLoaderData, useFetcher } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Badge } from "~/components/ui/badge";
import { Switch } from "~/components/ui/switch";
import type { Action } from "@auth9/core";
import { ActionTrigger } from "@auth9/core";
import { getAuth9Client, withService, getTriggers } from "~/lib/auth9-client";
import { FormattedDate } from "~/components/ui/formatted-date";
import { getAccessToken } from "~/services/session.server";
import { useState, useRef } from "react";
import { PlusIcon, MagnifyingGlassIcon, CheckCircledIcon, CrossCircledIcon, ClockIcon } from "@radix-ui/react-icons";

export const meta: MetaFunction = () => {
  return [{ title: "Actions - Auth9" }];
};

export async function loader({ params, request }: LoaderFunctionArgs) {
  const { serviceId } = params;
  if (!serviceId) throw new Error("Service ID is required");
  const accessToken = await getAccessToken(request);

  const url = new URL(request.url);
  const triggerFilter = url.searchParams.get("trigger") as string | null;

  const client = getAuth9Client(accessToken || undefined);
  const api = withService(client, serviceId);

  const actionsRes = await api.actions.list(triggerFilter || undefined);
  const triggersRes = await getTriggers(client);

  return {
    serviceId,
    actions: actionsRes.data,
    triggers: triggersRes.data,
    currentTrigger: triggerFilter,
  };
}

export async function action({ params, request }: { params: Record<string, string | undefined>; request: Request }) {
  const { serviceId } = params;
  if (!serviceId) return Response.json({ error: "Service ID required" }, { status: 400 });
  const accessToken = await getAccessToken(request);

  const formData = await request.formData();
  const intent = formData.get("intent");
  const actionId = formData.get("actionId") as string;

  const client = getAuth9Client(accessToken || undefined);
  const api = withService(client, serviceId);

  try {
    if (intent === "toggle") {
      const enabled = formData.get("enabled") === "true";
      await api.actions.update(actionId, { enabled });
      return { success: true };
    }

    if (intent === "delete") {
      await api.actions.delete(actionId);
      return { success: true };
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown error";
    return Response.json({ error: message }, { status: 400 });
  }

  return Response.json({ error: "Invalid intent" }, { status: 400 });
}

const TRIGGER_LABELS: Record<string, string> = {
  [ActionTrigger.PostLogin]: "Post Login",
  [ActionTrigger.PreUserRegistration]: "Pre Registration",
  [ActionTrigger.PostUserRegistration]: "Post Registration",
  [ActionTrigger.PostChangePassword]: "Post Password Change",
  [ActionTrigger.PostEmailVerification]: "Post Email Verification",
  [ActionTrigger.PreTokenRefresh]: "Pre Token Refresh",
};

export default function ActionsListPage() {
  const { serviceId, actions, triggers, currentTrigger } = useLoaderData<typeof loader>();
  const [searchQuery, setSearchQuery] = useState("");

  // Filter actions by search query
  const filteredActions = actions.filter((action) =>
    action.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    action.description?.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Actions</h1>
          <p className="text-muted-foreground mt-1">
            Manage authentication flow actions with TypeScript
          </p>
        </div>
        <Button asChild>
          <Link to={`/dashboard/services/${serviceId}/actions/new`}>
            <PlusIcon className="mr-2 h-4 w-4" />
            New Action
          </Link>
        </Button>
      </div>

      {/* Filters and Search */}
      <Card>
        <CardHeader>
          <CardTitle>Filters</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Trigger Filter */}
          <div>
            <label className="text-sm font-medium mb-2 block">Trigger Type</label>
            <div className="flex flex-wrap gap-2">
              <Button
                variant={!currentTrigger ? "default" : "outline"}
                size="sm"
                asChild
              >
                <Link to={`/dashboard/services/${serviceId}/actions`}>All</Link>
              </Button>
              {triggers.map((trigger) => (
                <Button
                  key={trigger}
                  variant={currentTrigger === trigger ? "default" : "outline"}
                  size="sm"
                  asChild
                >
                  <Link to={`/dashboard/services/${serviceId}/actions?trigger=${trigger}`}>
                    {TRIGGER_LABELS[trigger]}
                  </Link>
                </Button>
              ))}
            </div>
          </div>

          {/* Search */}
          <div className="relative">
            <MagnifyingGlassIcon className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              type="search"
              placeholder="Search actions..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9"
            />
          </div>
        </CardContent>
      </Card>

      {/* Actions List */}
      {filteredActions.length === 0 ? (
        <Card>
          <CardContent className="py-12">
            <div className="text-center">
              <h3 className="text-lg font-semibold mb-2">No actions found</h3>
              <p className="text-muted-foreground mb-4">
                {searchQuery
                  ? "Try adjusting your search query"
                  : "Get started by creating your first action"}
              </p>
              {!searchQuery && (
                <Button asChild>
                  <Link to={`/dashboard/services/${serviceId}/actions/new`}>
                    <PlusIcon className="mr-2 h-4 w-4" />
                    Create Action
                  </Link>
                </Button>
              )}
            </div>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4">
          {filteredActions.map((action) => (
            <ActionCard key={action.id} action={action} serviceId={serviceId} />
          ))}
        </div>
      )}
    </div>
  );
}

function ActionCard({ action, serviceId }: { action: Action; serviceId: string }) {
  const fetcher = useFetcher();
  const toggleFormRef = useRef<HTMLFormElement>(null);
  const isToggling = fetcher.state !== "idle" && fetcher.formData?.get("intent") === "toggle";
  const isDeleting = fetcher.state !== "idle" && fetcher.formData?.get("intent") === "delete";

  // Optimistic UI
  const enabled = isToggling
    ? fetcher.formData?.get("enabled") === "true"
    : action.enabled;

  const successRate =
    action.executionCount > 0
      ? (((action.executionCount - action.errorCount) / action.executionCount) * 100).toFixed(1)
      : "N/A";

  return (
    <Card className={isDeleting ? "opacity-50" : ""}>
      <CardHeader>
        <div className="flex items-start justify-between">
          <div className="flex-1">
            <div className="flex items-center gap-2 mb-1">
              <CardTitle className="text-lg">
                <Link
                  to={`/dashboard/services/${serviceId}/actions/${action.id}`}
                  className="hover:underline"
                >
                  {action.name}
                </Link>
              </CardTitle>
              <Badge variant={enabled ? "default" : "secondary"}>
                {enabled ? "Enabled" : "Disabled"}
              </Badge>
              {action.strictMode && (
                <Badge variant="destructive">Strict</Badge>
              )}
              <Badge variant="outline">{TRIGGER_LABELS[action.triggerId]}</Badge>
            </div>
            {action.description && (
              <CardDescription>{action.description}</CardDescription>
            )}
          </div>

          <div className="flex items-center gap-2">
            <fetcher.Form method="post" ref={toggleFormRef}>
              <input type="hidden" name="intent" value="toggle" />
              <input type="hidden" name="actionId" value={action.id} />
              <input type="hidden" name="enabled" value={String(!enabled)} />
              <Switch
                checked={enabled}
                disabled={isToggling}
                onCheckedChange={() => {
                  toggleFormRef.current?.requestSubmit();
                }}
              />
            </fetcher.Form>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-4 gap-4 text-sm">
          <div>
            <div className="text-muted-foreground mb-1">Executions</div>
            <div className="font-semibold">{action.executionCount.toLocaleString()}</div>
          </div>
          <div>
            <div className="text-muted-foreground mb-1">Success Rate</div>
            <div className="font-semibold flex items-center gap-1">
              {successRate !== "N/A" && (
                <>
                  {parseFloat(successRate) >= 95 ? (
                    <CheckCircledIcon className="h-4 w-4 text-green-500" />
                  ) : (
                    <CrossCircledIcon className="h-4 w-4 text-red-500" />
                  )}
                </>
              )}
              {successRate}%
            </div>
          </div>
          <div>
            <div className="text-muted-foreground mb-1">Last Executed</div>
            <div className="font-semibold flex items-center gap-1">
              <ClockIcon className="h-4 w-4" />
              {action.lastExecutedAt ? (
                <FormattedDate date={action.lastExecutedAt} />
              ) : (
                "Never"
              )}
            </div>
          </div>
          <div>
            <div className="text-muted-foreground mb-1">Order</div>
            <div className="font-semibold">{action.executionOrder}</div>
          </div>
        </div>

        {action.lastError && (
          <div className="mt-4 p-3 bg-destructive/10 rounded-md">
            <div className="text-sm font-medium text-destructive mb-1">Last Error</div>
            <div className="text-sm text-muted-foreground">{action.lastError}</div>
          </div>
        )}

        <div className="mt-4 flex gap-2">
          <Button asChild variant="outline" size="sm">
            <Link to={`/dashboard/services/${serviceId}/actions/${action.id}`}>
              View Details
            </Link>
          </Button>
          <Button asChild variant="outline" size="sm">
            <Link to={`/dashboard/services/${serviceId}/actions/${action.id}/edit`}>
              Edit
            </Link>
          </Button>
          <fetcher.Form method="post" className="inline">
            <input type="hidden" name="intent" value="delete" />
            <input type="hidden" name="actionId" value={action.id} />
            <Button
              type="submit"
              variant="destructive"
              size="sm"
              disabled={isDeleting}
              onClick={(e) => {
                if (!confirm(`Are you sure you want to delete "${action.name}"?`)) {
                  e.preventDefault();
                }
              }}
            >
              {isDeleting ? "Deleting..." : "Delete"}
            </Button>
          </fetcher.Form>
        </div>
      </CardContent>
    </Card>
  );
}
