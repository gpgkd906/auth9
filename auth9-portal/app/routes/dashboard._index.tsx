import { Card, CardContent, CardHeader, CardTitle } from "~/components/ui/card";

export default function DashboardIndex() {
  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-3xl font-bold text-gray-900">Dashboard</h1>
        <p className="mt-2 text-gray-600">
          Welcome to Auth9. Here&apos;s an overview of your identity service.
        </p>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <StatsCard title="Total Tenants" value="12" change="+2 this month" />
        <StatsCard title="Active Users" value="1,284" change="+124 this month" />
        <StatsCard title="Services" value="8" change="+1 this month" />
        <StatsCard title="Auth Requests" value="45.2K" change="+12% this week" />
      </div>

      {/* Recent Activity */}
      <Card>
        <CardHeader>
          <CardTitle>Recent Activity</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            {recentActivity.map((activity, index) => (
              <div key={index} className="flex items-center gap-4 py-3 border-b border-gray-100 last:border-0">
                <div className={`w-2 h-2 rounded-full ${activity.color}`} />
                <div className="flex-1">
                  <p className="text-sm text-gray-900">{activity.message}</p>
                  <p className="text-xs text-gray-500">{activity.time}</p>
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

function StatsCard({ title, value, change }: { title: string; value: string; change: string }) {
  return (
    <Card>
      <CardContent className="pt-6">
        <p className="text-sm font-medium text-gray-500">{title}</p>
        <p className="mt-2 text-3xl font-bold text-gray-900">{value}</p>
        <p className="mt-1 text-sm text-apple-green">{change}</p>
      </CardContent>
    </Card>
  );
}

const recentActivity = [
  { message: "New user registered: alice@acme.com", time: "2 minutes ago", color: "bg-apple-green" },
  { message: "Tenant 'Acme Corp' updated settings", time: "15 minutes ago", color: "bg-apple-blue" },
  { message: "Service 'api-gateway' credentials rotated", time: "1 hour ago", color: "bg-apple-orange" },
  { message: "Role 'Admin' permissions updated", time: "3 hours ago", color: "bg-apple-purple" },
  { message: "New tenant created: 'Stark Industries'", time: "5 hours ago", color: "bg-apple-green" },
];
