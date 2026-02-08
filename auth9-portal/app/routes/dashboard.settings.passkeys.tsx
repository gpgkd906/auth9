import { redirect } from "react-router";

export function loader() {
  return redirect("/dashboard/account/passkeys");
}

export default function SettingsPasskeysRedirect() {
  return null;
}
