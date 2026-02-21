//! Core SCIM service - orchestrates user/group CRUD operations

use crate::domain::scim::*;
use crate::domain::{CreateUserInput, StringUuid, UpdateUserInput, User};
use crate::error::{AppError, Result};
use crate::keycloak::KeycloakClient;
use crate::repository::scim_group_mapping::ScimGroupRoleMappingRepository;
use crate::repository::scim_log::ScimProvisioningLogRepository;
use crate::repository::UserRepository;
use chrono::{DateTime, Utc};
use std::sync::Arc;

use super::scim_filter::{compile_filter, parse_filter};
use super::scim_mapper::{map_patch_value_to_fields, map_scim_user_to_fields};

pub struct ScimService<U, G, L>
where
    U: UserRepository,
    G: ScimGroupRoleMappingRepository,
    L: ScimProvisioningLogRepository,
{
    user_repo: Arc<U>,
    group_mapping_repo: Arc<G>,
    log_repo: Arc<L>,
    keycloak: Option<KeycloakClient>,
}

impl<U, G, L> ScimService<U, G, L>
where
    U: UserRepository,
    G: ScimGroupRoleMappingRepository,
    L: ScimProvisioningLogRepository,
{
    pub fn new(
        user_repo: Arc<U>,
        group_mapping_repo: Arc<G>,
        log_repo: Arc<L>,
        keycloak: Option<KeycloakClient>,
    ) -> Self {
        Self {
            user_repo,
            group_mapping_repo,
            log_repo,
            keycloak,
        }
    }

    // ============================================================
    // User CRUD
    // ============================================================

    /// Create a new user via SCIM
    pub async fn create_user(
        &self,
        ctx: &ScimRequestContext,
        scim_user: ScimUser,
    ) -> Result<ScimUser> {
        let fields = map_scim_user_to_fields(&scim_user);

        let email = fields
            .email
            .ok_or_else(|| AppError::BadRequest("userName (email) is required".to_string()))?;

        // Check for duplicate
        if let Some(existing) = self.user_repo.find_by_email(&email).await? {
            // If user already exists, check if it's already SCIM-provisioned
            if existing.scim_external_id.is_some() {
                return Err(AppError::Conflict(format!(
                    "User with email {} already exists",
                    email
                )));
            }
            // Link existing user to SCIM
            self.user_repo
                .update_scim_fields(
                    existing.id,
                    fields.external_id.clone(),
                    Some(ctx.connector_id),
                )
                .await?;

            self.log_operation(
                ctx,
                "create",
                "User",
                fields.external_id.as_deref(),
                Some(existing.id),
                "success",
                None,
            )
            .await;

            return self.user_to_scim(&existing, ctx).await;
        }

        // Create in Keycloak first (if available)
        let keycloak_id = if let Some(kc) = &self.keycloak {
            let kc_input = crate::keycloak::CreateKeycloakUserInput {
                username: email.clone(),
                email: email.clone(),
                first_name: None,
                last_name: None,
                enabled: true,
                email_verified: true,
                credentials: None,
            };
            match kc.create_user(&kc_input).await {
                Ok(id) => id,
                Err(e) => {
                    self.log_operation(
                        ctx,
                        "create",
                        "User",
                        fields.external_id.as_deref(),
                        None,
                        "error",
                        Some(&format!("Keycloak error: {}", e)),
                    )
                    .await;
                    return Err(AppError::Keycloak(format!(
                        "Failed to create user in Keycloak: {}",
                        e
                    )));
                }
            }
        } else {
            format!("scim-{}", StringUuid::new_v4())
        };

        // Create in Auth9
        let input = CreateUserInput {
            email: email.clone(),
            display_name: fields.display_name.clone(),
            avatar_url: fields.avatar_url.clone(),
        };

        let user = self.user_repo.create(&keycloak_id, &input).await?;

        // Set SCIM fields
        self.user_repo
            .update_scim_fields(user.id, fields.external_id.clone(), Some(ctx.connector_id))
            .await?;

        // Handle active=false → lock the user
        if fields.active == Some(false) {
            let far_future = DateTime::parse_from_rfc3339("2037-12-31T23:59:59Z")
                .unwrap()
                .with_timezone(&Utc);
            self.user_repo
                .update_locked_until(user.id, Some(far_future))
                .await?;
        }

        self.log_operation(
            ctx,
            "create",
            "User",
            fields.external_id.as_deref(),
            Some(user.id),
            "success",
            None,
        )
        .await;

        // Re-fetch to get updated fields
        let updated_user =
            self.user_repo.find_by_id(user.id).await?.ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!("User not found after creation"))
            })?;

        self.user_to_scim(&updated_user, ctx).await
    }

    /// Get a user by auth9 ID
    pub async fn get_user(
        &self,
        user_id: StringUuid,
        ctx: &ScimRequestContext,
    ) -> Result<ScimUser> {
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User {} not found", user_id)))?;

        self.user_to_scim(&user, ctx).await
    }

    /// Replace (PUT) a user
    pub async fn replace_user(
        &self,
        user_id: StringUuid,
        ctx: &ScimRequestContext,
        scim_user: ScimUser,
    ) -> Result<ScimUser> {
        let existing = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User {} not found", user_id)))?;

        let fields = map_scim_user_to_fields(&scim_user);

        let update_input = UpdateUserInput {
            display_name: fields.display_name.clone(),
            avatar_url: fields.avatar_url.clone(),
        };
        self.user_repo.update(user_id, &update_input).await?;

        // Update SCIM fields
        if fields.external_id.is_some() {
            self.user_repo
                .update_scim_fields(
                    user_id,
                    fields.external_id.clone(),
                    existing.scim_provisioned_by,
                )
                .await?;
        }

        // Handle active flag
        if let Some(active) = fields.active {
            let locked = if active {
                None
            } else {
                Some(
                    DateTime::parse_from_rfc3339("2037-12-31T23:59:59Z")
                        .unwrap()
                        .with_timezone(&Utc),
                )
            };
            self.user_repo.update_locked_until(user_id, locked).await?;
        }

        self.log_operation(
            ctx,
            "replace",
            "User",
            fields.external_id.as_deref(),
            Some(user_id),
            "success",
            None,
        )
        .await;

        let updated =
            self.user_repo.find_by_id(user_id).await?.ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!("User not found after update"))
            })?;
        self.user_to_scim(&updated, ctx).await
    }

    /// Patch a user (incremental update)
    pub async fn patch_user(
        &self,
        user_id: StringUuid,
        ctx: &ScimRequestContext,
        patch: ScimPatchOp,
    ) -> Result<ScimUser> {
        let existing = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User {} not found", user_id)))?;

        for operation in &patch.operations {
            let op = operation.op.to_lowercase();
            match op.as_str() {
                "replace" | "add" => {
                    if let Some(value) = &operation.value {
                        let fields = map_patch_value_to_fields(operation.path.as_deref(), value);
                        self.apply_mapped_fields(user_id, &fields, &existing)
                            .await?;
                    }
                }
                "remove" => {
                    // Handle remove operations (e.g., remove displayName)
                    if let Some(path) = &operation.path {
                        match path.as_str() {
                            "displayName" => {
                                let input = UpdateUserInput {
                                    display_name: None,
                                    avatar_url: None,
                                };
                                self.user_repo.update(user_id, &input).await?;
                            }
                            "photos" => {
                                let input = UpdateUserInput {
                                    display_name: None,
                                    avatar_url: None,
                                };
                                self.user_repo.update(user_id, &input).await?;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {
                    return Err(AppError::BadRequest(format!(
                        "Unknown SCIM patch operation: {}",
                        op
                    )));
                }
            }
        }

        self.log_operation(
            ctx,
            "patch",
            "User",
            existing.scim_external_id.as_deref(),
            Some(user_id),
            "success",
            None,
        )
        .await;

        let updated = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("User not found after patch")))?;
        self.user_to_scim(&updated, ctx).await
    }

    /// Delete (deactivate) a user - sets locked_until
    pub async fn delete_user(&self, user_id: StringUuid, ctx: &ScimRequestContext) -> Result<()> {
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User {} not found", user_id)))?;

        // Soft-delete: lock the user
        let far_future = DateTime::parse_from_rfc3339("2037-12-31T23:59:59Z")
            .unwrap()
            .with_timezone(&Utc);
        self.user_repo
            .update_locked_until(user_id, Some(far_future))
            .await?;

        self.log_operation(
            ctx,
            "delete",
            "User",
            user.scim_external_id.as_deref(),
            Some(user_id),
            "success",
            None,
        )
        .await;

        Ok(())
    }

    /// List users with optional SCIM filter
    pub async fn list_users(
        &self,
        ctx: &ScimRequestContext,
        filter: Option<&str>,
        start_index: i64,
        count: i64,
    ) -> Result<ScimListResponse<ScimUser>> {
        let offset = (start_index - 1).max(0);

        if let Some(filter_str) = filter {
            let expr = parse_filter(filter_str)?;
            let compiled = compile_filter(&expr)?;

            // Build dynamic query
            // For simplicity, we handle the common case: single userName eq filter
            // For complex filters, we fall back to listing all + filtering
            if compiled.bindings.len() == 1
                && compiled.where_clause.contains("users.email = ?")
                && !compiled.where_clause.contains("AND")
                && !compiled.where_clause.contains("OR")
            {
                // Optimized: single email lookup
                if let Some(user) = self.user_repo.find_by_email(&compiled.bindings[0]).await? {
                    let scim_user = self.user_to_scim(&user, ctx).await?;
                    return Ok(ScimListResponse::new(
                        vec![scim_user],
                        1,
                        start_index,
                        count,
                    ));
                }
                return Ok(ScimListResponse::new(vec![], 0, start_index, count));
            }

            // For externalId eq filter
            if compiled.bindings.len() == 1
                && compiled.where_clause.contains("users.scim_external_id = ?")
                && !compiled.where_clause.contains("AND")
                && !compiled.where_clause.contains("OR")
            {
                if let Some(user) = self
                    .user_repo
                    .find_by_scim_external_id(compiled.bindings[0].clone())
                    .await?
                {
                    let scim_user = self.user_to_scim(&user, ctx).await?;
                    return Ok(ScimListResponse::new(
                        vec![scim_user],
                        1,
                        start_index,
                        count,
                    ));
                }
                return Ok(ScimListResponse::new(vec![], 0, start_index, count));
            }

            // Generic: list + in-memory filter (for more complex queries)
            // In production, you'd want to generate dynamic SQL, but for now we use
            // search with the first binding as a pattern
            if let Some(first_binding) = compiled.bindings.first() {
                let users = self.user_repo.search(first_binding, offset, count).await?;
                let total = self.user_repo.search_count(first_binding).await?;
                let mut scim_users = Vec::new();
                for user in users {
                    scim_users.push(self.user_to_scim(&user, ctx).await?);
                }
                return Ok(ScimListResponse::new(scim_users, total, start_index, count));
            }
        }

        // No filter: list all users
        let users = self.user_repo.list(offset, count).await?;
        let total = self.user_repo.count().await?;
        let mut scim_users = Vec::new();
        for user in users {
            scim_users.push(self.user_to_scim(&user, ctx).await?);
        }
        Ok(ScimListResponse::new(scim_users, total, start_index, count))
    }

    // ============================================================
    // Group CRUD
    // ============================================================

    /// Create a SCIM group (maps to Auth9 role via ScimGroupRoleMapping)
    pub async fn create_group(
        &self,
        ctx: &ScimRequestContext,
        group: ScimGroup,
    ) -> Result<ScimGroup> {
        let scim_group_id = group
            .external_id
            .clone()
            .unwrap_or_else(|| StringUuid::new_v4().to_string());

        // Check if mapping already exists
        if self
            .group_mapping_repo
            .find_by_scim_group(ctx.connector_id, &scim_group_id)
            .await?
            .is_some()
        {
            return Err(AppError::Conflict(format!(
                "Group {} already exists",
                group.display_name
            )));
        }

        // Create a mapping (role_id is a placeholder - admin should configure mappings)
        let mapping = ScimGroupRoleMapping {
            id: StringUuid::new_v4(),
            tenant_id: ctx.tenant_id,
            connector_id: ctx.connector_id,
            scim_group_id: scim_group_id.clone(),
            scim_group_display_name: Some(group.display_name.clone()),
            role_id: StringUuid::new_v4(), // Placeholder role
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.group_mapping_repo.upsert(&mapping).await?;

        self.log_operation(
            ctx,
            "create",
            "Group",
            Some(&scim_group_id),
            Some(mapping.id),
            "success",
            None,
        )
        .await;

        Ok(ScimGroup {
            schemas: vec![ScimGroup::SCHEMA.to_string()],
            id: Some(mapping.id.to_string()),
            external_id: Some(scim_group_id),
            display_name: group.display_name,
            members: group.members,
            meta: Some(ScimMeta {
                resource_type: "Group".to_string(),
                created: Some(Utc::now().to_rfc3339()),
                last_modified: Some(Utc::now().to_rfc3339()),
                location: Some(format!("{}/Groups/{}", ctx.base_url, mapping.id)),
            }),
        })
    }

    /// Get a SCIM group by mapping ID
    pub async fn get_group(
        &self,
        group_id: StringUuid,
        ctx: &ScimRequestContext,
    ) -> Result<ScimGroup> {
        let mappings = self
            .group_mapping_repo
            .list_by_connector(ctx.connector_id)
            .await?;

        let mapping = mappings
            .iter()
            .find(|m| m.id == group_id)
            .ok_or_else(|| AppError::NotFound(format!("Group {} not found", group_id)))?;

        Ok(ScimGroup {
            schemas: vec![ScimGroup::SCHEMA.to_string()],
            id: Some(mapping.id.to_string()),
            external_id: Some(mapping.scim_group_id.clone()),
            display_name: mapping.scim_group_display_name.clone().unwrap_or_default(),
            members: vec![], // Members would need to be resolved from user_tenant_roles
            meta: Some(ScimMeta {
                resource_type: "Group".to_string(),
                created: Some(mapping.created_at.to_rfc3339()),
                last_modified: Some(mapping.updated_at.to_rfc3339()),
                location: Some(format!("{}/Groups/{}", ctx.base_url, mapping.id)),
            }),
        })
    }

    /// Patch a SCIM group
    pub async fn patch_group(
        &self,
        group_id: StringUuid,
        ctx: &ScimRequestContext,
        patch: ScimPatchOp,
    ) -> Result<ScimGroup> {
        // For now, just acknowledge the patch - member management requires RBAC integration
        for operation in &patch.operations {
            let op = operation.op.to_lowercase();
            match op.as_str() {
                "add" | "replace" | "remove" => {
                    // Log the operation for now
                    tracing::info!(
                        "SCIM Group PATCH: op={}, path={:?}, group_id={}",
                        op,
                        operation.path,
                        group_id
                    );
                }
                _ => {
                    return Err(AppError::BadRequest(format!(
                        "Unknown patch operation: {}",
                        op
                    )));
                }
            }
        }

        self.log_operation(ctx, "patch", "Group", None, Some(group_id), "success", None)
            .await;

        self.get_group(group_id, ctx).await
    }

    /// Delete a SCIM group mapping
    pub async fn delete_group(&self, group_id: StringUuid, ctx: &ScimRequestContext) -> Result<()> {
        self.group_mapping_repo.delete(group_id).await?;

        self.log_operation(
            ctx,
            "delete",
            "Group",
            None,
            Some(group_id),
            "success",
            None,
        )
        .await;

        Ok(())
    }

    /// List SCIM groups
    pub async fn list_groups(
        &self,
        ctx: &ScimRequestContext,
        start_index: i64,
        count: i64,
    ) -> Result<ScimListResponse<ScimGroup>> {
        let mappings = self
            .group_mapping_repo
            .list_by_connector(ctx.connector_id)
            .await?;

        let total = mappings.len() as i64;
        let offset = (start_index - 1).max(0) as usize;
        let groups: Vec<ScimGroup> = mappings
            .into_iter()
            .skip(offset)
            .take(count as usize)
            .map(|m| ScimGroup {
                schemas: vec![ScimGroup::SCHEMA.to_string()],
                id: Some(m.id.to_string()),
                external_id: Some(m.scim_group_id),
                display_name: m.scim_group_display_name.unwrap_or_default(),
                members: vec![],
                meta: Some(ScimMeta {
                    resource_type: "Group".to_string(),
                    created: Some(m.created_at.to_rfc3339()),
                    last_modified: Some(m.updated_at.to_rfc3339()),
                    location: Some(format!("{}/Groups/{}", ctx.base_url, m.id)),
                }),
            })
            .collect();

        Ok(ScimListResponse::new(groups, total, start_index, count))
    }

    // ============================================================
    // Bulk Operations
    // ============================================================

    pub async fn process_bulk(
        &self,
        ctx: &ScimRequestContext,
        request: ScimBulkRequest,
    ) -> Result<ScimBulkResponse> {
        let mut responses = Vec::new();
        let fail_on_errors = request.fail_on_errors.unwrap_or(0);
        let mut error_count = 0;

        for op in request.operations {
            let result = self.process_bulk_operation(ctx, &op).await;
            let response = match result {
                Ok(resp) => resp,
                Err(e) => {
                    error_count += 1;
                    if fail_on_errors > 0 && error_count >= fail_on_errors {
                        // Stop processing
                        responses.push(ScimBulkOperationResponse {
                            method: op.method.clone(),
                            bulk_id: op.bulk_id.clone(),
                            location: None,
                            status: "500".to_string(),
                            response: Some(
                                serde_json::to_value(ScimError::internal(e.to_string()))
                                    .unwrap_or_default(),
                            ),
                        });
                        break;
                    }
                    ScimBulkOperationResponse {
                        method: op.method.clone(),
                        bulk_id: op.bulk_id.clone(),
                        location: None,
                        status: "400".to_string(),
                        response: Some(
                            serde_json::to_value(ScimError::bad_request(e.to_string()))
                                .unwrap_or_default(),
                        ),
                    }
                }
            };
            responses.push(response);
        }

        Ok(ScimBulkResponse {
            schemas: vec![ScimBulkResponse::SCHEMA.to_string()],
            operations: responses,
        })
    }

    // ============================================================
    // Helper Methods
    // ============================================================

    async fn process_bulk_operation(
        &self,
        ctx: &ScimRequestContext,
        op: &ScimBulkOperation,
    ) -> Result<ScimBulkOperationResponse> {
        let method = op.method.to_uppercase();
        let path = &op.path;

        match method.as_str() {
            "POST" if path.contains("/Users") => {
                if let Some(data) = &op.data {
                    let scim_user: ScimUser = serde_json::from_value(data.clone())
                        .map_err(|e| AppError::BadRequest(format!("Invalid User data: {}", e)))?;
                    let result = self.create_user(ctx, scim_user).await?;
                    Ok(ScimBulkOperationResponse {
                        method: op.method.clone(),
                        bulk_id: op.bulk_id.clone(),
                        location: result.meta.as_ref().and_then(|m| m.location.clone()),
                        status: "201".to_string(),
                        response: None,
                    })
                } else {
                    Err(AppError::BadRequest("Missing data for POST".to_string()))
                }
            }
            "DELETE" if path.contains("/Users/") => {
                let id_str = path.rsplit('/').next().unwrap_or("");
                let user_id = StringUuid::parse_str(id_str)
                    .map_err(|_| AppError::BadRequest(format!("Invalid user ID: {}", id_str)))?;
                self.delete_user(user_id, ctx).await?;
                Ok(ScimBulkOperationResponse {
                    method: op.method.clone(),
                    bulk_id: op.bulk_id.clone(),
                    location: None,
                    status: "204".to_string(),
                    response: None,
                })
            }
            _ => Err(AppError::BadRequest(format!(
                "Unsupported bulk operation: {} {}",
                method, path
            ))),
        }
    }

    async fn apply_mapped_fields(
        &self,
        user_id: StringUuid,
        fields: &MappedUserFields,
        existing: &User,
    ) -> Result<()> {
        // Update basic fields
        if fields.display_name.is_some() || fields.avatar_url.is_some() {
            let input = UpdateUserInput {
                display_name: fields.display_name.clone(),
                avatar_url: fields.avatar_url.clone(),
            };
            self.user_repo.update(user_id, &input).await?;
        }

        // Update SCIM external ID
        if fields.external_id.is_some() {
            self.user_repo
                .update_scim_fields(
                    user_id,
                    fields.external_id.clone(),
                    existing.scim_provisioned_by,
                )
                .await?;
        }

        // Handle active flag
        if let Some(active) = fields.active {
            let locked = if active {
                None
            } else {
                Some(
                    DateTime::parse_from_rfc3339("2037-12-31T23:59:59Z")
                        .unwrap()
                        .with_timezone(&Utc),
                )
            };
            self.user_repo.update_locked_until(user_id, locked).await?;
        }

        Ok(())
    }

    /// Convert an Auth9 User to a SCIM User resource
    async fn user_to_scim(&self, user: &User, ctx: &ScimRequestContext) -> Result<ScimUser> {
        Ok(ScimUser {
            schemas: vec![ScimUser::SCHEMA.to_string()],
            id: Some(user.id.to_string()),
            external_id: user.scim_external_id.clone(),
            user_name: user.email.clone(),
            name: user.display_name.as_ref().map(|dn| {
                let parts: Vec<&str> = dn.splitn(2, ' ').collect();
                ScimName {
                    given_name: parts.first().map(|s| s.to_string()),
                    family_name: if parts.len() > 1 {
                        Some(parts[1].to_string())
                    } else {
                        None
                    },
                    formatted: Some(dn.clone()),
                }
            }),
            display_name: user.display_name.clone(),
            emails: vec![ScimEmail {
                value: user.email.clone(),
                email_type: Some("work".to_string()),
                primary: true,
            }],
            photos: user
                .avatar_url
                .as_ref()
                .map(|url| {
                    vec![ScimPhoto {
                        value: url.clone(),
                        photo_type: Some("photo".to_string()),
                    }]
                })
                .unwrap_or_default(),
            active: user.locked_until.is_none(),
            meta: Some(ScimMeta {
                resource_type: "User".to_string(),
                created: Some(user.created_at.to_rfc3339()),
                last_modified: Some(user.updated_at.to_rfc3339()),
                location: Some(format!("{}/Users/{}", ctx.base_url, user.id)),
            }),
            groups: vec![],
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn log_operation(
        &self,
        ctx: &ScimRequestContext,
        operation: &str,
        resource_type: &str,
        scim_resource_id: Option<&str>,
        auth9_resource_id: Option<StringUuid>,
        status: &str,
        error_detail: Option<&str>,
    ) {
        let input = CreateScimLogInput {
            tenant_id: ctx.tenant_id,
            connector_id: ctx.connector_id,
            operation: operation.to_string(),
            resource_type: resource_type.to_string(),
            scim_resource_id: scim_resource_id.map(|s| s.to_string()),
            auth9_resource_id,
            status: status.to_string(),
            error_detail: error_detail.map(|s| s.to_string()),
            response_status: None,
        };
        if let Err(e) = self.log_repo.create(&input).await {
            tracing::warn!("Failed to log SCIM operation: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::scim_group_mapping::MockScimGroupRoleMappingRepository;
    use crate::repository::scim_log::MockScimProvisioningLogRepository;
    use crate::repository::user::MockUserRepository;

    fn make_ctx() -> ScimRequestContext {
        ScimRequestContext {
            tenant_id: StringUuid::new_v4(),
            connector_id: StringUuid::new_v4(),
            token_id: StringUuid::new_v4(),
            base_url: "https://example.com/api/v1/scim/v2".to_string(),
        }
    }

    fn make_user(email: &str) -> User {
        User {
            id: StringUuid::new_v4(),
            email: email.to_string(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_get_user_not_found() {
        let mut user_mock = MockUserRepository::new();
        user_mock.expect_find_by_id().returning(|_| Ok(None));

        let group_mock = MockScimGroupRoleMappingRepository::new();
        let log_mock = MockScimProvisioningLogRepository::new();

        let service = ScimService::new(
            Arc::new(user_mock),
            Arc::new(group_mock),
            Arc::new(log_mock),
            None,
        );

        let ctx = make_ctx();
        let result = service.get_user(StringUuid::new_v4(), &ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_user_success() {
        let user = make_user("test@example.com");
        let user_id = user.id;
        let mut user_mock = MockUserRepository::new();
        user_mock
            .expect_find_by_id()
            .returning(move |_| Ok(Some(user.clone())));

        let group_mock = MockScimGroupRoleMappingRepository::new();
        let log_mock = MockScimProvisioningLogRepository::new();

        let service = ScimService::new(
            Arc::new(user_mock),
            Arc::new(group_mock),
            Arc::new(log_mock),
            None,
        );

        let ctx = make_ctx();
        let result = service.get_user(user_id, &ctx).await.unwrap();
        assert_eq!(result.user_name, "test@example.com");
        assert_eq!(result.id, Some(user_id.to_string()));
        assert!(result.active);
    }

    #[tokio::test]
    async fn test_delete_user_sets_locked() {
        let user = make_user("test@example.com");
        let user_id = user.id;

        let mut user_mock = MockUserRepository::new();
        user_mock
            .expect_find_by_id()
            .returning(move |_| Ok(Some(user.clone())));
        user_mock
            .expect_update_locked_until()
            .returning(|_, _| Ok(()));

        let group_mock = MockScimGroupRoleMappingRepository::new();
        let mut log_mock = MockScimProvisioningLogRepository::new();
        log_mock.expect_create().returning(|_| {
            Ok(ScimProvisioningLog {
                id: StringUuid::new_v4(),
                tenant_id: StringUuid::new_v4(),
                connector_id: StringUuid::new_v4(),
                operation: "delete".to_string(),
                resource_type: "User".to_string(),
                scim_resource_id: None,
                auth9_resource_id: None,
                status: "success".to_string(),
                error_detail: None,
                response_status: None,
                created_at: Utc::now(),
            })
        });

        let service = ScimService::new(
            Arc::new(user_mock),
            Arc::new(group_mock),
            Arc::new(log_mock),
            None,
        );

        let ctx = make_ctx();
        let result = service.delete_user(user_id, &ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_users_no_filter() {
        let user1 = make_user("a@example.com");
        let user2 = make_user("b@example.com");

        let mut user_mock = MockUserRepository::new();
        let users = vec![user1, user2];
        user_mock
            .expect_list()
            .returning(move |_, _| Ok(users.clone()));
        user_mock.expect_count().returning(|| Ok(2));

        let group_mock = MockScimGroupRoleMappingRepository::new();
        let log_mock = MockScimProvisioningLogRepository::new();

        let service = ScimService::new(
            Arc::new(user_mock),
            Arc::new(group_mock),
            Arc::new(log_mock),
            None,
        );

        let ctx = make_ctx();
        let result = service.list_users(&ctx, None, 1, 100).await.unwrap();
        assert_eq!(result.total_results, 2);
        assert_eq!(result.resources.len(), 2);
    }

    #[tokio::test]
    async fn test_list_users_with_email_filter() {
        let user = make_user("john@example.com");

        let mut user_mock = MockUserRepository::new();
        let user_clone = user.clone();
        user_mock
            .expect_find_by_email()
            .returning(move |_| Ok(Some(user_clone.clone())));

        let group_mock = MockScimGroupRoleMappingRepository::new();
        let log_mock = MockScimProvisioningLogRepository::new();

        let service = ScimService::new(
            Arc::new(user_mock),
            Arc::new(group_mock),
            Arc::new(log_mock),
            None,
        );

        let ctx = make_ctx();
        let result = service
            .list_users(&ctx, Some("userName eq \"john@example.com\""), 1, 100)
            .await
            .unwrap();
        assert_eq!(result.total_results, 1);
        assert_eq!(result.resources[0].user_name, "john@example.com");
    }
}
