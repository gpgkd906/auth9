/**
 * Development mode entry point when not running within Keycloak.
 * Shows a preview/demo mode with mock data.
 */
export default function AppEntrypoint() {
  return (
    <div style={{
      minHeight: "100vh",
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      backgroundColor: "#f5f5f7",
      fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif"
    }}>
      <div style={{ textAlign: "center", padding: "2rem" }}>
        <h1 style={{ color: "#1d1d1f", marginBottom: "1rem" }}>Auth9 Keycloak Theme</h1>
        <p style={{ color: "#86868b" }}>
          This theme is designed to run within Keycloak.
        </p>
        <p style={{ color: "#86868b", marginTop: "0.5rem" }}>
          Use Storybook to preview login pages: <code>npm run storybook</code>
        </p>
      </div>
    </div>
  );
}
