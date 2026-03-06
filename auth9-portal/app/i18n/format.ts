import { useMemo } from "react";
import { useLocale } from "./index";

export function useFormatters() {
  const { locale } = useLocale();

  return useMemo(() => {
    return {
      date: (value: Date | string, options?: Intl.DateTimeFormatOptions) =>
        new Intl.DateTimeFormat(locale, options).format(new Date(value)),
      dateTime: (value: Date | string, options?: Intl.DateTimeFormatOptions) =>
        new Intl.DateTimeFormat(locale, {
          dateStyle: "medium",
          timeStyle: "short",
          ...options,
        }).format(new Date(value)),
      number: (value: number, options?: Intl.NumberFormatOptions) =>
        new Intl.NumberFormat(locale, options).format(value),
    };
  }, [locale]);
}
