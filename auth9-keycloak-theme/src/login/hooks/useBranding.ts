import { useState, useEffect } from "react";

/**
 * Branding configuration fetched from auth9 API
 */
export interface BrandingConfig {
  logo_url?: string;
  primary_color: string;
  secondary_color: string;
  background_color: string;
  text_color: string;
  custom_css?: string;
  company_name?: string;
  favicon_url?: string;
  allow_registration: boolean;
}

/**
 * Default branding configuration - matches auth9-core defaults
 */
export const DEFAULT_BRANDING: BrandingConfig = {
  primary_color: "#007AFF",
  secondary_color: "#5856D6",
  background_color: "#F5F5F7",
  text_color: "#1D1D1F",
  allow_registration: false,
};

interface UseBrandingResult {
  branding: BrandingConfig;
  loading: boolean;
  error: string | null;
}

/**
 * Hook to fetch branding configuration from auth9 API.
 * Falls back to defaults if the API is unavailable.
 *
 * @param apiUrl - The auth9 API URL from theme properties
 */
export function useBranding(apiUrl: string): UseBrandingResult {
  const [branding, setBranding] = useState<BrandingConfig>(DEFAULT_BRANDING);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();

    async function fetchBranding() {
      try {
        const response = await fetch(`${apiUrl}/api/v1/public/branding`, {
          headers: { Accept: "application/json" },
          signal: controller.signal,
        });

        if (!response.ok) {
          throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        const json = await response.json();

        // auth9 API returns { data: BrandingConfig }
        if (json.data) {
          setBranding(json.data);
        }
      } catch (err) {
        if (err instanceof Error && err.name === "AbortError") {
          return;
        }
        const message = err instanceof Error ? err.message : "Unknown error";
        console.warn(`[Auth9 Theme] Using default branding: ${message}`);
        setError(message);
      } finally {
        setLoading(false);
      }
    }

    fetchBranding();

    return () => controller.abort();
  }, [apiUrl]);

  return { branding, loading, error };
}
