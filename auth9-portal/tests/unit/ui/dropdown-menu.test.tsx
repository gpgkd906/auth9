import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi } from "vitest";
import {
    DropdownMenu,
    DropdownMenuTrigger,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuSeparator,
    DropdownMenuCheckboxItem,
    DropdownMenuRadioGroup,
    DropdownMenuRadioItem,
    DropdownMenuSub,
    DropdownMenuSubTrigger,
    DropdownMenuSubContent,
    DropdownMenuShortcut,
} from "~/components/ui/dropdown-menu";
import { Button } from "~/components/ui/button";

describe("DropdownMenu", () => {
    it("renders trigger and opens menu on click", async () => {
        const user = userEvent.setup();

        render(
            <DropdownMenu>
                <DropdownMenuTrigger asChild>
                    <Button>Open Menu</Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent>
                    <DropdownMenuItem>Item 1</DropdownMenuItem>
                    <DropdownMenuItem>Item 2</DropdownMenuItem>
                </DropdownMenuContent>
            </DropdownMenu>
        );

        await user.click(screen.getByRole("button", { name: /open menu/i }));

        expect(screen.getByText("Item 1")).toBeInTheDocument();
        expect(screen.getByText("Item 2")).toBeInTheDocument();
    });

    it("calls onClick handler when menu item is clicked", async () => {
        const handleClick = vi.fn();
        const user = userEvent.setup();

        render(
            <DropdownMenu>
                <DropdownMenuTrigger asChild>
                    <Button>Open Menu</Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent>
                    <DropdownMenuItem onClick={handleClick}>Click Me</DropdownMenuItem>
                </DropdownMenuContent>
            </DropdownMenu>
        );

        await user.click(screen.getByRole("button", { name: /open menu/i }));
        await user.click(screen.getByText("Click Me"));

        expect(handleClick).toHaveBeenCalledTimes(1);
    });

    it("renders label and separator", async () => {
        const user = userEvent.setup();

        render(
            <DropdownMenu>
                <DropdownMenuTrigger asChild>
                    <Button>Open Menu</Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent>
                    <DropdownMenuLabel>Actions</DropdownMenuLabel>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem>Item</DropdownMenuItem>
                </DropdownMenuContent>
            </DropdownMenu>
        );

        await user.click(screen.getByRole("button", { name: /open menu/i }));

        expect(screen.getByText("Actions")).toBeInTheDocument();
    });

    it("renders checkbox items", async () => {
        const user = userEvent.setup();
        const checked = false;

        render(
            <DropdownMenu>
                <DropdownMenuTrigger asChild>
                    <Button>Open Menu</Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent>
                    <DropdownMenuCheckboxItem checked={checked}>
                        Check Option
                    </DropdownMenuCheckboxItem>
                </DropdownMenuContent>
            </DropdownMenu>
        );

        await user.click(screen.getByRole("button", { name: /open menu/i }));
        expect(screen.getByText("Check Option")).toBeInTheDocument();
    });

    it("renders radio group items", async () => {
        const user = userEvent.setup();

        render(
            <DropdownMenu>
                <DropdownMenuTrigger asChild>
                    <Button>Open Menu</Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent>
                    <DropdownMenuRadioGroup value="option1">
                        <DropdownMenuRadioItem value="option1">Option 1</DropdownMenuRadioItem>
                        <DropdownMenuRadioItem value="option2">Option 2</DropdownMenuRadioItem>
                    </DropdownMenuRadioGroup>
                </DropdownMenuContent>
            </DropdownMenu>
        );

        await user.click(screen.getByRole("button", { name: /open menu/i }));
        expect(screen.getByText("Option 1")).toBeInTheDocument();
        expect(screen.getByText("Option 2")).toBeInTheDocument();
    });

    it("renders submenu", async () => {
        const user = userEvent.setup();

        render(
            <DropdownMenu>
                <DropdownMenuTrigger asChild>
                    <Button>Open Menu</Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent>
                    <DropdownMenuSub>
                        <DropdownMenuSubTrigger>More Options</DropdownMenuSubTrigger>
                        <DropdownMenuSubContent>
                            <DropdownMenuItem>Sub Item</DropdownMenuItem>
                        </DropdownMenuSubContent>
                    </DropdownMenuSub>
                </DropdownMenuContent>
            </DropdownMenu>
        );

        await user.click(screen.getByRole("button", { name: /open menu/i }));
        expect(screen.getByText("More Options")).toBeInTheDocument();
    });

    it("renders shortcut", async () => {
        const user = userEvent.setup();

        render(
            <DropdownMenu>
                <DropdownMenuTrigger asChild>
                    <Button>Open Menu</Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent>
                    <DropdownMenuItem>
                        Copy
                        <DropdownMenuShortcut>⌘C</DropdownMenuShortcut>
                    </DropdownMenuItem>
                </DropdownMenuContent>
            </DropdownMenu>
        );

        await user.click(screen.getByRole("button", { name: /open menu/i }));
        expect(screen.getByText("⌘C")).toBeInTheDocument();
    });

    it("applies inset prop to menu item", async () => {
        const user = userEvent.setup();

        render(
            <DropdownMenu>
                <DropdownMenuTrigger asChild>
                    <Button>Open Menu</Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent>
                    <DropdownMenuItem inset>Inset Item</DropdownMenuItem>
                </DropdownMenuContent>
            </DropdownMenu>
        );

        await user.click(screen.getByRole("button", { name: /open menu/i }));
        expect(screen.getByText("Inset Item")).toBeInTheDocument();
    });

    it("applies inset prop to label", async () => {
        const user = userEvent.setup();

        render(
            <DropdownMenu>
                <DropdownMenuTrigger asChild>
                    <Button>Open Menu</Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent>
                    <DropdownMenuLabel inset>Inset Label</DropdownMenuLabel>
                </DropdownMenuContent>
            </DropdownMenu>
        );

        await user.click(screen.getByRole("button", { name: /open menu/i }));
        expect(screen.getByText("Inset Label")).toBeInTheDocument();
    });

    it("applies inset prop to submenu trigger", async () => {
        const user = userEvent.setup();

        render(
            <DropdownMenu>
                <DropdownMenuTrigger asChild>
                    <Button>Open Menu</Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent>
                    <DropdownMenuSub>
                        <DropdownMenuSubTrigger inset>Inset Submenu</DropdownMenuSubTrigger>
                        <DropdownMenuSubContent>
                            <DropdownMenuItem>Sub Item</DropdownMenuItem>
                        </DropdownMenuSubContent>
                    </DropdownMenuSub>
                </DropdownMenuContent>
            </DropdownMenu>
        );

        await user.click(screen.getByRole("button", { name: /open menu/i }));
        expect(screen.getByText("Inset Submenu")).toBeInTheDocument();
    });
});
