import { describe, it, expect } from "vitest";
import { loader } from "~/routes/logout";

describe("Logout Page", () => {
  // ============================================================================
  // Loader Tests
  // ============================================================================

  it("loader redirects to auth9 logout endpoint", async () => {
    const request = new Request("http://localhost:3000/logout");
    const response = await loader({ request, params: {}, context: {} });

    expect(response.status).toBe(302);
    const location = response.headers.get("Location");
    expect(location).toContain("/api/v1/auth/logout");
    expect(location).toContain("post_logout_redirect_uri=");
  });

  it("loader includes portal URL in redirect", async () => {
    const request = new Request("http://localhost:3000/logout");
    const response = await loader({ request, params: {}, context: {} });

    const location = response.headers.get("Location");
    expect(location).toContain(encodeURIComponent("http://localhost:3000"));
  });
});
