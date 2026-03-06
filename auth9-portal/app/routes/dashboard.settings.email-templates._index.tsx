import type { LoaderFunctionArgs, MetaFunction } from "react-router";
import { Link, useLoaderData } from "react-router";
import { Pencil1Icon } from "@radix-ui/react-icons";
import { Badge } from "~/components/ui/badge";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "~/components/ui/table";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { emailTemplateApi, type EmailTemplateWithContent } from "~/services/api";
import { getAccessToken } from "~/services/session.server";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";

export const meta: MetaFunction = ({ matches }) => buildMeta(resolveMetaLocale(matches), "settings.emailTemplatesPage.metaTitle");

export async function loader({ request }: LoaderFunctionArgs) {
  const locale = await resolveLocale(request);
  const accessToken = await getAccessToken(request);

  try {
    const result = await emailTemplateApi.list(accessToken || undefined);
    return { templates: result.data, error: null };
  } catch (error) {
    const message = error instanceof Error ? error.message : translate(locale, "settings.emailTemplatesPage.loadFailed");
    return { templates: [] as EmailTemplateWithContent[], error: message };
  }
}

export default function EmailTemplatesPage() {
  const { templates, error } = useLoaderData<typeof loader>();
  const { t } = useI18n();

  return (
    <div className="space-y-6">
      {error && <div className="rounded-xl border border-red-200 bg-red-50 p-4 text-sm text-red-700">{error}</div>}

      <Card>
        <CardHeader>
          <CardTitle>{t("settings.emailTemplatesPage.title")}</CardTitle>
          <CardDescription>{t("settings.emailTemplatesPage.description")}</CardDescription>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t("settings.emailTemplatesPage.template")}</TableHead>
                <TableHead>{t("settings.emailTemplatesPage.descriptionColumn")}</TableHead>
                <TableHead className="w-[100px]">{t("settings.emailTemplatesPage.status")}</TableHead>
                <TableHead className="w-[80px]">{t("settings.emailTemplatesPage.action")}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {templates.map((template) => (
                <TableRow key={template.metadata.template_type}>
                  <TableCell className="font-medium">{template.metadata.name}</TableCell>
                  <TableCell className="text-[var(--text-secondary)]">{template.metadata.description}</TableCell>
                  <TableCell>
                    {template.is_customized ? (
                      <Badge variant="default" className="bg-blue-100 text-blue-700 hover:bg-blue-100">
                        {t("settings.emailTemplatesPage.customized")}
                      </Badge>
                    ) : (
                      <Badge variant="secondary">{t("settings.emailTemplatesPage.default")}</Badge>
                    )}
                  </TableCell>
                  <TableCell>
                    <Button asChild variant="ghost" size="sm">
                      <Link to={template.metadata.template_type}>
                        <Pencil1Icon className="mr-1 h-4 w-4" />
                        {t("settings.emailTemplatesPage.edit")}
                      </Link>
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-base">{t("settings.emailTemplatesPage.variablesTitle")}</CardTitle>
          <CardDescription>
            {t("settings.emailTemplatesPage.variablesDescription", { syntax: "{{variable_name}}" })}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="text-sm text-[var(--text-secondary)]">
            <p className="mb-2">{t("settings.emailTemplatesPage.commonVariables")}</p>
            <ul className="list-inside list-disc space-y-1 text-[var(--text-secondary)]">
              <li>
                <code className="rounded bg-[var(--sidebar-item-hover)] px-1">{"{{app_name}}"}</code> - {t("settings.emailTemplatesPage.appNameVariable")}
              </li>
              <li>
                <code className="rounded bg-[var(--sidebar-item-hover)] px-1">{"{{year}}"}</code> - {t("settings.emailTemplatesPage.yearVariable")}
              </li>
            </ul>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
