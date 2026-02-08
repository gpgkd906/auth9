import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import {
    Table,
    TableHeader,
    TableBody,
    TableFooter,
    TableHead,
    TableRow,
    TableCell,
    TableCaption,
} from "~/components/ui/table";

describe("Table", () => {
    it("renders all table subcomponents", () => {
        render(
            <Table>
                <TableCaption>A list of items</TableCaption>
                <TableHeader>
                    <TableRow>
                        <TableHead>Name</TableHead>
                        <TableHead>Value</TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    <TableRow>
                        <TableCell>Item 1</TableCell>
                        <TableCell>100</TableCell>
                    </TableRow>
                </TableBody>
                <TableFooter>
                    <TableRow>
                        <TableCell>Total</TableCell>
                        <TableCell>100</TableCell>
                    </TableRow>
                </TableFooter>
            </Table>
        );

        expect(screen.getByText("A list of items")).toBeInTheDocument();
        expect(screen.getByText("Name")).toBeInTheDocument();
        expect(screen.getByText("Value")).toBeInTheDocument();
        expect(screen.getByText("Item 1")).toBeInTheDocument();
        expect(screen.getByText("Total")).toBeInTheDocument();
    });

    it("applies custom className to Table", () => {
        render(
            <Table className="custom-table">
                <TableBody>
                    <TableRow>
                        <TableCell>Test</TableCell>
                    </TableRow>
                </TableBody>
            </Table>
        );

        const table = screen.getByRole("table");
        expect(table).toHaveClass("custom-table");
    });

    it("applies custom className to TableFooter", () => {
        render(
            <Table>
                <TableFooter className="custom-footer" data-testid="tfooter">
                    <TableRow>
                        <TableCell>Footer</TableCell>
                    </TableRow>
                </TableFooter>
            </Table>
        );

        expect(screen.getByTestId("tfooter")).toHaveClass("custom-footer");
    });

    it("applies custom className to TableCaption", () => {
        render(
            <Table>
                <TableCaption className="custom-caption">Caption text</TableCaption>
                <TableBody>
                    <TableRow>
                        <TableCell>Data</TableCell>
                    </TableRow>
                </TableBody>
            </Table>
        );

        expect(screen.getByText("Caption text")).toHaveClass("custom-caption");
    });
});
