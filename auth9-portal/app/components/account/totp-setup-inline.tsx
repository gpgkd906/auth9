import { useState, useCallback, useRef, useEffect } from "react";
import { Form, useNavigation } from "react-router";
import QRCode from "qrcode";
import { OtpInput } from "~/components/ui/otp-input";
import { Button } from "~/components/ui/button";
import { useI18n } from "~/i18n";
import { hostedLoginApi, type TotpEnrollmentResponse } from "~/services/api";

interface TotpSetupInlineProps {
  accessToken: string;
  onCancel: () => void;
  error?: string;
}

export function TotpSetupInline({ accessToken, onCancel, error }: TotpSetupInlineProps) {
  const { t } = useI18n();
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";
  const formRef = useRef<HTMLFormElement>(null);

  const [enrollment, setEnrollment] = useState<TotpEnrollmentResponse | null>(null);
  const [qrDataUrl, setQrDataUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [enrollError, setEnrollError] = useState<string | null>(null);
  const [showManual, setShowManual] = useState(false);

  const startEnrollment = useCallback(async () => {
    setLoading(true);
    setEnrollError(null);
    try {
      const result = await hostedLoginApi.totpEnrollStart(accessToken);
      setEnrollment(result);
      const dataUrl = await QRCode.toDataURL(result.otpauth_uri, {
        width: 200,
        margin: 2,
        color: { dark: "#1D1D1F", light: "#FFFFFF" },
      });
      setQrDataUrl(dataUrl);
    } catch {
      setEnrollError(t("accountMfa.totp.enrollFailed"));
    } finally {
      setLoading(false);
    }
  }, [accessToken, t]);

  useEffect(() => {
    startEnrollment();
  }, [startEnrollment]);

  const handleOtpComplete = useCallback(
    (code: string) => {
      if (!formRef.current) return;
      const codeInput = formRef.current.querySelector<HTMLInputElement>('input[name="code"]');
      if (codeInput) {
        codeInput.value = code;
        formRef.current.requestSubmit();
      }
    },
    []
  );

  if (loading) {
    return (
      <div className="flex items-center justify-center py-8">
        <div className="h-8 w-8 animate-spin rounded-full border-2 border-[var(--accent-blue)] border-t-transparent" />
      </div>
    );
  }

  if (enrollError) {
    return (
      <div className="space-y-3 py-4">
        <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md">
          {enrollError}
        </div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm" onClick={startEnrollment}>
            {t("accountMfa.totp.retry")}
          </Button>
          <Button variant="ghost" size="sm" onClick={onCancel}>
            {t("accountMfa.totp.cancel")}
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-5 py-4">
      {/* QR Code */}
      <div className="flex justify-center">
        <div className="rounded-2xl border border-[var(--glass-border-subtle)] bg-white p-3">
          <img
            src={qrDataUrl ?? ""}
            alt={t("accountMfa.totp.qrAlt")}
            width={200}
            height={200}
          />
        </div>
      </div>

      {/* Manual Entry Toggle */}
      <div className="text-center">
        <button
          type="button"
          className="text-sm font-medium text-[var(--accent-blue)] hover:underline"
          onClick={() => setShowManual(!showManual)}
        >
          {t("accountMfa.totp.manualEntryToggle")}
        </button>
      </div>

      {showManual && enrollment && (
        <div className="rounded-2xl border border-[var(--glass-border-subtle)] bg-[var(--glass-bg)] p-4">
          <p className="text-xs font-medium uppercase tracking-wider text-[var(--text-tertiary)] mb-2">
            {t("accountMfa.totp.manualEntryLabel")}
          </p>
          <code className="block break-all text-sm font-mono text-[var(--text-primary)] select-all">
            {enrollment.secret}
          </code>
        </div>
      )}

      {/* Verify Code */}
      <div className="space-y-2">
        <p className="text-sm text-center text-[var(--text-secondary)]">
          {t("accountMfa.totp.verifyDescription")}
        </p>
        <Form method="post" ref={formRef} className="space-y-4">
          <input type="hidden" name="intent" value="verify_totp" />
          <input type="hidden" name="setup_token" value={enrollment?.setup_token ?? ""} />
          <input type="hidden" name="code" value="" />

          <OtpInput
            onComplete={handleOtpComplete}
            disabled={isSubmitting}
            error={!!error}
          />

          {error && (
            <div className="rounded-xl border border-[var(--accent-red)]/25 bg-[var(--accent-red)]/12 p-3 text-sm text-[var(--accent-red)]">
              {error}
            </div>
          )}
        </Form>
      </div>

      {/* Cancel */}
      <div className="flex justify-center">
        <Button variant="ghost" size="sm" onClick={onCancel} disabled={isSubmitting}>
          {t("accountMfa.totp.cancel")}
        </Button>
      </div>
    </div>
  );
}
