import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import AuditLogsPage, { loader } from "~/routes/dashboard.audit-logs";
import { auditApi } from "~/services/api";

// Mock audit API
vi.mock("~/services/api", () => ({
    auditApi: { list: vi.fn() },
}));

describe("Audit Logs Page", () => {
    const mockAuditLogs = {
        data: [
            {
                id: 1,
                action: "CREATE",
                resource_type: "tenant",
                resource_id: "tenant-123",
                actor_id: "user-1",
                actor_email: "user1@example.com",
                actor_display_name: "User One",
                created_at: new Date().toISOString(),
            },
            {
                id: 2,
                action: "UPDATE",
                resource_type: "user",
                resource_id: "user-456",
                actor_id: "admin",
                actor_email: "admin@example.com",
                actor_display_name: "Admin User",
                created_at: new Date().toISOString(),
            },
            {
                id: 3,
                action: "DELETE",
                resource_type: "role",
                resource_id: undefined,
                actor_id: undefined,
                actor_email: undefined,
                actor_display_name: undefined,
                created_at: new Date().toISOString(),
            },
        ],
        pagination: {
            total: 150,
            page: 1,
            per_page: 50,
            total_pages: 3,
        },
    };

    it("renders audit logs table with data", async () => {
        vi.mocked(auditApi.list).mockResolvedValue(mockAuditLogs);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/audit-logs",
                Component: AuditLogsPage,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/audit-logs"]} />);

        await waitFor(() => {
            expect(screen.getByText("Audit Logs")).toBeInTheDocument();
            expect(screen.getByText("Audit Trail")).toBeInTheDocument();
        });
    });

    it("displays pagination info", async () => {
        vi.mocked(auditApi.list).mockResolvedValue(mockAuditLogs);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/audit-logs",
                Component: AuditLogsPage,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/audit-logs"]} />);

        await waitFor(() => {
            expect(screen.getByText(/150 events/)).toBeInTheDocument();
            expect(screen.getByText(/Page 1 of/)).toBeInTheDocument();
        });
    });

    it("renders log entries in table", async () => {
        vi.mocked(auditApi.list).mockResolvedValue(mockAuditLogs);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/audit-logs",
                Component: AuditLogsPage,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/audit-logs"]} />);

        await waitFor(() => {
            // Check actions
            expect(screen.getByText("CREATE")).toBeInTheDocument();
            expect(screen.getByText("UPDATE")).toBeInTheDocument();
            expect(screen.getByText("DELETE")).toBeInTheDocument();
            // Check resource types with IDs
            expect(screen.getByText("tenant:tenant-123")).toBeInTheDocument();
            expect(screen.getByText("user:user-456")).toBeInTheDocument();
            // Check actors (now uses actor_email)
            expect(screen.getByText("user1@example.com")).toBeInTheDocument();
            expect(screen.getByText("admin@example.com")).toBeInTheDocument();
        });
    });

    it("shows empty state when no logs", async () => {
        vi.mocked(auditApi.list).mockResolvedValue({
            data: [],
            pagination: { total: 0, page: 1, per_page: 50, total_pages: 1 },
        });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/audit-logs",
                Component: AuditLogsPage,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/audit-logs"]} />);

        await waitFor(() => {
            expect(screen.getByText("No audit logs found")).toBeInTheDocument();
        });
    });

    it("handles null actor_id gracefully", async () => {
        vi.mocked(auditApi.list).mockResolvedValue(mockAuditLogs);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/audit-logs",
                Component: AuditLogsPage,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/audit-logs"]} />);

        await waitFor(() => {
            // The entry with null actor_id should show "-"
            const dashElements = screen.getAllByText("-");
            expect(dashElements.length).toBeGreaterThan(0);
        });
    });
});
