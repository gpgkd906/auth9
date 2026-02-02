import { Outlet } from "react-router";

/**
 * Layout component for tenant routes.
 * Renders child routes like:
 * - /dashboard/tenants (index)
 * - /dashboard/tenants/:tenantId/invitations
 * - /dashboard/tenants/:tenantId/webhooks
 */
export default function TenantsLayout() {
  return <Outlet />;
}
