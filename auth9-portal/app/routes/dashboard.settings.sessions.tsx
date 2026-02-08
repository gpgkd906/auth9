import { redirect } from "react-router";

export function loader() {
  return redirect("/dashboard/account/sessions");
}

export default function SettingsSessionsRedirect() {
  return null;
}
