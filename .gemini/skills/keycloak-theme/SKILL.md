---
name: keycloak-theme
description: Build and customize Auth9 Keycloak login theme using Keycloakify. Use when working on login/register UI, adding new auth pages, modifying branding integration, building theme JAR, or deploying themes to Keycloak.
---

# Auth9 Keycloak Theme

Custom login theme for Keycloak using Keycloakify v11 with dynamic branding from auth9-core API.

## Project Location

```
auth9-keycloak-theme/
├── package.json              # npm scripts and dependencies
├── vite.config.ts            # Vite + Keycloakify plugin config
├── Dockerfile                # Docker build (includes Maven)
├── src/
│   ├── main.tsx              # Entry point (production)
│   ├── main.app.tsx          # Dev mode fallback
│   └── login/
│       ├── KcContext.ts      # Extended Keycloak context type
│       ├── KcPage.tsx        # Page router
│       ├── i18n.ts           # Internationalization
│       ├── hooks/
│       │   └── useBranding.ts    # Fetch branding from API
│       ├── components/
│       │   └── BrandingProvider.tsx  # Branding context + CSS vars
│       ├── pages/            # Custom page implementations
│       │   ├── Login.tsx
│       │   ├── Register.tsx
│       │   ├── LoginResetPassword.tsx
│       │   └── LoginOtp.tsx
│       └── styles/
│           └── index.css
```

## Quick Reference

### Commands

```bash
# Install dependencies
cd auth9-keycloak-theme && npm install

# Development server
npm run dev

# Type check
npx tsc --noEmit

# Build React app
npm run build

# Build theme JAR (requires Maven)
npm run build-keycloak-theme

# Docker build (recommended - no Maven needed)
docker build -t auth9-keycloak-theme .
docker run --rm -v $(pwd)/output:/theme-output auth9-keycloak-theme

# Docker Compose build
docker-compose --profile build up auth9-theme-builder
```

### Deploy to Keycloak

1. Build theme JAR using Docker method above
2. JAR is copied to `keycloak-theme` volume (or `output/` locally)
3. Restart Keycloak: `docker-compose restart keycloak`
4. In Keycloak Admin → Realm Settings → Themes → Login Theme → select `auth9`

## Core Architecture

### Branding Flow

```
Portal (settings/branding) → auth9-core API → Keycloak theme (JS fetch)
                             ↓
                    GET /api/v1/public/branding
                             ↓
                    { primary_color, logo_url, ... }
```

The theme fetches branding at runtime - no rebuild needed for color/logo changes.

### Key Components

| Component | Purpose |
|-----------|---------|
| `useBranding(apiUrl)` | Hook to fetch branding config from API |
| `BrandingProvider` | Context provider, applies CSS variables |
| `KcPage` | Routes `pageId` to custom or default page |
| `KcContext` | Extended type with `auth9ApiUrl` property |

### CSS Variables

Set by `BrandingProvider`, use in custom pages:

```css
--auth9-primary    /* Primary color (buttons, links) */
--auth9-secondary  /* Secondary color */
--auth9-bg         /* Background color */
--auth9-text       /* Text color */
```

## Adding a New Custom Page

### Step 1: Eject the page template

```bash
cd auth9-keycloak-theme
npx keycloakify eject-page
# Select the page to customize (e.g., login-update-password.ftl)
```

### Step 2: Create page component

Create `src/login/pages/LoginUpdatePassword.tsx`:

```tsx
import type { PageProps } from "keycloakify/login/pages/PageProps";
import type { KcContext } from "../KcContext";
import type { I18n } from "../i18n";
import { useBrandingContext } from "../components/BrandingProvider";

export default function LoginUpdatePassword(
  props: PageProps<Extract<KcContext, { pageId: "login-update-password.ftl" }>, I18n>
) {
  const { kcContext, i18n } = props;
  const branding = useBrandingContext();
  const { msg, msgStr } = i18n;

  return (
    <div style={{ backgroundColor: branding.background_color }}>
      {/* Page content using branding colors */}
    </div>
  );
}
```

### Step 3: Add route in KcPage.tsx

```tsx
// Add import at top
const LoginUpdatePassword = lazy(() => import("./pages/LoginUpdatePassword"));

// Add case in switch statement
case "login-update-password.ftl":
  return (
    <LoginUpdatePassword
      kcContext={kcContext}
      i18n={i18n}
      doUseDefaultCss={false}
      classes={{}}
      Template={Template}
    />
  );
```

### Step 4: Build and test

```bash
npm run build
docker-compose --profile build up --build auth9-theme-builder
docker-compose restart keycloak
```

## Branding API Response

```json
{
  "data": {
    "logo_url": "https://example.com/logo.png",
    "primary_color": "#007AFF",
    "secondary_color": "#5856D6",
    "background_color": "#F5F5F7",
    "text_color": "#1D1D1F",
    "custom_css": ".login-form { border-radius: 8px; }",
    "company_name": "My Company",
    "favicon_url": "https://example.com/favicon.ico"
  }
}
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `AUTH9_API_URL` | `http://localhost:8080` | auth9-core API URL |

Set in Keycloak container:

```yaml
environment:
  AUTH9_API_URL: http://auth9-core:8080
```

## Troubleshooting

### Theme not showing

```bash
# Check JAR is mounted
docker exec auth9-keycloak ls -la /opt/keycloak/providers/

# Check Keycloak logs
docker logs auth9-keycloak 2>&1 | grep -i theme
```

### Branding not loading

```bash
# Test API accessibility from Keycloak container
docker exec auth9-keycloak curl -s http://auth9-core:8080/api/v1/public/branding

# Check browser console for CORS errors
```

### Build failures

```bash
# Clean rebuild
rm -rf node_modules dist dist_keycloak
npm install
npx tsc --noEmit  # Check for TS errors
npm run build
```

## Page ID Reference

| Page ID | Description |
|---------|-------------|
| `login.ftl` | Username/password login |
| `register.ftl` | New user registration |
| `login-reset-password.ftl` | Request password reset |
| `login-otp.ftl` | OTP/TOTP verification |
| `login-update-password.ftl` | Force password update |
| `login-verify-email.ftl` | Email verification |
| `login-idp-link-confirm.ftl` | IdP linking confirmation |
| `error.ftl` | Error page |
