# Auth9 Keycloak Theme

Custom Keycloak login theme for Auth9 with dynamic branding support. This theme fetches branding configuration (logo, colors, company name) from the auth9 API at runtime, allowing customization without restarting Keycloak.

## Features

- **Dynamic Branding**: Fetches logo, colors, and company name from auth9 API
- **Custom CSS**: Supports injecting custom CSS from the branding settings
- **Modern Design**: Clean, responsive login/registration pages
- **No Default Styles**: Completely custom UI without Keycloak's default PatternFly CSS

## Prerequisites

- Node.js 20+
- npm 10+
- Maven (for building the JAR, or use Docker)

## Development

```bash
# Install dependencies
npm install

# Start Vite dev server
npm run dev

# Build the React app
npm run build
```

## Building the Theme JAR

### Option 1: Local Build (requires Maven)

```bash
# Build the theme JAR
npm run build-keycloak-theme

# Output: dist_keycloak/keycloak-theme-auth9.jar
```

### Option 2: Docker Build (recommended)

```bash
# Build the theme using Docker
docker build -t auth9-keycloak-theme .

# Extract the JAR to current directory
docker run --rm -v $(pwd)/output:/theme-output auth9-keycloak-theme
```

## Docker Compose Integration

Build and deploy with docker-compose:

```bash
# Build the theme JAR
docker-compose --profile build up auth9-theme-builder

# Start all services (including Keycloak with theme)
docker-compose up -d
```

## Configuration

### Environment Variables

The theme reads configuration from Keycloak's theme properties:

| Property | Default | Description |
|----------|---------|-------------|
| `AUTH9_API_URL` | `http://localhost:8080` | URL to the auth9 API |

Set via environment variable when starting Keycloak:

```bash
AUTH9_API_URL=https://api.example.com docker-compose up keycloak
```

## Branding API

The theme fetches branding configuration from:

```
GET /api/v1/public/branding
```

Response format:

```json
{
  "data": {
    "logo_url": "https://example.com/logo.png",
    "primary_color": "#007AFF",
    "secondary_color": "#5856D6",
    "background_color": "#F5F5F7",
    "text_color": "#1D1D1F",
    "custom_css": ".custom { color: red; }",
    "company_name": "My Company",
    "favicon_url": "https://example.com/favicon.ico"
  }
}
```

## Customized Pages

| Page | File | Description |
|------|------|-------------|
| Login | `src/login/pages/Login.tsx` | Main login page |
| Register | `src/login/pages/Register.tsx` | User registration |
| Reset Password | `src/login/pages/LoginResetPassword.tsx` | Password reset request |
| OTP | `src/login/pages/LoginOtp.tsx` | One-time password input |

Other pages use Keycloakify's default implementation.

## Installing in Keycloak

1. Copy the JAR to Keycloak's providers directory:
   ```bash
   cp dist_keycloak/keycloak-theme-auth9.jar /opt/keycloak/providers/
   ```

2. Restart Keycloak

3. Go to Realm Settings → Themes → Login Theme → Select "auth9"

## Architecture

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  auth9-portal   │────▶│   auth9-core    │◀────│    Keycloak     │
│  (Settings UI)  │     │  (Branding API) │     │ (Keycloakify    │
│                 │     │                 │     │   Theme)        │
└─────────────────┘     └─────────────────┘     └─────────────────┘
        │                       │                       │
        │   PUT /api/v1/       │   GET /api/v1/        │
        │   system/branding    │   public/branding     │
        └──────────────────────┴───────────────────────┘
```

## License

MIT
