interface LogoProps {
  logoUrl?: string;
  companyName?: string;
  fallbackText?: string;
}

/**
 * Company branding component - shows logo image or company name.
 */
export function Logo({ logoUrl, companyName, fallbackText }: LogoProps) {
  if (logoUrl) {
    return (
      <img
        src={logoUrl}
        alt={companyName || "Logo"}
        className="login-logo-image"
        referrerPolicy="no-referrer"
        crossOrigin="anonymous"
      />
    );
  }

  if (companyName) {
    return <h1 className="login-title">{companyName}</h1>;
  }

  if (fallbackText) {
    return <h1 className="login-title">{fallbackText}</h1>;
  }

  // Default Auth9 logo
  return <div className="login-logo">A9</div>;
}
