import type { LoaderFunctionArgs } from "react-router";
import { useLoaderData, Link, redirect, useNavigate } from "react-router";
import { Card, CardContent, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Form } from "react-router";
import { analyticsApi, type LoginEvent } from "~/services/api";
import { getAccessToken } from "~/services/session.server";
import { useState } from "react";
import {
  CheckCircledIcon,
  CrossCircledIcon,
  LockClosedIcon,
  PersonIcon,
} from "@radix-ui/react-icons";

export async function loader({ request }: LoaderFunctionArgs) {
  const accessToken = await getAccessToken(request);
  if (!accessToken) {
    throw redirect("/login");
  }

  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const perPage = 20;
  const email = url.searchParams.get("email") || undefined;

  try {
    const response = await analyticsApi.listEvents(page, perPage, email, accessToken);
    return { events: response.data, pagination: response.pagination, email };
  } catch {
    return {
      events: [],
      pagination: { page: 1, per_page: perPage, total: 0, total_pages: 0 },
      error: "Failed to load events",
      email,
    };
  }
}

function getEventIcon(eventType: string) {
  switch (eventType) {
    case "success":
    case "social":
      return <CheckCircledIcon className="h-4 w-4 text-[var(--accent-green)]" />;
    case "failed_password":
    case "failed_mfa":
      return <CrossCircledIcon className="h-4 w-4 text-[var(--accent-red)]" />;
    case "locked":
      return <LockClosedIcon className="h-4 w-4 text-[var(--accent-orange)]" />;
    default:
      return <PersonIcon className="h-4 w-4 text-[var(--text-secondary)]" />;
  }
}

function getEventLabel(eventType: string) {
  switch (eventType) {
    case "success":
      return "Login Success";
    case "social":
      return "Social Login";
    case "failed_password":
      return "Wrong Password";
    case "failed_mfa":
      return "MFA Failed";
    case "locked":
      return "Account Locked";
    default:
      return eventType;
  }
}

function getEventBadgeColor(eventType: string) {
  switch (eventType) {
    case "success":
    case "social":
      return "bg-green-100 text-[var(--accent-green)]";
    case "failed_password":
    case "failed_mfa":
      return "bg-red-100 text-red-700";
    case "locked":
      return "bg-orange-100 text-orange-700";
    default:
      return "bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)]";
  }
}

function formatDate(dateString: string) {
  const date = new Date(dateString);
  return date.toLocaleString();
}

export default function LoginEventsPage() {
  const { events, pagination, error, email } = useLoaderData<typeof loader>();
  const navigate = useNavigate();
  const [emailFilter, setEmailFilter] = useState(email || "");

  const handleFilterSubmit = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const params = new URLSearchParams();
    if (emailFilter.trim()) {
      params.set("email", emailFilter);
    }
    params.set("page", "1");
    navigate(`/dashboard/analytics/events?${params.toString()}`);
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Login Events</h1>
          <p className="text-[var(--text-secondary)]">
            Detailed log of all authentication attempts
          </p>
        </div>
        <Link to="/dashboard/analytics">
          <Button variant="outline">← Back to Analytics</Button>
        </Link>
      </div>

      {error && (
        <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md">{error}</div>
      )}

      {/* Filter */}
      <Form onSubmit={handleFilterSubmit} className="flex gap-2">
        <Input
          type="email"
          placeholder="Filter by email address..."
          value={emailFilter}
          onChange={(e) => setEmailFilter(e.target.value)}
          className="flex-1"
        />
        <Button type="submit" variant="outline">Filter</Button>
      </Form>

      {/* Events Table */}
      <Card>
        <CardHeader>
          <CardTitle className="text-lg">
            Recent Events
            <span className="ml-2 text-sm font-normal text-[var(--text-secondary)]">
              {pagination.total.toLocaleString()} total
            </span>
          </CardTitle>
        </CardHeader>
        <CardContent>
          {events.length === 0 ? (
            <p className="text-[var(--text-secondary)] text-center py-8">No events found</p>
          ) : (
            <div className="overflow-x-auto">
              <table className="min-w-full divide-y divide-[var(--glass-border-subtle)] text-sm">
                <thead className="bg-[var(--sidebar-item-hover)]">
                  <tr>
                    <th className="px-4 py-3 text-left font-medium text-[var(--text-secondary)]">
                      Time
                    </th>
                    <th className="px-4 py-3 text-left font-medium text-[var(--text-secondary)]">
                      Event
                    </th>
                    <th className="px-4 py-3 text-left font-medium text-[var(--text-secondary)]">
                      User
                    </th>
                    <th className="px-4 py-3 text-left font-medium text-[var(--text-secondary)]">
                      IP Address
                    </th>
                    <th className="px-4 py-3 text-left font-medium text-[var(--text-secondary)]">
                      Device
                    </th>
                    <th className="px-4 py-3 text-left font-medium text-[var(--text-secondary)]">
                      Details
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-[var(--glass-border-subtle)]">
                  {events.map((event: LoginEvent) => (
                    <tr key={event.id} className="hover:bg-[var(--sidebar-item-hover)]">
                      <td className="px-4 py-3 whitespace-nowrap text-[var(--text-secondary)]">
                        {formatDate(event.created_at)}
                      </td>
                      <td className="px-4 py-3 whitespace-nowrap">
                        <span
                          className={`inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium ${getEventBadgeColor(
                            event.event_type
                          )}`}
                        >
                          {getEventIcon(event.event_type)}
                          {getEventLabel(event.event_type)}
                        </span>
                      </td>
                      <td className="px-4 py-3">
                        <div className="max-w-[200px] truncate">
                          {event.email || event.user_id || "Unknown"}
                        </div>
                      </td>
                      <td className="px-4 py-3 whitespace-nowrap text-[var(--text-secondary)]">
                        {event.ip_address || "-"}
                      </td>
                      <td className="px-4 py-3 whitespace-nowrap">
                        <span className="capitalize text-[var(--text-secondary)]">
                          {event.device_type || "-"}
                        </span>
                      </td>
                      <td className="px-4 py-3">
                        {event.failure_reason && (
                          <span className="text-[var(--accent-red)] text-xs">
                            {event.failure_reason}
                          </span>
                        )}
                        {event.location && (
                          <span className="text-[var(--text-secondary)] text-xs">
                            {event.location}
                          </span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}

          {/* Pagination */}
          {pagination.total_pages > 1 && (
            <div className="flex items-center justify-between mt-4 pt-4 border-t">
              <div className="text-sm text-[var(--text-secondary)]">
                Page {pagination.page} of {pagination.total_pages}
              </div>
              <div className="flex gap-2">
                {pagination.page > 1 && (
                  <Link to={`?page=${pagination.page - 1}${email ? `&email=${encodeURIComponent(email)}` : ""}`}>
                    <Button variant="outline" size="sm">
                      Previous
                    </Button>
                  </Link>
                )}
                {pagination.page < pagination.total_pages && (
                  <Link to={`?page=${pagination.page + 1}${email ? `&email=${encodeURIComponent(email)}` : ""}`}>
                    <Button variant="outline" size="sm">
                      Next
                    </Button>
                  </Link>
                )}
              </div>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
