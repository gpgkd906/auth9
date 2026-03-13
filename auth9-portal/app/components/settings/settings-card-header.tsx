import type { ReactNode } from "react";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { cn } from "~/lib/utils";

type SettingsSectionHeadingProps = {
  title: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  className?: string;
};

export function SettingsSectionHeading({
  title,
  description,
  actions,
  className,
}: SettingsSectionHeadingProps) {
  return (
    <div
      className={cn(
        "flex min-h-[5.75rem] flex-col justify-center gap-4 md:flex-row md:items-center md:justify-between",
        className
      )}
    >
      <div className="min-w-0 flex-1 space-y-2">
        <CardTitle>{title}</CardTitle>
        {description ? <CardDescription>{description}</CardDescription> : null}
      </div>
      {actions ? (
        <div className="flex w-full flex-col items-stretch gap-2 sm:flex-row sm:justify-start md:w-auto md:flex-none md:justify-end">
          {actions}
        </div>
      ) : null}
    </div>
  );
}

type SettingsHeroCardProps = SettingsSectionHeadingProps & {
  cardClassName?: string;
  headerClassName?: string;
};

export function SettingsHeroCard({
  title,
  description,
  actions,
  className,
  cardClassName,
  headerClassName,
}: SettingsHeroCardProps) {
  return (
    <Card className={cardClassName}>
      <CardHeader className={cn("p-5 pb-5 sm:p-6 sm:pb-6", headerClassName)}>
        <SettingsSectionHeading
          title={title}
          description={description}
          actions={actions}
          className={className}
        />
      </CardHeader>
    </Card>
  );
}
