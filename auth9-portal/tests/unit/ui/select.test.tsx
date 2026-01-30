import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
    SelectGroup,
    SelectLabel,
} from "~/components/ui/select";

describe("Select Component", () => {
    it("renders select trigger", () => {
        render(
            <Select>
                <SelectTrigger data-testid="select-trigger">
                    <SelectValue placeholder="Select an option" />
                </SelectTrigger>
            </Select>
        );

        expect(screen.getByTestId("select-trigger")).toBeInTheDocument();
    });

    it("displays placeholder text", () => {
        render(
            <Select>
                <SelectTrigger>
                    <SelectValue placeholder="Choose..." />
                </SelectTrigger>
            </Select>
        );

        expect(screen.getByText("Choose...")).toBeInTheDocument();
    });

    it("applies custom className to trigger", () => {
        render(
            <Select>
                <SelectTrigger className="custom-trigger" data-testid="select-trigger">
                    <SelectValue placeholder="Select" />
                </SelectTrigger>
            </Select>
        );

        expect(screen.getByTestId("select-trigger")).toHaveClass("custom-trigger");
    });

    it("can be disabled", () => {
        render(
            <Select disabled>
                <SelectTrigger data-testid="select-trigger">
                    <SelectValue placeholder="Select" />
                </SelectTrigger>
            </Select>
        );

        expect(screen.getByTestId("select-trigger")).toBeDisabled();
    });

    it("displays selected value", () => {
        render(
            <Select defaultValue="option1">
                <SelectTrigger>
                    <SelectValue />
                </SelectTrigger>
                <SelectContent>
                    <SelectItem value="option1">Option 1</SelectItem>
                    <SelectItem value="option2">Option 2</SelectItem>
                </SelectContent>
            </Select>
        );

        expect(screen.getByText("Option 1")).toBeInTheDocument();
    });

    it("renders select with group and label", () => {
        render(
            <Select defaultValue="apple">
                <SelectTrigger>
                    <SelectValue />
                </SelectTrigger>
                <SelectContent>
                    <SelectGroup>
                        <SelectLabel>Fruits</SelectLabel>
                        <SelectItem value="apple">Apple</SelectItem>
                        <SelectItem value="banana">Banana</SelectItem>
                    </SelectGroup>
                </SelectContent>
            </Select>
        );

        expect(screen.getByText("Apple")).toBeInTheDocument();
    });

    it("has chevron icon in trigger", () => {
        render(
            <Select>
                <SelectTrigger data-testid="select-trigger">
                    <SelectValue placeholder="Select" />
                </SelectTrigger>
            </Select>
        );

        const trigger = screen.getByTestId("select-trigger");
        // The chevron is inside the trigger
        expect(trigger.querySelector("svg")).toBeInTheDocument();
    });
});
