import { render } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import SettingsPasskeysRedirect, { loader } from "~/routes/dashboard.settings.passkeys";

describe("Settings Passkeys Redirect", () => {
    it("loader redirects to /dashboard/account/passkeys", () => {
        const response = loader();
        expect(response.status).toBe(302);
        expect(response.headers.get("Location")).toBe("/dashboard/account/passkeys");
    });

    it("component renders null", () => {
        const { container } = render(<SettingsPasskeysRedirect />);
        expect(container.innerHTML).toBe("");
    });
});
