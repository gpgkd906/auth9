import { useFormatters } from "~/i18n/format";

/**
 * Client-safe date formatting component.
 *
 * `new Date().toLocaleString()` produces different output on the server
 * (Node.js locale/timezone) vs the browser (user locale/timezone), which
 * causes React hydration error #418.  This wrapper uses
 * `suppressHydrationWarning` so the client value silently wins.
 */
export function FormattedDate({
  date,
  className,
  options,
}: {
  date: string | Date;
  className?: string;
  options?: Intl.DateTimeFormatOptions;
}) {
  const { dateTime } = useFormatters();
  const formatted = dateTime(date, options);

  return (
    <span className={className} suppressHydrationWarning>
      {formatted}
    </span>
  );
}
