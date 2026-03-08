import { LightningBoltIcon, PlusIcon } from "@radix-ui/react-icons";
import { Link } from "react-router";
import { Badge } from "~/components/ui/badge";
import { Button } from "~/components/ui/button";
import { Card, CardContent } from "~/components/ui/card";
import { useI18n } from "~/i18n";
import type { Action } from "~/services/api";

interface ServiceActionsTabProps {
  actions: Action[];
  serviceId: string;
}

export function ServiceActionsTab({ actions, serviceId }: ServiceActionsTabProps) {
  const { t } = useI18n();

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-lg font-semibold">{t("serviceActions.title")}</h3>
          <p className="text-sm text-[var(--text-secondary)]">
            {t("services.detail.tabs.actions", { count: actions.length })}
          </p>
        </div>
        <Button asChild>
          <Link to={`/dashboard/services/${serviceId}/actions/new`}>
            <PlusIcon className="mr-2 h-4 w-4" />
            {t("serviceActions.newAction")}
          </Link>
        </Button>
      </div>

      {actions.length === 0 ? (
        <Card>
          <CardContent className="py-12">
            <div className="text-center">
              <LightningBoltIcon className="mx-auto mb-3 h-8 w-8 text-[var(--text-tertiary)]" />
              <h3 className="mb-2 text-lg font-semibold">{t("serviceActions.noActions")}</h3>
              <p className="mb-4 text-[var(--text-secondary)]">{t("serviceActions.noActionsDescription")}</p>
              <Button asChild>
                <Link to={`/dashboard/services/${serviceId}/actions/new`}>
                  <PlusIcon className="mr-2 h-4 w-4" />
                  {t("serviceActions.createAction")}
                </Link>
              </Button>
            </div>
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-3">
          {actions.map((action) => (
            <Card key={action.id}>
              <div className="p-4">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Link to={`/dashboard/services/${serviceId}/actions/${action.id}`} className="font-medium hover:underline">
                      {action.name}
                    </Link>
                    <Badge variant={action.enabled ? "default" : "secondary"}>
                      {action.enabled ? t("serviceActions.enabled") : t("serviceActions.disabled")}
                    </Badge>
                    <Badge variant="outline">{action.trigger_id}</Badge>
                  </div>
                  <div className="flex items-center gap-2">
                    <Button asChild variant="outline" size="sm">
                      <Link to={`/dashboard/services/${serviceId}/actions/${action.id}`}>
                        {t("serviceActions.viewDetails")}
                      </Link>
                    </Button>
                    <Button asChild variant="outline" size="sm">
                      <Link to={`/dashboard/services/${serviceId}/actions/${action.id}/edit`}>
                        {t("serviceActions.edit")}
                      </Link>
                    </Button>
                  </div>
                </div>
                {action.description && <p className="mt-1 text-sm text-[var(--text-secondary)]">{action.description}</p>}
              </div>
            </Card>
          ))}
          <div className="pt-2 text-center">
            <Button asChild variant="outline">
              <Link to={`/dashboard/services/${serviceId}/actions`}>{t("serviceActions.title")}</Link>
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
