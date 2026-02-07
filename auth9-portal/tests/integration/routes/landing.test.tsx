import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import Index, { meta } from "~/routes/_index";

describe("Landing Page", () => {
  it("meta returns correct title and description", () => {
    const result = meta({} as Parameters<typeof meta>[0]);
    expect(result).toEqual([
      { title: "Auth9 - Modern Identity Management" },
      { name: "description", content: "Secure, scalable identity and access management" },
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
      expect(screen.getByText("Multi-Tenant")).toBeInTheDocument();
      expect(screen.getByText("Dynamic RBAC")).toBeInTheDocument();
    });
  });

  it("renders navigation links", async () => {
    const RoutesStub = createRoutesStub([
      { path: "/", Component: Index },
    ]);

    render(<RoutesStub initialEntries={["/"]} />);

    await waitFor(() => {
      expect(screen.getByText("Sign In")).toBeInTheDocument();
      expect(screen.getByText("Get Started")).toBeInTheDocument();
    });
  });
});
