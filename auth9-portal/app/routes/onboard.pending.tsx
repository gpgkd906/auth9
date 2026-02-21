import type { MetaFunction, LoaderFunctionArgs } from "react-router";
import { Link } from "react-router";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { requireAuthWithUpdate } from "~/services/session.server";

export const meta: MetaFunction = () => {
  return [{ title: "Pending Activation - Auth9" }];
};

export async function loader({ request }: LoaderFunctionArgs) {
  await requireAuthWithUpdate(request);
  return {};
}

export default function OnboardPending() {
  return (
    <Card className="w-full max-w-lg relative z-10 animate-fade-in-up">
      <CardHeader className="text-center">
        <div className="logo-icon mx-auto mb-4">A9</div>
        <CardTitle className="text-2xl">Pending activation</CardTitle>
        <CardDescription>
          Your organization has been created but is awaiting activation.
        </CardDescription>
      </CardHeader>
      <CardContent className="text-center space-y-4">
        <p className="text-sm text-[var(--text-secondary)]">
          Your email domain doesn&apos;t match the organization domain.
          A platform administrator will review and activate your request.
        </p>

        <div className="flex flex-col gap-2">
          <Button variant="outline" asChild>
            <Link to="/onboard">
              Go back and try another domain
            </Link>
          </Button>
          <Button variant="outline" asChild>
            <Link to="/logout">
              Sign out
            </Link>
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
