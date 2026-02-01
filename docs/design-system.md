# Auth9 Portal Design System

## Overview

Auth9 Portal uses the **Liquid Glass** design language, inspired by Apple's WWDC 2025 design direction. This system creates a modern, premium feel with semi-transparent surfaces, subtle blur effects, and smooth animations.

## Design Philosophy

### Core Visual Characteristics
- **Semi-transparency**: Glass-like surfaces that reveal content beneath
- **Blur effects**: Backdrop blur creates depth and hierarchy
- **Light refraction**: Subtle highlights simulate light passing through glass
- **Rounded corners**: Soft, organic shapes (20px for cards, 24px for sidebar)
- **Dynamic backgrounds**: Animated gradients that respond to content

### Design Principles
1. **Clarity**: Content should always be readable despite transparency effects
2. **Depth**: Use shadows and blur to create visual hierarchy
3. **Consistency**: Same glass treatment across all surfaces
4. **Performance**: Provide fallbacks for devices that don't support backdrop-filter

---

## Color System

### Light Mode (`:root`)

| Token | Value | Usage |
|-------|-------|-------|
| `--bg-primary` | `#F2F2F7` | Page background |
| `--bg-secondary` | `#FFFFFF` | Secondary surfaces |
| `--bg-tertiary` | `#E5E5EA` | Tertiary backgrounds |
| `--glass-bg` | `rgba(255, 255, 255, 0.72)` | Glass card background |
| `--glass-bg-hover` | `rgba(255, 255, 255, 0.85)` | Glass card hover state |
| `--glass-border` | `rgba(255, 255, 255, 0.5)` | Glass border (bright) |
| `--glass-border-subtle` | `rgba(0, 0, 0, 0.06)` | Subtle dividers |
| `--glass-shadow` | `rgba(0, 0, 0, 0.08)` | Default shadow color |
| `--glass-shadow-strong` | `rgba(0, 0, 0, 0.15)` | Emphasized shadow |
| `--glass-highlight` | `rgba(255, 255, 255, 0.9)` | Inner highlight (top edge) |
| `--glass-illumination` | `rgba(255, 255, 255, 0.4)` | Gradient illumination |
| `--text-primary` | `#1D1D1F` | Main text |
| `--text-secondary` | `#86868B` | Secondary text |
| `--text-tertiary` | `#AEAEB2` | Placeholder/disabled text |
| `--text-inverse` | `#FFFFFF` | Text on dark backgrounds |

### Dark Mode (`[data-theme="dark"]`)

| Token | Value | Usage |
|-------|-------|-------|
| `--bg-primary` | `#000000` | Page background |
| `--bg-secondary` | `#1C1C1E` | Secondary surfaces |
| `--bg-tertiary` | `#2C2C2E` | Tertiary backgrounds |
| `--glass-bg` | `rgba(44, 44, 46, 0.65)` | Glass card background |
| `--glass-bg-hover` | `rgba(58, 58, 60, 0.75)` | Glass card hover state |
| `--glass-border` | `rgba(255, 255, 255, 0.1)` | Glass border |
| `--glass-border-subtle` | `rgba(255, 255, 255, 0.05)` | Subtle dividers |
| `--glass-shadow` | `rgba(0, 0, 0, 0.4)` | Default shadow color |
| `--glass-shadow-strong` | `rgba(0, 0, 0, 0.6)` | Emphasized shadow |
| `--glass-highlight` | `rgba(255, 255, 255, 0.15)` | Inner highlight |
| `--glass-illumination` | `rgba(255, 255, 255, 0.05)` | Gradient illumination |
| `--text-primary` | `#FFFFFF` | Main text |
| `--text-secondary` | `#98989D` | Secondary text |
| `--text-tertiary` | `#636366` | Placeholder/disabled text |
| `--text-inverse` | `#000000` | Text on light backgrounds |

### Accent Colors (Same for both modes)

| Token | Value | Usage |
|-------|-------|-------|
| `--accent-blue` | `#007AFF` | Primary action, links |
| `--accent-blue-light` | Light: `rgba(0, 122, 255, 0.12)` / Dark: `rgba(0, 122, 255, 0.2)` | Blue tint backgrounds |
| `--accent-green` | `#34C759` | Success states |
| `--accent-green-light` | Light: `rgba(52, 199, 89, 0.12)` / Dark: `rgba(52, 199, 89, 0.2)` | Success backgrounds |
| `--accent-orange` | `#FF9500` | Warning states |
| `--accent-orange-light` | Light: `rgba(255, 149, 0, 0.12)` / Dark: `rgba(255, 149, 0, 0.2)` | Warning backgrounds |
| `--accent-red` | `#FF3B30` | Error/destructive |
| `--accent-red-light` | Light: `rgba(255, 59, 48, 0.12)` / Dark: `rgba(255, 59, 48, 0.2)` | Error backgrounds |
| `--accent-purple` | `#AF52DE` | Highlight, branding |
| `--accent-purple-light` | Light: `rgba(175, 82, 222, 0.12)` / Dark: `rgba(175, 82, 222, 0.2)` | Purple tint |
| `--accent-cyan` | `#32ADE6` | Information |
| `--accent-cyan-light` | Light: `rgba(50, 173, 230, 0.12)` / Dark: `rgba(50, 173, 230, 0.2)` | Cyan tint |

### Sidebar Colors

| Token | Light Mode | Dark Mode |
|-------|------------|-----------|
| `--sidebar-bg` | `rgba(255, 255, 255, 0.7)` | `rgba(28, 28, 30, 0.75)` |
| `--sidebar-border` | `rgba(0, 0, 0, 0.08)` | `rgba(255, 255, 255, 0.08)` |
| `--sidebar-item-hover` | `rgba(0, 0, 0, 0.04)` | `rgba(255, 255, 255, 0.06)` |
| `--sidebar-item-active-bg` | `var(--accent-blue-light)` | `var(--accent-blue-light)` |
| `--sidebar-item-active-text` | `var(--accent-blue)` | `var(--accent-blue)` |

---

## Spacing & Layout

### Spacing Scale (Compact)

| Size | Value | Usage |
|------|-------|-------|
| `xs` | 4px | Inline spacing, icon gaps |
| `sm` | 8px | Compact element spacing |
| `md` | 12px | Default element spacing |
| `lg` | 16px | Section padding, grid gaps |
| `xl` | 20px | Card padding |
| `2xl` | 24px | Page section spacing |

### Layout Guidelines

- **Grid gaps**: Use `gap-4` (16px) for stat cards, content grids
- **Section spacing**: Use `space-y-6` (24px) between major page sections
- **Card padding**: `p-5` (20px) for card headers/content
- **Table cells**: `px-4 py-3` (16px horizontal, 12px vertical)

### Border Radius

| Token | Value | Usage |
|-------|-------|-------|
| Cards | 20px | Main content cards |
| Sidebar | 24px | Floating sidebar container |
| Buttons | 12px | All button variants |
| Inputs | 12px | Form inputs, selects |
| Badges | 100px (pill) | Status badges |
| Avatars | 50% | Circular avatars |
| Small elements | 10px | Dropdown items, tabs |

---

## Glass Effect Parameters

### Primary Glass (Cards, Panels)

```css
.liquid-glass {
  background: var(--glass-bg);
  backdrop-filter: blur(24px) saturate(180%);
  -webkit-backdrop-filter: blur(24px) saturate(180%);
  border: 1px solid var(--glass-border);
  border-radius: 20px;
  box-shadow:
    0 8px 32px var(--glass-shadow),
    inset 0 1px 0 var(--glass-highlight),
    inset 0 -1px 0 rgba(0, 0, 0, 0.05);
}

/* Illumination gradient overlay */
.liquid-glass::before {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: inherit;
  background: linear-gradient(
    135deg,
    var(--glass-illumination) 0%,
    transparent 50%
  );
  pointer-events: none;
}
```

### Hover State

```css
.liquid-glass:hover {
  background: var(--glass-bg-hover);
  box-shadow:
    0 12px 40px var(--glass-shadow-strong),
    inset 0 1px 0 var(--glass-highlight),
    inset 0 -1px 0 rgba(0, 0, 0, 0.05);
  transform: translateY(-2px);
}
```

### Sidebar Glass (Heavy Blur)

```css
.sidebar {
  background: var(--sidebar-bg);
  backdrop-filter: blur(40px) saturate(180%);
  border: 1px solid var(--sidebar-border);
  border-radius: 24px;
}
```

### Performance Fallback

```css
@supports not (backdrop-filter: blur(24px)) {
  .liquid-glass {
    background: var(--bg-secondary);
  }
}
```

---

## Components

### Button Variants

| Variant | Description | Usage |
|---------|-------------|-------|
| `default` | Blue background, white text | Primary actions |
| `secondary` | Glass background | Secondary actions |
| `outline` | Transparent with border | Tertiary actions |
| `ghost` | No background | Minimal actions |
| `glass` | Full glass effect | Special emphasis |
| `destructive` | Red background | Delete, danger actions |

```tsx
// Primary button
<Button>Create Tenant</Button>

// Glass button
<Button variant="glass">Sign In</Button>

// Destructive button
<Button variant="destructive">Delete User</Button>
```

### Card Component

Cards automatically apply `liquid-glass` styling:

```tsx
<Card>
  <CardHeader>
    <CardTitle>Recent Users</CardTitle>
    <CardDescription>Latest registrations</CardDescription>
  </CardHeader>
  <CardContent>
    {/* Content */}
  </CardContent>
</Card>
```

### Stat Card with Glow

```tsx
<Card className="stat-card stat-card-blue">
  <div className="stat-icon stat-icon-blue">
    <UsersIcon />
  </div>
  <div className="stat-label">Active Users</div>
  <div className="stat-value">1,847</div>
  <div className="stat-change stat-change-positive">
    +12% from last month
  </div>
</Card>
```

### Input Fields

```tsx
<div className="form-group">
  <Label htmlFor="email">Email</Label>
  <Input
    id="email"
    type="email"
    placeholder="you@example.com"
  />
</div>
```

### Sidebar Navigation Item

```tsx
<Link
  to="/dashboard/users"
  className={cn("sidebar-item", isActive && "active")}
>
  <UsersIcon className="w-5 h-5" />
  Users
</Link>
```

### Badge Variants

```tsx
// Success
<Badge variant="success">Active</Badge>

// Warning
<Badge variant="warning">Pending</Badge>

// Danger
<Badge variant="danger">Suspended</Badge>
```

---

## Animation Specifications

### Entry Animation (fadeInUp)

```css
@keyframes fadeInUp {
  from {
    opacity: 0;
    transform: translateY(20px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.animate-fade-in-up {
  animation: fadeInUp 0.5s cubic-bezier(0.34, 1.56, 0.64, 1) forwards;
}
```

### Background Animation

```css
@keyframes backdrop-shift {
  0% { transform: translate(0, 0) scale(1); }
  100% { transform: translate(5%, 5%) scale(1.1); }
}

.page-backdrop::before {
  animation: backdrop-shift 20s ease-in-out infinite alternate;
}
```

### Stagger Delays

```css
.delay-1 { animation-delay: 50ms; }
.delay-2 { animation-delay: 100ms; }
.delay-3 { animation-delay: 150ms; }
.delay-4 { animation-delay: 200ms; }
.delay-5 { animation-delay: 250ms; }
.delay-6 { animation-delay: 300ms; }
```

### Theme Transition

```css
body {
  transition: background 0.4s ease, color 0.3s ease;
}

.liquid-glass {
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}
```

### Hover Effects

- **Cards**: `transform: translateY(-2px)` + enhanced shadow
- **Buttons**: `transform: translateY(-1px)` for primary
- **Sidebar items**: Background color transition only (no transform)

---

## Dynamic Background

The page background includes an animated gradient that shifts subtly:

```css
.page-backdrop {
  position: fixed;
  inset: 0;
  z-index: -1;
  background: var(--bg-primary);
  overflow: hidden;
}

.page-backdrop::before {
  content: '';
  position: absolute;
  inset: -50%;
  background: var(--backdrop-gradient);
  animation: backdrop-shift 20s ease-in-out infinite alternate;
}
```

### Light Mode Gradient

```css
--backdrop-gradient: linear-gradient(135deg,
  rgba(255, 204, 128, 0.3) 0%,
  rgba(255, 182, 193, 0.25) 25%,
  rgba(173, 216, 230, 0.3) 50%,
  rgba(221, 160, 221, 0.25) 75%,
  rgba(255, 218, 185, 0.3) 100%);
```

### Dark Mode Gradient

```css
--backdrop-gradient: linear-gradient(135deg,
  rgba(99, 102, 241, 0.15) 0%,
  rgba(168, 85, 247, 0.12) 25%,
  rgba(6, 182, 212, 0.1) 50%,
  rgba(236, 72, 153, 0.08) 75%,
  rgba(34, 211, 238, 0.12) 100%);
```

---

## Theme Toggle

The theme toggle appears in the top-right corner:

```tsx
<ThemeToggle />
```

Implementation:
- Fixed position: `top: 24px; right: 24px`
- Glass container with two icon buttons
- Active state shows blue tint
- Persists to localStorage as `auth9-theme`

```tsx
// Usage in dashboard layout
export default function Dashboard() {
  return (
    <div className="min-h-screen">
      <div className="page-backdrop" />
      <ThemeToggle />
      <aside className="sidebar">...</aside>
      <main>...</main>
    </div>
  );
}
```

---

## Typography

### Font Stack

```css
font-family: "Inter", -apple-system, BlinkMacSystemFont,
             "SF Pro Display", "SF Pro Text", sans-serif;
```

### Type Scale (Compact)

| Element | Size | Weight | Line Height | Notes |
|---------|------|--------|-------------|-------|
| Dashboard Title | 28px | 700 | 1.2 | Main dashboard heading |
| Page Title | 24px | 600 | 1.2 | Sub-page headings |
| Stat Value | 26-28px | 700 | 1.0 | Numeric stats |
| Card Title | 16-17px | 600 | 1.3 | Card headers |
| Body | 13-14px | 400 | 1.5 | General text |
| Small/Label | 13px | 500 | 1.4 | Form labels, descriptions |
| Caption | 11-12px | 500 | 1.3 | Timestamps, metadata |
| Table Header | 11px | 600 | 1.0 | Uppercase table headers |
| Nav Section | 11px | 600 | 1.0 | Sidebar section titles |

### Letter Spacing

- Headings: `-0.02em`
- Uppercase labels: `0.04em` to `0.06em`

---

## Accessibility

### Color Contrast

All text combinations meet WCAG AA standards:
- Primary text on glass: 7:1+ ratio
- Secondary text on glass: 4.5:1+ ratio
- Links: Clearly distinguishable blue

### Focus States

```css
:focus-visible {
  outline: 2px solid var(--accent-blue);
  outline-offset: 2px;
}
```

### Reduced Motion

```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```

---

## Example Patterns

### Stats Grid

```tsx
<div className="grid grid-cols-4 gap-4">
  <Card className="stat-card stat-card-blue">
    <div className="stat-icon stat-icon-blue">
      <BuildingIcon />
    </div>
    <div className="stat-label">Total Tenants</div>
    <div className="stat-value">12</div>
  </Card>
  {/* More stat cards... */}
</div>
```

### Data Table

```tsx
<Card>
  <CardHeader>
    <CardTitle>Users</CardTitle>
  </CardHeader>
  <Table>
    <TableHeader>
      <TableRow>
        <TableHead>Name</TableHead>
        <TableHead>Status</TableHead>
      </TableRow>
    </TableHeader>
    <TableBody>
      <TableRow>
        <TableCell>John Doe</TableCell>
        <TableCell>
          <Badge variant="success">Active</Badge>
        </TableCell>
      </TableRow>
    </TableBody>
  </Table>
</Card>
```

### Form Layout

```tsx
<Card>
  <CardHeader>
    <CardTitle>Create Tenant</CardTitle>
  </CardHeader>
  <CardContent>
    <form className="space-y-4">
      <div className="form-group">
        <Label htmlFor="name">Name</Label>
        <Input id="name" placeholder="Acme Corp" />
      </div>
      <div className="form-group">
        <Label htmlFor="slug">Slug</Label>
        <Input id="slug" placeholder="acme-corp" />
      </div>
      <div className="flex gap-3 justify-end">
        <Button variant="outline">Cancel</Button>
        <Button type="submit">Create</Button>
      </div>
    </form>
  </CardContent>
</Card>
```

### Dialog

```tsx
<Dialog>
  <DialogTrigger asChild>
    <Button>Open Dialog</Button>
  </DialogTrigger>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Confirm Action</DialogTitle>
      <DialogDescription>
        Are you sure you want to proceed?
      </DialogDescription>
    </DialogHeader>
    <DialogFooter>
      <Button variant="outline">Cancel</Button>
      <Button>Confirm</Button>
    </DialogFooter>
  </DialogContent>
</Dialog>
```

---

## File Reference

- **Design Preview**: `/design/preview-liquid-glass.html`
- **CSS Variables**: `/app/styles/tailwind.css`
- **Theme Hook**: `/app/hooks/useTheme.ts`
- **Theme Toggle**: `/app/components/ThemeToggle.tsx`
- **UI Components**: `/app/components/ui/*.tsx`
