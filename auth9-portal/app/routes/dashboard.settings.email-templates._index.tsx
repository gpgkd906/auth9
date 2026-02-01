import type { MetaFunction } from "react-router";
import { Link, useLoaderData } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Badge } from "~/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "~/components/ui/table";
import { emailTemplateApi, type EmailTemplateWithContent } from "~/services/api";
import { Pencil1Icon } from "@radix-ui/react-icons";

export const meta: MetaFunction = () => {
  return [{ title: "Email Templates - Auth9" }];
};

export async function loader() {
  try {
    const result = await emailTemplateApi.list();
    return { templates: result.data, error: null };
  } catch (error) {
    const message = error instanceof Error ? error.message : "Failed to load templates";
    return { templates: [] as EmailTemplateWithContent[], error: message };
  }
}

export default function EmailTemplatesPage() {
  const { templates, error } = useLoaderData<typeof loader>();

  return (
    <div className="space-y-6">
      {error && (
        <div className="rounded-xl bg-red-50 border border-red-200 p-4 text-sm text-red-700">
          {error}
        </div>
      )}

      <Card>
        <CardHeader>
          <CardTitle>Email Templates</CardTitle>
          <CardDescription>
            Customize the content and appearance of emails sent to users. Changes take effect immediately.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Template</TableHead>
                <TableHead>Description</TableHead>
                <TableHead className="w-[100px]">Status</TableHead>
                <TableHead className="w-[80px]">Action</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {templates.map((template) => (
                <TableRow key={template.metadata.template_type}>
                  <TableCell className="font-medium">
                    {template.metadata.name}
                  </TableCell>
                  <TableCell className="text-[var(--text-secondary)]">
                    {template.metadata.description}
                  </TableCell>
                  <TableCell>
                    {template.is_customized ? (
                      <Badge variant="default" className="bg-blue-100 text-blue-700 hover:bg-blue-100">
                        Custom
                      </Badge>
                    ) : (
                      <Badge variant="secondary">
                        Default
                      </Badge>
                    )}
                  </TableCell>
                  <TableCell>
                    <Button asChild variant="ghost" size="sm">
                      <Link to={template.metadata.template_type}>
                        <Pencil1Icon className="h-4 w-4 mr-1" />
                        Edit
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
          <CardTitle className="text-base">Template Variables</CardTitle>
          <CardDescription>
            Use variables in your templates with the {"{{variable_name}}"} syntax. Each template type has specific variables available.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="text-sm text-[var(--text-secondary)]">
            <p className="mb-2">Common variables available in all templates:</p>
            <ul className="list-disc list-inside space-y-1 text-[var(--text-secondary)]">
              <li><code className="bg-[var(--sidebar-item-hover)] px-1 rounded">{"{{app_name}}"}</code> - Application name (Auth9)</li>
              <li><code className="bg-[var(--sidebar-item-hover)] px-1 rounded">{"{{year}}"}</code> - Current year</li>
            </ul>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
