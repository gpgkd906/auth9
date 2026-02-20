import { createRoutesStub } from "react-router";
import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import AbacPoliciesPage, { action, loader } from "~/routes/dashboard.abac";
import { abacApi } from "~/services/api";
import { getAccessToken, getSession } from "~/services/session.server";

vi.mock("~/services/api", () => ({
  abacApi: {
    listPolicies: vi.fn(),
    createDraft: vi.fn(),
    updateDraft: vi.fn(),
    publish: vi.fn(),
    rollback: vi.fn(),
    simulate: vi.fn(),
  },
}));

vi.mock("~/services/session.server", () => ({
  getAccessToken: vi.fn(),
  getSession: vi.fn(),
}));

describe("ABAC Policies Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getSession).mockResolvedValue({ activeTenantId: "tenant-1" } as Awaited<ReturnType<typeof getSession>>);
    vi.mocked(getAccessToken).mockResolvedValue("test-token");
  });

  it("renders page title and current mode", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/abac",
        Component: AbacPoliciesPage,
        loader: () => ({
          tenantId: "tenant-1",
          payload: {
            policy_set: {
              policy_set_id: "set-1",
              tenant_id: "tenant-1",
              mode: "shadow",
              published_version_id: "v2",
              published_version_no: 2,
            },
            versions: [
              {
                id: "v2",
                policy_set_id: "set-1",
                version_no: 2,
                status: "published",
                change_note: "Enable deny after office hours",
                created_by: "user-1",
                created_at: new Date().toISOString(),
                published_at: new Date().toISOString(),
              },
            ],
          },
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/abac"]} />);

    expect(await screen.findByText("ABAC Policies")).toBeInTheDocument();
    expect(screen.getByText("shadow")).toBeInTheDocument();
    expect(screen.getByText(/v2 \(v2\)/)).toBeInTheDocument();
  });

  it("loader resolves tenant context and fetches policies", async () => {
    vi.mocked(abacApi.listPolicies).mockResolvedValue({
      data: { policy_set: null, versions: [] },
    });

    const request = new Request("http://localhost/dashboard/abac");
    const result = await loader({ request, params: {}, context: {} } as never);

    expect(abacApi.listPolicies).toHaveBeenCalledWith("tenant-1", "test-token");
    expect(result).toEqual({
      tenantId: "tenant-1",
      payload: { policy_set: null, versions: [] },
    });
  });

  it("action create_draft calls createDraft", async () => {
    vi.mocked(abacApi.createDraft).mockResolvedValue({
      data: { id: "v1", policy_set_id: "set-1", version_no: 1, status: "draft" },
    });

    const formData = new FormData();
    formData.set("intent", "create_draft");
    formData.set("change_note", "first draft");
    formData.set("policy_json", JSON.stringify({ rules: [] }));
    const request = new Request("http://localhost/dashboard/abac", { method: "POST", body: formData });

    const result = await action({ request, params: {}, context: {} } as never);

    expect(abacApi.createDraft).toHaveBeenCalledWith(
      "tenant-1",
      { policy: { rules: [] }, change_note: "first draft" },
      "test-token"
    );
    expect(result).toEqual({ success: true, intent: "create_draft" });
  });

  it("action update_draft calls updateDraft", async () => {
    vi.mocked(abacApi.updateDraft).mockResolvedValue({ message: "ABAC draft policy updated" });

    const formData = new FormData();
    formData.set("intent", "update_draft");
    formData.set("version_id", "v2");
    formData.set("change_note", "update note");
    formData.set("policy_json", JSON.stringify({ rules: [] }));
    const request = new Request("http://localhost/dashboard/abac", { method: "POST", body: formData });

    const result = await action({ request, params: {}, context: {} } as never);

    expect(abacApi.updateDraft).toHaveBeenCalledWith(
      "tenant-1",
      "v2",
      { policy: { rules: [] }, change_note: "update note" },
      "test-token"
    );
    expect(result).toEqual({ success: true, intent: "update_draft" });
  });

  it("action publish and rollback call corresponding api", async () => {
    vi.mocked(abacApi.publish).mockResolvedValue({ message: "ABAC policy published" });
    vi.mocked(abacApi.rollback).mockResolvedValue({ message: "ABAC policy rolled back" });

    const publishForm = new FormData();
    publishForm.set("intent", "publish");
    publishForm.set("version_id", "v3");
    publishForm.set("mode", "shadow");
    const publishRequest = new Request("http://localhost/dashboard/abac", { method: "POST", body: publishForm });
    const publishResult = await action({ request: publishRequest, params: {}, context: {} } as never);

    expect(abacApi.publish).toHaveBeenCalledWith("tenant-1", "v3", "shadow", "test-token");
    expect(publishResult).toEqual({ success: true, intent: "publish" });

    const rollbackForm = new FormData();
    rollbackForm.set("intent", "rollback");
    rollbackForm.set("version_id", "v1");
    rollbackForm.set("mode", "enforce");
    const rollbackRequest = new Request("http://localhost/dashboard/abac", { method: "POST", body: rollbackForm });
    const rollbackResult = await action({ request: rollbackRequest, params: {}, context: {} } as never);

    expect(abacApi.rollback).toHaveBeenCalledWith("tenant-1", "v1", "enforce", "test-token");
    expect(rollbackResult).toEqual({ success: true, intent: "rollback" });
  });

  it("action simulate returns simulation payload", async () => {
    vi.mocked(abacApi.simulate).mockResolvedValue({
      data: {
        decision: "deny",
        matched_allow_rule_ids: [],
        matched_deny_rule_ids: ["deny_off_hours"],
      },
    });

    const formData = new FormData();
    formData.set("intent", "simulate");
    formData.set("sim_action", "user_manage");
    formData.set("sim_resource_type", "tenant");
    formData.set("sim_subject_json", JSON.stringify({ roles: ["admin"] }));
    formData.set("sim_resource_json", JSON.stringify({ tenant_id: "tenant-1" }));
    formData.set("sim_request_json", JSON.stringify({ ip: "127.0.0.1" }));
    formData.set("sim_env_json", JSON.stringify({ hour: 20 }));
    const request = new Request("http://localhost/dashboard/abac", { method: "POST", body: formData });

    const result = await action({ request, params: {}, context: {} } as never);

    expect(abacApi.simulate).toHaveBeenCalledWith(
      "tenant-1",
      {
        policy: undefined,
        simulation: {
          action: "user_manage",
          resource_type: "tenant",
          subject: { roles: ["admin"] },
          resource: { tenant_id: "tenant-1" },
          request: { ip: "127.0.0.1" },
          env: { hour: 20 },
        },
      },
      "test-token"
    );
    expect(result).toEqual({
      success: true,
      intent: "simulate",
      simulation: {
        decision: "deny",
        matched_allow_rule_ids: [],
        matched_deny_rule_ids: ["deny_off_hours"],
      },
    });
  });

  it("action returns 400 when simulation fields are missing", async () => {
    const formData = new FormData();
    formData.set("intent", "simulate");
    const request = new Request("http://localhost/dashboard/abac", { method: "POST", body: formData });

    const result = await action({ request, params: {}, context: {} } as never);
    expect(result).toBeInstanceOf(Response);
    const response = result as Response;
    expect(response.status).toBe(400);
    await expect(response.json()).resolves.toEqual({
      error: "Simulation action and resource type are required",
    });
  });
});
