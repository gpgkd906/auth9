import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { Textarea } from "~/components/ui/textarea";

describe("Textarea Component", () => {
    it("renders with default styles", () => {
        render(<Textarea data-testid="textarea" />);
        
        const textarea = screen.getByTestId("textarea");
        expect(textarea).toBeInTheDocument();
        expect(textarea.tagName).toBe("TEXTAREA");
    });

    it("applies custom className", () => {
        render(<Textarea className="custom-class" data-testid="textarea" />);
        
        const textarea = screen.getByTestId("textarea");
        expect(textarea).toHaveClass("custom-class");
    });

    it("renders with placeholder", () => {
        render(<Textarea placeholder="Enter your message..." />);
        
        expect(screen.getByPlaceholderText("Enter your message...")).toBeInTheDocument();
    });

    it("handles value change", () => {
        const handleChange = vi.fn();
        render(<Textarea onChange={handleChange} data-testid="textarea" />);
        
        const textarea = screen.getByTestId("textarea");
        fireEvent.change(textarea, { target: { value: "Hello, world!" } });
        
        expect(handleChange).toHaveBeenCalled();
    });

    it("can be disabled", () => {
        render(<Textarea disabled data-testid="textarea" />);
        
        const textarea = screen.getByTestId("textarea");
        expect(textarea).toBeDisabled();
    });

    it("supports rows attribute", () => {
        render(<Textarea rows={5} data-testid="textarea" />);
        
        const textarea = screen.getByTestId("textarea");
        expect(textarea).toHaveAttribute("rows", "5");
    });

    it("forwards ref correctly", () => {
        const ref = vi.fn();
        render(<Textarea ref={ref} />);
        
        expect(ref).toHaveBeenCalled();
        expect(ref.mock.calls[0][0]).toBeInstanceOf(HTMLTextAreaElement);
    });

    it("renders with initial value", () => {
        render(<Textarea defaultValue="Initial text" data-testid="textarea" />);
        
        const textarea = screen.getByTestId("textarea") as HTMLTextAreaElement;
        expect(textarea.value).toBe("Initial text");
    });

    it("supports required attribute", () => {
        render(<Textarea required data-testid="textarea" />);
        
        const textarea = screen.getByTestId("textarea");
        expect(textarea).toBeRequired();
    });

    it("supports name attribute", () => {
        render(<Textarea name="message" data-testid="textarea" />);
        
        const textarea = screen.getByTestId("textarea");
        expect(textarea).toHaveAttribute("name", "message");
    });

    it("supports id attribute", () => {
        render(<Textarea id="my-textarea" data-testid="textarea" />);
        
        const textarea = screen.getByTestId("textarea");
        expect(textarea).toHaveAttribute("id", "my-textarea");
    });

    it("supports aria-label for accessibility", () => {
        render(<Textarea aria-label="Message input" data-testid="textarea" />);
        
        const textarea = screen.getByTestId("textarea");
        expect(textarea).toHaveAttribute("aria-label", "Message input");
    });
});
