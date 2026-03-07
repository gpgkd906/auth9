import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createRoutesStub } from "react-router";
import { LanguageSwitcher } from "~/components/LanguageSwitcher";

function renderSwitcher() {
  const RoutesStub = createRoutesStub([
    {
      path: "/",
      Component: () => <LanguageSwitcher />,
    },
  ]);
  return render(<RoutesStub initialEntries={["/"]} />);
}

describe("LanguageSwitcher", () => {
  it("renders a language select with aria-label", async () => {
    renderSwitcher();
    const select = await screen.findByRole("combobox", { name: /switch language|切换语言/i });
    expect(select).toBeInTheDocument();
  });

  it("renders all three language options", async () => {
    renderSwitcher();
    const options = await screen.findAllByRole("option");
    expect(options).toHaveLength(3);

    const values = options.map((opt) => (opt as HTMLOptionElement).value);
    expect(values).toContain("zh-CN");
    expect(values).toContain("en-US");
    expect(values).toContain("ja");
  });

  it("displays localized language names", async () => {
    renderSwitcher();
    await screen.findAllByRole("option");

    expect(screen.getByText("简体中文")).toBeInTheDocument();
    expect(screen.getByText("English")).toBeInTheDocument();
    expect(screen.getByText("日本語")).toBeInTheDocument();
  });

  it("allows changing language via select", async () => {
    const user = userEvent.setup();
    renderSwitcher();

    const select = await screen.findByRole("combobox");
    await user.selectOptions(select, "ja");
    expect((select as HTMLSelectElement).value).toBe("ja");
  });
});
