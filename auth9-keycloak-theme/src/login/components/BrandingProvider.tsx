import { createContext, useContext, useEffect, type ReactNode, type CSSProperties } from "react";
import { useBranding, type BrandingConfig, DEFAULT_BRANDING } from "../hooks/useBranding";

const BrandingContext = createContext<BrandingConfig>(DEFAULT_BRANDING);

interface BrandingProviderProps {
  apiUrl: string;
  clientId?: string;
  children: ReactNode;
}

/**
 * Converts a hex color to RGB values for use in rgba() functions.
 */
function hexToRgb(hex: string): { r: number; g: number; b: number } | null {
  const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  return result
    ? {
        r: parseInt(result[1], 16),
        g: parseInt(result[2], 16),
        b: parseInt(result[3], 16),
      }
    : null;
}

/**
 * Provides branding configuration to child components.
 * Fetches configuration from auth9 API and applies CSS variables.
 *
 * Maps branding API colors to Liquid Glass CSS variables:
 * - primary_color → --accent-blue (main action color)
 * - secondary_color → --accent-purple (secondary actions, links)
 * - background_color → --bg-primary (page background)
 * - text_color → --text-primary (main text)
 */
export function BrandingProvider({ apiUrl, clientId, children }: BrandingProviderProps) {
  const { branding, loading } = useBranding(apiUrl, clientId);

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

  // Generate light variants of accent colors
  const primaryRgb = hexToRgb(branding.primary_color);
  const secondaryRgb = hexToRgb(branding.secondary_color);

  // CSS variables for theme colors - both legacy and Liquid Glass
  const style: CSSProperties = {
    // Legacy auth9 variables (backward compatibility)
    "--auth9-primary": branding.primary_color,
    "--auth9-secondary": branding.secondary_color,
    "--auth9-bg": branding.background_color,
    "--auth9-text": branding.text_color,
    // Liquid Glass accent overrides
    "--accent-blue": branding.primary_color,
    "--accent-blue-light": primaryRgb
      ? `rgba(${primaryRgb.r}, ${primaryRgb.g}, ${primaryRgb.b}, 0.12)`
      : "rgba(0, 122, 255, 0.12)",
    "--accent-purple": branding.secondary_color,
    "--accent-purple-light": secondaryRgb
      ? `rgba(${secondaryRgb.r}, ${secondaryRgb.g}, ${secondaryRgb.b}, 0.12)`
      : "rgba(175, 82, 222, 0.12)",
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
