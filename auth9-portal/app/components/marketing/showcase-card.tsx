import type { ReactNode } from "react";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { cn } from "~/lib/utils";

type ShowcaseCardProps = {
  title: ReactNode;
  description: ReactNode;
  icon?: ReactNode;
  headerExtra?: ReactNode;
  className?: string;
  contentClassName?: string;
};

export function ShowcaseCard({
  title,
  description,
  icon,
  headerExtra,
  className,
  contentClassName,
}: ShowcaseCardProps) {
  return (
    <Card className={cn("h-full", className)}>
      <CardHeader
        className={cn(
          "flex h-full min-h-[16rem] flex-col p-6 pb-6 sm:p-7 sm:pb-7",
          contentClassName
        )}
      >
        {icon ? <div className="mb-5">{icon}</div> : null}
        <div className="mt-auto space-y-3">
          <div className="flex flex-wrap items-center gap-2">
            <CardTitle>{title}</CardTitle>
            {headerExtra}
          </div>
          <CardDescription className="text-[15px] leading-8">
            {description}
          </CardDescription>
        </div>
      </CardHeader>
    </Card>
  );
}
