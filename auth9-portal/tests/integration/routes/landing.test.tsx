import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import Index, { meta } from "~/routes/_index";

describe("Landing Page", () => {
  it("meta returns correct title and description", () => {
    const result = meta({} as Parameters<typeof meta>[0]);
    expect(result).toEqual([
      { title: "Auth9 - 现代身份管理" },
      { name: "description", content: "安全、可扩展的身份与访问管理平台" },
    ]);
  });

  it("renders landing page with hero content", async () => {
    const RoutesStub = createRoutesStub([
      { path: "/", Component: Index },
    ]);

    render(<RoutesStub initialEntries={["/"]} />);

    await waitFor(() => {
      expect(screen.getByText("Auth9")).toBeInTheDocument();
      expect(screen.getByText("Made Simple")).toBeInTheDocument();
    });
  });

  it("renders feature cards", async () => {
    const RoutesStub = createRoutesStub([
      { path: "/", Component: Index },
    ]);

    render(<RoutesStub initialEntries={["/"]} />);

    await waitFor(() => {
      expect(screen.getByText("Single Sign-On")).toBeInTheDocument();
      expect(screen.getByText("Multi-tenant")).toBeInTheDocument();
      expect(screen.getByText("Dynamic RBAC")).toBeInTheDocument();
    });
  });

  it("renders navigation links", async () => {
    const RoutesStub = createRoutesStub([
      { path: "/", Component: Index },
    ]);

    render(<RoutesStub initialEntries={["/"]} />);

    await waitFor(() => {
      expect(screen.getByText("Sign in")).toBeInTheDocument();
      expect(screen.getByText("Get started")).toBeInTheDocument();
    });
  });
});
