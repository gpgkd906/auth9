//! SCIM Discovery endpoints (ServiceProviderConfig, Schemas, ResourceTypes)

use crate::domain::scim::*;
use crate::domains::provisioning::api::ScimJson;
use crate::domains::provisioning::context::ProvisioningContext;
use axum::response::IntoResponse;

/// GET /ServiceProviderConfig
pub async fn service_provider_config<S: ProvisioningContext>() -> impl IntoResponse {
    ScimJson(ScimServiceProviderConfig::default())
}

/// GET /Schemas
pub async fn schemas<S: ProvisioningContext>() -> impl IntoResponse {
    let user_schema = ScimSchema {
        id: ScimUser::SCHEMA.to_string(),
        name: "User".to_string(),
        description: "User Account".to_string(),
        attributes: vec![
            ScimSchemaAttribute {
                name: "userName".to_string(),
                attr_type: "string".to_string(),
                multi_valued: false,
                required: true,
                mutability: "readWrite".to_string(),
                returned: "default".to_string(),
                uniqueness: "server".to_string(),
            },
            ScimSchemaAttribute {
                name: "displayName".to_string(),
                attr_type: "string".to_string(),
                multi_valued: false,
                required: false,
                mutability: "readWrite".to_string(),
                returned: "default".to_string(),
                uniqueness: "none".to_string(),
            },
            ScimSchemaAttribute {
                name: "active".to_string(),
                attr_type: "boolean".to_string(),
                multi_valued: false,
                required: false,
                mutability: "readWrite".to_string(),
                returned: "default".to_string(),
                uniqueness: "none".to_string(),
            },
            ScimSchemaAttribute {
                name: "emails".to_string(),
                attr_type: "complex".to_string(),
                multi_valued: true,
                required: false,
                mutability: "readWrite".to_string(),
                returned: "default".to_string(),
                uniqueness: "none".to_string(),
            },
        ],
        meta: ScimMeta {
            resource_type: "Schema".to_string(),
            created: None,
            last_modified: None,
            location: None,
        },
    };

    let group_schema = ScimSchema {
        id: ScimGroup::SCHEMA.to_string(),
        name: "Group".to_string(),
        description: "Group".to_string(),
        attributes: vec![
            ScimSchemaAttribute {
                name: "displayName".to_string(),
                attr_type: "string".to_string(),
                multi_valued: false,
                required: true,
                mutability: "readWrite".to_string(),
                returned: "default".to_string(),
                uniqueness: "none".to_string(),
            },
            ScimSchemaAttribute {
                name: "members".to_string(),
                attr_type: "complex".to_string(),
                multi_valued: true,
                required: false,
                mutability: "readWrite".to_string(),
                returned: "default".to_string(),
                uniqueness: "none".to_string(),
            },
        ],
        meta: ScimMeta {
            resource_type: "Schema".to_string(),
            created: None,
            last_modified: None,
            location: None,
        },
    };

    ScimJson(vec![user_schema, group_schema])
}

/// GET /ResourceTypes
pub async fn resource_types<S: ProvisioningContext>() -> impl IntoResponse {
    let types = vec![
        ScimResourceType {
            schemas: vec!["urn:ietf:params:scim:schemas:core:2.0:ResourceType".to_string()],
            id: "User".to_string(),
            name: "User".to_string(),
            endpoint: "/Users".to_string(),
            schema: ScimUser::SCHEMA.to_string(),
            meta: ScimMeta {
                resource_type: "ResourceType".to_string(),
                created: None,
                last_modified: None,
                location: None,
            },
        },
        ScimResourceType {
            schemas: vec!["urn:ietf:params:scim:schemas:core:2.0:ResourceType".to_string()],
            id: "Group".to_string(),
            name: "Group".to_string(),
            endpoint: "/Groups".to_string(),
            schema: ScimGroup::SCHEMA.to_string(),
            meta: ScimMeta {
                resource_type: "ResourceType".to_string(),
                created: None,
                last_modified: None,
                location: None,
            },
        },
    ];

    ScimJson(types)
}
