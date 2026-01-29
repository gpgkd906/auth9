---
name: remix-conventions
description: Remix/TypeScript conventions for auth9-portal
globs: auth9-portal/**/*.{ts,tsx}
alwaysApply: false
---

# Remix Conventions (auth9-portal)

## Tech Stack

- **Framework**: Remix + Vite
- **UI**: Radix UI + Tailwind CSS
- **Validation**: Zod + Conform
- **Testing**: Vitest (unit), Playwright (E2E)
- **State**: Remix loader/action (server-first)

## Apple-Style UI Design

```tsx
// Design principles:
// - Minimalism: generous whitespace, clean lines
// - Glassmorphism: backdrop-filter: blur(), semi-transparent bg
// - Large radius: rounded-2xl for cards, rounded-xl for buttons
// - Subtle animations: hover/focus transitions (200-300ms)
// - System fonts: font-sans (Inter/SF Pro Display)
// - Restrained colors: neutral grays + single accent color

// ✅ Example card
<div className="rounded-2xl bg-white/80 backdrop-blur-xl 
               shadow-sm border border-gray-100 p-6">
  ...
</div>
```

## Route Structure

```
app/routes/
├── _index.tsx          # Landing page
├── login.tsx           # Login page
├── dashboard.tsx       # Dashboard layout
├── dashboard._index.tsx # Dashboard home
├── tenants._index.tsx  # Tenant list
└── tenants.$id.tsx     # Tenant detail
```

## Data Loading Pattern

```tsx
// ✅ Use loader for data fetching
export async function loader({ request }: LoaderFunctionArgs) {
  const tenants = await api.getTenants();
  return json({ tenants });
}

// ✅ Use action for mutations
export async function action({ request }: ActionFunctionArgs) {
  const formData = await request.formData();
  const result = await api.createTenant(formData);
  return redirect(`/tenants/${result.id}`);
}
```

## Component Guidelines

```tsx
// ✅ Functional components only
export function TenantCard({ tenant }: { tenant: Tenant }) {
  return (
    <Card className="hover:shadow-md transition-shadow duration-200">
      <CardHeader>
        <h3 className="text-lg font-semibold">{tenant.name}</h3>
      </CardHeader>
    </Card>
  );
}

// ✅ Extract hooks for reusable logic
function useTenantForm() {
  const [isPending, startTransition] = useTransition();
  // ...
}
```

## API Service Layer

```typescript
// app/services/api.ts
// Centralize API calls with proper typing
export const api = {
  async getTenants(): Promise<Tenant[]> {
    const res = await fetch(`${API_URL}/api/v1/tenants`);
    if (!res.ok) throw new Error('Failed to fetch tenants');
    return res.json();
  },
};
```
