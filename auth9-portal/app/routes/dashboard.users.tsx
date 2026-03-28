import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import {
  Form,
  useActionData,
  useLoaderData,
  useNavigate,
  useNavigation,
  useOutletContext,
  useSearchParams,
  useSubmit,
} from "react-router";
import { useEffect, useRef, useState, type FormEvent } from "react";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { CreateUserDialog } from "~/components/users/create-user-dialog";
import { EditUserDialog } from "~/components/users/edit-user-dialog";
import { ManageUserRolesDialog } from "~/components/users/manage-user-roles-dialog";
import { ManageUserTenantsDialog } from "~/components/users/manage-user-tenants-dialog";
import { MfaConfirmationDialog } from "~/components/users/mfa-confirmation-dialog";
import type { TenantInfo, UserTenant } from "~/components/users/types";
import { UsersDirectory } from "~/components/users/users-directory";
import { mapApiError } from "~/lib/error-messages";
import { useConfirm } from "~/hooks/useConfirm";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { translate } from "~/i18n/translate";
import {
  rbacApi,
  serviceApi,
  sessionApi,
  tenantApi,
  userApi,
  type Role,
  type Tenant,
  type TenantUserWithTenant,
  type User,
} from "~/services/api";
import { resolveLocale } from "~/services/locale.server";
import { destroySession, getAccessToken, getSession } from "~/services/session.server";

export const meta: MetaFunction = ({ matches }) => buildMeta(resolveMetaLocale(matches), "usersPage.metaTitle");

export function HydrateFallback() {
  return null;
}

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const perPage = Number(url.searchParams.get("perPage") || "20");
  const search = url.searchParams.get("search") || undefined;
  const accessToken = await getAccessToken(request);

  const [users, tenants, services] = await Promise.all([
    userApi.list(page, perPage, search, accessToken || undefined),
    tenantApi.list(1, 100, undefined, accessToken || undefined),
    serviceApi.list(undefined, 1, 100, accessToken || undefined),
  ]);

  return { users, tenants, services };
}

export async function action({ request }: ActionFunctionArgs) {
  const locale = await resolveLocale(request);
  const formData = await request.formData();
  const intent = formData.get("intent");
  const accessToken = await getAccessToken(request);

  try {
    if (intent === "update_user") {
      const id = formData.get("id") as string;
      const display_name = formData.get("display_name") as string;
      await userApi.update(id, { display_name }, accessToken || undefined);
      return { success: true, intent };
    }

    if (intent === "add_to_tenant") {
      const user_id = formData.get("user_id") as string;
      const tenant_id = formData.get("tenant_id") as string;
      const role_in_tenant = formData.get("role_in_tenant") as string;
      await userApi.addToTenant(user_id, tenant_id, role_in_tenant, accessToken || undefined);
      return { success: true, intent };
    }

    if (intent === "remove_from_tenant") {
      const user_id = formData.get("user_id") as string;
      const tenant_id = formData.get("tenant_id") as string;
      await userApi.removeFromTenant(user_id, tenant_id, accessToken || undefined);
      return { success: true, intent };
    }

    if (intent === "assign_roles") {
      const user_id = formData.get("user_id") as string;
      const tenant_id = formData.get("tenant_id") as string;
      const roles_json = formData.get("roles") as string;
      const service_id = formData.get("service_id") as string | null;
      const roles = JSON.parse(roles_json);

      await rbacApi.assignRoles(
        {
          user_id,
          tenant_id,
          role_ids: roles,
          ...(service_id ? { service_id } : {}),
        },
        accessToken || undefined
      );
      return { success: true, intent };
    }

    if (intent === "create_user") {
      const email = formData.get("email") as string;
      const display_name = formData.get("display_name") as string;
      const password = formData.get("password") as string;
      const tenant_id = formData.get("tenant_id") as string | null;

      await userApi.create(
        { email, display_name, password, ...(tenant_id ? { tenant_id } : {}) },
        accessToken || undefined
      );
      return { success: true, intent };
    }

    if (intent === "unassign_role") {
      const user_id = formData.get("user_id") as string;
      const tenant_id = formData.get("tenant_id") as string;
      const role_id = formData.get("role_id") as string;
      await rbacApi.unassignRole(user_id, tenant_id, role_id, accessToken || undefined);
      return { success: true, intent };
    }

    if (intent === "delete_user") {
      const id = formData.get("id") as string;
      await userApi.delete(id, accessToken || undefined);
      return { success: true, intent };
    }

    if (intent === "force_logout") {
      const id = formData.get("id") as string;
      await sessionApi.forceLogoutUser(id, accessToken || undefined);

      try {
        await userApi.getMe(accessToken || undefined);
      } catch {
        const session = await getSession(request);
        const cookie = session ? await destroySession(session) : undefined;
        const { redirect } = await import("react-router");
        throw redirect("/login", cookie ? { headers: { "Set-Cookie": cookie } } : undefined);
      }

      return { success: true, intent };
    }

    if (intent === "get_user_tenants") {
      const user_id = formData.get("user_id") as string;
      const tenants = await userApi.getTenants(user_id, accessToken || undefined);
      return { success: true, data: tenants.data, intent };
    }

    if (intent === "get_user_assigned_roles") {
      const user_id = formData.get("user_id") as string;
      const tenant_id = formData.get("tenant_id") as string;
      const roles = await rbacApi.getUserAssignedRoles(user_id, tenant_id, accessToken || undefined);
      return { success: true, data: roles.data, intent };
    }

    if (intent === "get_service_roles") {
      const service_id = formData.get("service_id") as string;
      const roles = await rbacApi.listRoles(service_id, accessToken || undefined);
      return { success: true, data: roles.data, intent };
    }

    if (intent === "enable_mfa") {
      const id = formData.get("id") as string;
      const confirm_password = formData.get("confirm_password") as string;
      await userApi.enableMfa(id, confirm_password, accessToken || undefined);
      return { success: true, intent };
    }

    if (intent === "disable_mfa") {
      const id = formData.get("id") as string;
      const confirm_password = formData.get("confirm_password") as string;
      await userApi.disableMfa(id, confirm_password, accessToken || undefined);
      return { success: true, intent };
    }

    if (intent === "update_role_in_tenant") {
      const user_id = formData.get("user_id") as string;
      const tenant_id = formData.get("tenant_id") as string;
      const role_in_tenant = formData.get("role_in_tenant") as string;
      await userApi.updateRoleInTenant(user_id, tenant_id, role_in_tenant, accessToken || undefined);
      return { success: true, intent };
    }
  } catch (error) {
    const message = mapApiError(error, locale);
    return { error: message, intent };
  }

  return { error: translate(locale, "usersPage.invalidIntent"), intent };
}

export default function UsersPage() {
  const { users, tenants, services } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const submit = useSubmit();
  const navigate = useNavigate();
  const confirm = useConfirm();
  const [searchParams] = useSearchParams();
  const { activeTenant } = useOutletContext<{ activeTenant?: TenantUserWithTenant }>();
  const { t } = useI18n();

  const activeTenantId = activeTenant?.tenant_id;
  const currentSearch = searchParams.get("search") || "";
  const isSubmitting = navigation.state === "submitting";
  const createUserError =
    actionData && "error" in actionData && actionData.intent === "create_user" ? String(actionData.error) : null;
  const addToTenantError =
    actionData && "error" in actionData && actionData.intent === "add_to_tenant" ? String(actionData.error) : null;

  const [searchInput, setSearchInput] = useState(currentSearch);
  const [editingUser, setEditingUser] = useState<User | null>(null);
  const [creatingUser, setCreatingUser] = useState(false);
  const [managingTenantsUser, setManagingTenantsUser] = useState<User | null>(null);
  const [managingRoles, setManagingRoles] = useState<{ user: User; tenant: TenantInfo } | null>(null);
  const [mfaAction, setMfaAction] = useState<{ user: User; action: "enable" | "disable" } | null>(null);
  const [mfaError, setMfaError] = useState<string | null>(null);
  const [selectedServiceId, setSelectedServiceId] = useState("");
  const [availableRoles, setAvailableRoles] = useState<Role[]>([]);
  const [assignedRoleIds, setAssignedRoleIds] = useState<Set<string>>(new Set());
  const [allAssignedRoles, setAllAssignedRoles] = useState<Role[]>([]);
  const [userTenants, setUserTenants] = useState<UserTenant[]>([]);
  const [loadingTenants, setLoadingTenants] = useState(false);
  const [tenantsError, setTenantsError] = useState<string | null>(null);
  const createUserButtonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    setSearchInput(currentSearch);
  }, [currentSearch]);

  useEffect(() => {
    if (actionData && "success" in actionData && actionData.success) {
      if (actionData.intent === "update_user") {
        setEditingUser(null);
      }
      if (actionData.intent === "create_user") {
        setCreatingUser(false);
      }
      if (actionData.intent === "assign_roles") {
        setManagingRoles(null);
      }
      if (actionData.intent === "enable_mfa" || actionData.intent === "disable_mfa") {
        setMfaAction(null);
        setMfaError(null);
      }
      if (actionData.intent === "update_role_in_tenant" && managingTenantsUser) {
        const formData = new FormData();
        formData.append("intent", "get_user_tenants");
        formData.append("user_id", managingTenantsUser.id);
        submit(formData, { method: "post" });
      }
      if (actionData.intent === "unassign_role" && managingRoles) {
        const formData = new FormData();
        formData.append("intent", "get_user_assigned_roles");
        formData.append("user_id", managingRoles.user.id);
        formData.append("tenant_id", managingRoles.tenant.id);
        submit(formData, { method: "post" });
      }
      if (actionData.intent === "get_user_assigned_roles" && actionData.data) {
        const roles = actionData.data as Role[];
        setAllAssignedRoles(roles);
        setAssignedRoleIds(new Set(roles.map((role) => role.id)));
      }
      if (actionData.intent === "get_service_roles" && actionData.data) {
        setAvailableRoles(actionData.data as Role[]);
      }
    }

    if (actionData && "error" in actionData && (actionData.intent === "enable_mfa" || actionData.intent === "disable_mfa")) {
      setMfaError(String(actionData.error));
    }
  }, [actionData, managingRoles, managingTenantsUser, submit]);

  useEffect(() => {
    if (managingTenantsUser) {
      setLoadingTenants(true);
      setTenantsError(null);
      const formData = new FormData();
      formData.append("intent", "get_user_tenants");
      formData.append("user_id", managingTenantsUser.id);
      submit(formData, { method: "post" });
    }
  }, [managingTenantsUser, submit]);

  useEffect(() => {
    if (actionData && actionData.intent === "get_user_tenants") {
      setLoadingTenants(false);
      if ("success" in actionData && actionData.success) {
        setUserTenants((actionData.data as UserTenant[]) || []);
      } else if ("error" in actionData) {
        setTenantsError(String(actionData.error));
      }
    }
  }, [actionData]);

  useEffect(() => {
    if (managingRoles) {
      const formData = new FormData();
      formData.append("intent", "get_user_assigned_roles");
      formData.append("user_id", managingRoles.user.id);
      formData.append("tenant_id", managingRoles.tenant.id);
      submit(formData, { method: "post" });
    }
  }, [managingRoles, submit]);

  useEffect(() => {
    if (!managingRoles) {
      setSelectedServiceId("");
      setAvailableRoles([]);
      setAssignedRoleIds(new Set());
      setAllAssignedRoles([]);
    }
  }, [managingRoles]);

  useEffect(() => {
    if (selectedServiceId) {
      const formData = new FormData();
      formData.append("intent", "get_service_roles");
      formData.append("service_id", selectedServiceId);
      submit(formData, { method: "post" });
    } else {
      setAvailableRoles([]);
    }
  }, [selectedServiceId, submit]);

  const handleAssignRoles = () => {
    if (!managingRoles) {
      return;
    }

    const rolesToAdd = Array.from(assignedRoleIds).filter((roleId) =>
      availableRoles.some((availableRole) => availableRole.id === roleId)
    );

    submit(
      {
        intent: "assign_roles",
        user_id: managingRoles.user.id,
        tenant_id: managingRoles.tenant.id,
        roles: JSON.stringify(rolesToAdd),
        ...(selectedServiceId ? { service_id: selectedServiceId } : {}),
      },
      { method: "post" }
    );
  };

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const handleRoleCheckedChange = (roleId: string, checked: boolean, _wasOriginallyAssigned: boolean) => {
    const nextAssignedRoleIds = new Set(assignedRoleIds);

    if (checked) {
      nextAssignedRoleIds.add(roleId);
    } else {
      nextAssignedRoleIds.delete(roleId);
    }

    setAssignedRoleIds(nextAssignedRoleIds);
  };

  const handleSearchSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const params = new URLSearchParams();
    if (searchInput.trim()) {
      params.set("search", searchInput);
    }
    params.set("page", "1");
    navigate(`/dashboard/users?${params.toString()}`);
  };

  const handleClearFilter = () => {
    setSearchInput("");
    navigate("/dashboard/users?page=1");
  };

  const handleForceLogout = async (user: User) => {
    const ok = await confirm({
      title: t("usersPage.forceLogoutTitle"),
      description: t("usersPage.forceLogoutDescription"),
      confirmLabel: t("usersPage.forceLogoutConfirm"),
    });

    if (ok) {
      submit({ intent: "force_logout", id: user.id }, { method: "post" });
    }
  };

  const handleDeleteUser = async (user: User) => {
    const ok = await confirm({
      title: t("usersPage.deleteUserTitle"),
      description: (
        <>
          {t("usersPage.deleteUserDescriptionLead")}{" "}
          <strong className="font-semibold text-[var(--text-primary)]">{user.email}</strong>
          {t("usersPage.deleteUserDescriptionTail")}
        </>
      ),
      variant: "destructive",
    });

    if (ok) {
      submit({ intent: "delete_user", id: user.id }, { method: "post" });
    }
  };

  return (
    <div className="space-y-6">
      <div className="mb-6 flex flex-col justify-between gap-4 sm:flex-row sm:items-center">
        <div>
          <h1 className="mb-2 text-[24px] font-semibold tracking-tight text-[var(--text-primary)]">
            {t("usersPage.title")}
          </h1>
          <p className="text-sm text-[var(--text-secondary)]">{t("usersPage.description")}</p>
        </div>
        <Button
          ref={createUserButtonRef}
          onClick={() => setCreatingUser(true)}
          className="min-h-11 w-full sm:min-h-10 sm:w-auto"
        >
          + {t("usersPage.createUser")}
        </Button>
      </div>

      <Form onSubmit={handleSearchSubmit} className="flex gap-2">
        <Input
          type="text"
          placeholder={t("usersPage.searchPlaceholder")}
          aria-label={t("usersPage.searchAria")}
          value={searchInput}
          onChange={(event) => setSearchInput(event.target.value)}
          className="min-h-11 flex-1 sm:min-h-10"
        />
        <Button type="submit" variant="outline" className="min-h-11 sm:min-h-10">
          {t("usersPage.search")}
        </Button>
      </Form>

      <UsersDirectory
        currentSearch={currentSearch}
        pagination={users.pagination}
        users={users.data}
        onClearFilter={handleClearFilter}
        onDeleteUser={handleDeleteUser}
        onEditUser={setEditingUser}
        onForceLogout={handleForceLogout}
        onManageTenants={setManagingTenantsUser}
        onToggleMfa={({ action, user }) => {
          setMfaAction({ action, user });
          setMfaError(null);
        }}
      />

      <EditUserDialog isSubmitting={isSubmitting} user={editingUser} onOpenChange={(open) => !open && setEditingUser(null)} />

      <CreateUserDialog
        activeTenantId={activeTenantId}
        error={createUserError}
        isSubmitting={isSubmitting}
        open={creatingUser}
        restoreFocusRef={createUserButtonRef}
        tenants={tenants.data}
        onOpenChange={setCreatingUser}
      />

      <ManageUserTenantsDialog
        addToTenantError={addToTenantError}
        loadingTenants={loadingTenants}
        tenants={tenants.data as Tenant[]}
        tenantsError={tenantsError}
        user={managingTenantsUser}
        userTenants={userTenants}
        onManageRoles={(tenant) => managingTenantsUser && setManagingRoles({ user: managingTenantsUser, tenant })}
        onOpenChange={(open) => !open && setManagingTenantsUser(null)}
        onUpdateRoleInTenant={(tenantId, roleInTenant) => {
          if (!managingTenantsUser) {
            return;
          }
          submit(
            {
              intent: "update_role_in_tenant",
              user_id: managingTenantsUser.id,
              tenant_id: tenantId,
              role_in_tenant: roleInTenant,
            },
            { method: "post" }
          );
        }}
      />

      <MfaConfirmationDialog
        error={mfaError}
        isSubmitting={isSubmitting}
        mfaAction={mfaAction}
        onOpenChange={(open) => {
          if (!open) {
            setMfaAction(null);
            setMfaError(null);
          }
        }}
      />

      <ManageUserRolesDialog
        allAssignedRoles={allAssignedRoles}
        availableRoles={availableRoles}
        assignedRoleIds={assignedRoleIds}
        isSubmitting={isSubmitting}
        managingRoles={managingRoles}
        selectedServiceId={selectedServiceId}
        services={services.data}
        setSelectedServiceId={setSelectedServiceId}
        onAssignRoles={handleAssignRoles}
        onOpenChange={(open) => !open && setManagingRoles(null)}
        onRoleCheckedChange={handleRoleCheckedChange}
      />
    </div>
  );
}
