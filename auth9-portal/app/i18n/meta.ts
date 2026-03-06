import type { MetaDescriptor } from "react-router";
import { DEFAULT_LOCALE, type AppLocale } from "./resources";
import { translate } from "./translate";

export function resolveMetaLocale(matches: Array<{ id?: string; data?: unknown }> | undefined): AppLocale {
  const rootMatch = matches?.find((match) => match.id === "root");
  const locale = (rootMatch?.data as { locale?: AppLocale } | undefined)?.locale;
  return locale || DEFAULT_LOCALE;
}

export function buildMeta(
  locale: AppLocale,
  titleKey: string,
  descriptionKey?: string,
  values?: Record<string, string | number>
): MetaDescriptor[] {
  const descriptors: MetaDescriptor[] = [{ title: translate(locale, titleKey, values) }];
  if (descriptionKey) {
    descriptors.push({
      name: "description",
      content: translate(locale, descriptionKey, values),
    });
  }
  return descriptors;
}
