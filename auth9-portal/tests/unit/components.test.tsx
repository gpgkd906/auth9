import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Button } from '~/components/ui/button';
import { Input } from '~/components/ui/input';
import { Label } from '~/components/ui/label';
import { Checkbox } from '~/components/ui/checkbox';

describe('UI Components', () => {
  describe('Button', () => {
    it('should render with default variant and size', () => {
      render(<Button>Click me</Button>);

      const button = screen.getByRole('button', { name: /click me/i });
      expect(button).toBeInTheDocument();
      expect(button).toHaveClass('bg-[var(--accent-blue)]');
    });

    it('should render with destructive variant', () => {
      render(<Button variant="destructive">Delete</Button>);

      const button = screen.getByRole('button', { name: /delete/i });
      expect(button).toHaveClass('bg-[var(--accent-red)]');
    });

    it('should render with outline variant', () => {
      render(<Button variant="outline">Cancel</Button>);

      const button = screen.getByRole('button', { name: /cancel/i });
      expect(button).toHaveClass('border-[var(--glass-border-subtle)]');
    });

    it('should render with secondary variant', () => {
      render(<Button variant="secondary">Secondary</Button>);

      const button = screen.getByRole('button', { name: /secondary/i });
      expect(button).toHaveClass('bg-[var(--sidebar-item-hover)]');
    });

    it('should render with ghost variant', () => {
      render(<Button variant="ghost">Ghost</Button>);

      const button = screen.getByRole('button', { name: /ghost/i });
      expect(button).toHaveClass('hover:bg-[var(--sidebar-item-hover)]');
    });

    it('should render with link variant', () => {
      render(<Button variant="link">Link</Button>);

      const button = screen.getByRole('button', { name: /link/i });
      expect(button).toHaveClass('text-[var(--accent-blue)]');
    });

    it('should render with small size', () => {
      render(<Button size="sm">Small</Button>);

      const button = screen.getByRole('button', { name: /small/i });
      expect(button).toHaveClass('h-8');
    });

    it('should render with large size', () => {
      render(<Button size="lg">Large</Button>);

      const button = screen.getByRole('button', { name: /large/i });
      expect(button).toHaveClass('h-11');
    });

    it('should render with icon size', () => {
      render(<Button size="icon">+</Button>);

      const button = screen.getByRole('button', { name: /\+/i });
      expect(button).toHaveClass('h-9', 'w-9');
    });

    it('should be disabled when disabled prop is true', () => {
      render(<Button disabled>Disabled</Button>);

      const button = screen.getByRole('button', { name: /disabled/i });
      expect(button).toBeDisabled();
    });

    it('should call onClick handler when clicked', async () => {
      const user = userEvent.setup();
      const handleClick = vi.fn();

      render(<Button onClick={handleClick}>Click</Button>);

      await user.click(screen.getByRole('button', { name: /click/i }));
      expect(handleClick).toHaveBeenCalledTimes(1);
    });

    it('should not call onClick when disabled', async () => {
      const user = userEvent.setup();
      const handleClick = vi.fn();

      render(<Button onClick={handleClick} disabled>Disabled</Button>);

      await user.click(screen.getByRole('button', { name: /disabled/i }));
      expect(handleClick).not.toHaveBeenCalled();
    });

    it('should accept custom className', () => {
      render(<Button className="custom-class">Custom</Button>);

      const button = screen.getByRole('button', { name: /custom/i });
      expect(button).toHaveClass('custom-class');
    });

    it('should render as child component when asChild is true', () => {
      render(
        <Button asChild>
          <a href="/test">Link Button</a>
        </Button>
      );

      const link = screen.getByRole('link', { name: /link button/i });
      expect(link).toBeInTheDocument();
      expect(link).toHaveAttribute('href', '/test');
    });
  });

  describe('Input', () => {
    it('should render with default styles', () => {
      render(<Input placeholder="Enter text" />);

      const input = screen.getByPlaceholderText('Enter text');
      expect(input).toBeInTheDocument();
      expect(input).toHaveClass('rounded-[10px]');
    });

    it('should accept text input', async () => {
      const user = userEvent.setup();
      render(<Input placeholder="Type here" />);

      const input = screen.getByPlaceholderText('Type here');
      await user.type(input, 'Hello World');

      expect(input).toHaveValue('Hello World');
    });

    it('should render with type="password"', () => {
      render(<Input type="password" placeholder="Password" />);

      const input = screen.getByPlaceholderText('Password');
      expect(input).toHaveAttribute('type', 'password');
    });

    it('should render with type="email"', () => {
      render(<Input type="email" placeholder="Email" />);

      const input = screen.getByPlaceholderText('Email');
      expect(input).toHaveAttribute('type', 'email');
    });

    it('should render with type="number"', () => {
      render(<Input type="number" placeholder="Number" />);

      const input = screen.getByPlaceholderText('Number');
      expect(input).toHaveAttribute('type', 'number');
    });

    it('should be disabled when disabled prop is true', () => {
      render(<Input disabled placeholder="Disabled" />);

      const input = screen.getByPlaceholderText('Disabled');
      expect(input).toBeDisabled();
    });

    it('should accept custom className', () => {
      render(<Input className="custom-input" placeholder="Custom" />);

      const input = screen.getByPlaceholderText('Custom');
      expect(input).toHaveClass('custom-input');
    });

    it('should have disabled styling when disabled', () => {
      render(<Input disabled placeholder="Disabled" />);

      const input = screen.getByPlaceholderText('Disabled');
      expect(input).toHaveClass('disabled:opacity-50');
    });

    it('should call onChange handler', async () => {
      const user = userEvent.setup();
      const handleChange = vi.fn();

      render(<Input onChange={handleChange} placeholder="Change me" />);

      await user.type(screen.getByPlaceholderText('Change me'), 'a');
      expect(handleChange).toHaveBeenCalled();
    });

    it('should accept defaultValue', () => {
      render(<Input defaultValue="Default" placeholder="Input" />);

      const input = screen.getByPlaceholderText('Input');
      expect(input).toHaveValue('Default');
    });

    it('should accept value prop for controlled input', () => {
      render(<Input value="Controlled" onChange={() => { }} placeholder="Input" />);

      const input = screen.getByPlaceholderText('Input');
      expect(input).toHaveValue('Controlled');
    });
  });

  describe('Label', () => {
    it('should render with default styles', () => {
      render(<Label>Test Label</Label>);
      const label = screen.getByText('Test Label');
      expect(label).toBeInTheDocument();
      expect(label).toHaveClass('text-sm', 'font-medium');
    });

    it('should accept custom className', () => {
      render(<Label className="custom-label">Test Label</Label>);
      const label = screen.getByText('Test Label');
      expect(label).toHaveClass('custom-label');
    });
  });

  describe('Checkbox', () => {
    it('should render unchecked by default', () => {
      render(<Checkbox />);
      const checkbox = screen.getByRole('checkbox');
      expect(checkbox).toBeInTheDocument();
      expect(checkbox).not.toBeChecked();
    });

    it('should be checked when defaultChecked is true', () => {
      render(<Checkbox defaultChecked />);
      const checkbox = screen.getByRole('checkbox');
      expect(checkbox).toBeChecked();
    });

    it('should toggle when clicked', async () => {
      const user = userEvent.setup();
      render(<Checkbox />);
      const checkbox = screen.getByRole('checkbox');

      await user.click(checkbox);
      expect(checkbox).toBeChecked();

      await user.click(checkbox);
      expect(checkbox).not.toBeChecked();
    });

    it('should be disabled when disabled prop is true', () => {
      render(<Checkbox disabled />);
      const checkbox = screen.getByRole('checkbox');
      expect(checkbox).toBeDisabled();
    });

    it('should accept custom className', () => {
      render(<Checkbox className="custom-checkbox" />);
      const checkbox = screen.getByRole('checkbox');
      expect(checkbox).toHaveClass('custom-checkbox');
    });
  });
});
