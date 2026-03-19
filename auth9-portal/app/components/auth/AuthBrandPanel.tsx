import type { CSSProperties } from "react";
import type { BrandingConfig } from "~/services/api";

interface AuthBrandPanelProps {
  branding: BrandingConfig;
  eyebrow?: string;
  title: string;
  description: string;
}

export function getBrandMark(companyName: string): string {
  const trimmed = companyName.trim();
  if (trimmed.toLowerCase() === "auth9") return "A9";
  return trimmed.slice(0, 2).toUpperCase();
}

function hexToRgb(hex?: string): string | null {
  if (!hex) return null;
  const match = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  if (!match) return null;
  return `${parseInt(match[1], 16)} ${parseInt(match[2], 16)} ${parseInt(match[3], 16)}`;
}

export function buildAuthBrandStyle(branding: BrandingConfig): CSSProperties {
  const primaryRgb = hexToRgb(branding.primary_color);
  const secondaryRgb = hexToRgb(branding.secondary_color);

  return {
    "--accent-blue": branding.primary_color,
    "--accent-blue-light": primaryRgb ? `rgb(${primaryRgb} / 0.14)` : undefined,
    "--accent-purple": branding.secondary_color,
    "--accent-purple-light": secondaryRgb ? `rgb(${secondaryRgb} / 0.14)` : undefined,
  } as CSSProperties;
}

export function AuthBrandPanel({ branding, eyebrow, title, description }: AuthBrandPanelProps) {
  const companyName = branding.company_name?.trim() || "Auth9";

  return (
    <section className="auth-brand-panel hidden max-w-xl flex-col justify-between rounded-[32px] border border-white/40 bg-white/55 p-8 text-left shadow-xl backdrop-blur-2xl lg:flex">
      <div className="space-y-6">
        <div className="flex items-center gap-4">
          {branding.logo_url ? (
            <img
              src={branding.logo_url}
              alt={companyName}
              className="h-14 w-14 rounded-2xl border border-black/5 bg-white/90 object-contain p-2"
              referrerPolicy="no-referrer"
            />
          ) : (
            <div className="logo-icon m-0">{getBrandMark(companyName)}</div>
          )}
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.28em] text-[var(--text-secondary)]">
              {eyebrow ?? "Hosted Login"}
            </p>
            <h2 className="mt-1 text-2xl font-semibold text-[var(--text-primary)]">{companyName}</h2>
          </div>
        </div>

        <div className="space-y-3">
          <h1 className="text-4xl font-semibold tracking-tight text-[var(--text-primary)]">{title}</h1>
          <p className="max-w-lg text-base leading-7 text-[var(--text-secondary)]">{description}</p>
        </div>
      </div>

      <div className="space-y-4 rounded-[28px] border border-white/50 bg-white/50 p-6">
        <div className="grid gap-3 sm:grid-cols-2">
          <div className="rounded-2xl bg-white/70 p-4">
            <p className="text-xs uppercase tracking-[0.2em] text-[var(--text-tertiary)]">Secure</p>
            <p className="mt-2 text-sm text-[var(--text-primary)]">Password, MFA, SSO, Passkey, and social login — all in one place.</p>
          </div>
          <div className="rounded-2xl bg-white/70 p-4">
            <p className="text-xs uppercase tracking-[0.2em] text-[var(--text-tertiary)]">Customizable</p>
            <p className="mt-2 text-sm text-[var(--text-primary)]">Brand your login pages with your own logo, colors, and domain.</p>
          </div>
        </div>
      </div>
    </section>
  );
}
