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
}: {
  date: string | Date;
  className?: string;
}) {
  const formatted =
    date instanceof Date
      ? date.toLocaleString()
      : new Date(date).toLocaleString();

  return (
    <span className={className} suppressHydrationWarning>
      {formatted}
    </span>
  );
}
