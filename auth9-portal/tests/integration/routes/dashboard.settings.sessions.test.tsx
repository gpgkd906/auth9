import { render } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import SettingsSessionsRedirect, { loader } from "~/routes/dashboard.settings.sessions";

describe("Settings Sessions Redirect", () => {
    it("loader redirects to /dashboard/account/sessions", () => {
        const response = loader();
        expect(response.status).toBe(302);
        expect(response.headers.get("Location")).toBe("/dashboard/account/sessions");
    });

    it("component renders null", () => {
        const { container } = render(<SettingsSessionsRedirect />);
        expect(container.innerHTML).toBe("");
    });
});
