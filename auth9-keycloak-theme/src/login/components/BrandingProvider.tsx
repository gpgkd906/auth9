import { createContext, useContext, useEffect, type ReactNode, type CSSProperties } from "react";
import { useBranding, type BrandingConfig, DEFAULT_BRANDING } from "../hooks/useBranding";

const BrandingContext = createContext<BrandingConfig>(DEFAULT_BRANDING);

interface BrandingProviderProps {
  apiUrl: string;
  children: ReactNode;
}

/**
 * Provides branding configuration to child components.
 * Fetches configuration from auth9 API and applies CSS variables.
 */
export function BrandingProvider({ apiUrl, children }: BrandingProviderProps) {
  const { branding, loading } = useBranding(apiUrl);

  // Update favicon when branding loads
  useEffect(() => {
    if (branding.favicon_url) {
      const link = document.querySelector<HTMLLinkElement>("link[rel='icon']");
      if (link) {
        link.href = branding.favicon_url;
      } else {
        const newLink = document.createElement("link");
        newLink.rel = "icon";
        newLink.href = branding.favicon_url;
        document.head.appendChild(newLink);
      }
    }
  }, [branding.favicon_url]);

  // CSS variables for theme colors
  const style: CSSProperties = {
    "--auth9-primary": branding.primary_color,
    "--auth9-secondary": branding.secondary_color,
    "--auth9-bg": branding.background_color,
    "--auth9-text": branding.text_color,
  } as CSSProperties;

  return (
    <BrandingContext.Provider value={branding}>
      <div style={style} className="auth9-theme">
        {/* Inject custom CSS if provided */}
        {branding.custom_css && (
          <style dangerouslySetInnerHTML={{ __html: branding.custom_css }} />
        )}

        {/* Loading state */}
        {loading ? <LoadingSpinner /> : children}
      </div>
    </BrandingContext.Provider>
  );
}

/**
 * Hook to access branding configuration from context.
 */
export function useBrandingContext(): BrandingConfig {
  return useContext(BrandingContext);
}

/**
 * Simple loading spinner shown while branding is being fetched.
 */
function LoadingSpinner() {
  return (
    <div
      style={{
        minHeight: "100vh",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        backgroundColor: "var(--auth9-bg, #f5f5f7)",
      }}
    >
      <div
        style={{
          width: "2rem",
          height: "2rem",
          border: "3px solid #e5e7eb",
          borderTopColor: "var(--auth9-primary, #007AFF)",
          borderRadius: "50%",
          animation: "auth9-spin 0.8s linear infinite",
        }}
      />
      <style>{`
        @keyframes auth9-spin {
          to { transform: rotate(360deg); }
        }
      `}</style>
    </div>
  );
}
