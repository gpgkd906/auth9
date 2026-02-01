import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import {
    Card,
    CardHeader,
    CardTitle,
    CardDescription,
    CardContent,
    CardFooter,
} from "~/components/ui/card";

describe("Card", () => {
    it("renders all subcomponents correctly", () => {
        render(
            <Card className="custom-card">
                <CardHeader className="custom-header">
                    <CardTitle>Card Title</CardTitle>
                    <CardDescription>Card Description</CardDescription>
                </CardHeader>
                <CardContent className="custom-content">
                    <p>Content</p>
                </CardContent>
                <CardFooter className="custom-footer">
                    <p>Footer</p>
                </CardFooter>
            </Card>
        );

        const card = screen.getByText("Content").closest(".liquid-glass");
        expect(card).toHaveClass("custom-card");

        expect(screen.getByText("Card Title")).toBeInTheDocument();
        expect(screen.getByText("Card Description")).toBeInTheDocument();
        expect(screen.getByText("Content")).toBeInTheDocument();
        expect(screen.getByText("Footer")).toBeInTheDocument();
    });
});
