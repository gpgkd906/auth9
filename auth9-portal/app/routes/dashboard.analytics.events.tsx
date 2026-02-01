import type { LoaderFunctionArgs } from "react-router";
import { useLoaderData, Link } from "react-router";
import { Card, CardContent, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { analyticsApi, type LoginEvent } from "~/services/api";
import {
  CheckCircledIcon,
  CrossCircledIcon,
  LockClosedIcon,
  PersonIcon,
} from "@radix-ui/react-icons";

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const perPage = 50;

  try {
    const response = await analyticsApi.listEvents(page, perPage);
    return { events: response.data, pagination: response.pagination };
  } catch {
    return {
      events: [],
      pagination: { page: 1, per_page: perPage, total: 0, total_pages: 0 },
      error: "Failed to load events",
    };
  }
}

function getEventIcon(eventType: string) {
  switch (eventType) {
    case "success":
    case "social":
      return <CheckCircledIcon className="h-4 w-4 text-green-600" />;
    case "failed_password":
    case "failed_mfa":
      return <CrossCircledIcon className="h-4 w-4 text-red-600" />;
    case "locked":
      return <LockClosedIcon className="h-4 w-4 text-orange-600" />;
    default:
      return <PersonIcon className="h-4 w-4 text-gray-600" />;
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
      return "bg-green-100 text-green-700";
    case "failed_password":
    case "failed_mfa":
      return "bg-red-100 text-red-700";
    case "locked":
      return "bg-orange-100 text-orange-700";
    default:
      return "bg-gray-100 text-gray-700";
  }
}

function formatDate(dateString: string) {
  const date = new Date(dateString);
  return date.toLocaleString();
}

export default function LoginEventsPage() {
  const { events, pagination, error } = useLoaderData<typeof loader>();

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Login Events</h1>
          <p className="text-gray-500">
            Detailed log of all authentication attempts
          </p>
        </div>
        <Link to="/dashboard/analytics">
          <Button variant="outline">← Back to Analytics</Button>
        </Link>
      </div>

      {error && (
        <div className="text-sm text-red-600 bg-red-50 p-3 rounded-md">{error}</div>
      )}

      {/* Events Table */}
      <Card>
        <CardHeader>
          <CardTitle className="text-lg">
            Recent Events
            <span className="ml-2 text-sm font-normal text-gray-500">
              {pagination.total.toLocaleString()} total
            </span>
          </CardTitle>
        </CardHeader>
        <CardContent>
          {events.length === 0 ? (
            <p className="text-gray-500 text-center py-8">No events found</p>
          ) : (
            <div className="overflow-x-auto">
              <table className="min-w-full divide-y divide-gray-100 text-sm">
                <thead className="bg-gray-50">
                  <tr>
                    <th className="px-4 py-3 text-left font-medium text-gray-500">
                      Time
                    </th>
                    <th className="px-4 py-3 text-left font-medium text-gray-500">
                      Event
                    </th>
                    <th className="px-4 py-3 text-left font-medium text-gray-500">
                      User
                    </th>
                    <th className="px-4 py-3 text-left font-medium text-gray-500">
                      IP Address
                    </th>
                    <th className="px-4 py-3 text-left font-medium text-gray-500">
                      Device
                    </th>
                    <th className="px-4 py-3 text-left font-medium text-gray-500">
                      Details
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-100">
                  {events.map((event: LoginEvent) => (
                    <tr key={event.id} className="hover:bg-gray-50">
                      <td className="px-4 py-3 whitespace-nowrap text-gray-500">
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
                      <td className="px-4 py-3 whitespace-nowrap text-gray-500">
                        {event.ip_address || "-"}
                      </td>
                      <td className="px-4 py-3 whitespace-nowrap">
                        <span className="capitalize text-gray-600">
                          {event.device_type || "-"}
                        </span>
                      </td>
                      <td className="px-4 py-3">
                        {event.failure_reason && (
                          <span className="text-red-600 text-xs">
                            {event.failure_reason}
                          </span>
                        )}
                        {event.location && (
                          <span className="text-gray-500 text-xs">
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
              <div className="text-sm text-gray-500">
                Page {pagination.page} of {pagination.total_pages}
              </div>
              <div className="flex gap-2">
                {pagination.page > 1 && (
                  <Link to={`?page=${pagination.page - 1}`}>
                    <Button variant="outline" size="sm">
                      Previous
                    </Button>
                  </Link>
                )}
                {pagination.page < pagination.total_pages && (
                  <Link to={`?page=${pagination.page + 1}`}>
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
