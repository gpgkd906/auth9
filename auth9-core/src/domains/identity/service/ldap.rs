//! LDAP/Active Directory authentication service.

use crate::domains::tenant_access::api::tenant_sso::ConnectorTestResult;
use crate::error::{AppError, Result};
use crate::models::ldap::{escape_ldap_search_filter, LdapConfig, LdapUserProfile};
#[cfg(test)]
use crate::models::ldap::SearchScope;
use async_trait::async_trait;
use ldap3::{drive, LdapConnAsync, LdapConnSettings, Scope};
use std::time::Duration;

// ── Trait (mockable for tests) ──

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait LdapAuthenticator: Send + Sync {
    async fn authenticate(
        &self,
        config: &LdapConfig,
        username: &str,
        password: &str,
    ) -> Result<LdapUserProfile>;

    async fn test_connection(&self, config: &LdapConfig) -> Result<ConnectorTestResult>;

    async fn search_users(
        &self,
        config: &LdapConfig,
        query: &str,
        limit: u32,
    ) -> Result<Vec<LdapUserProfile>>;
}

// ── Default Implementation ──

pub struct DefaultLdapAuthenticator;

impl DefaultLdapAuthenticator {
    pub fn new() -> Self {
        Self
    }

    async fn connect(config: &LdapConfig) -> Result<ldap3::Ldap> {
        let settings = LdapConnSettings::new()
            .set_conn_timeout(Duration::from_secs(config.connection_timeout_secs));

        let (conn, ldap) = LdapConnAsync::with_settings(settings, &config.server_url)
            .await
            .map_err(|e| {
                AppError::BadRequest(format!("Failed to connect to LDAP server: {}", e))
            })?;
        drive!(conn);

        // Note: ldaps:// connections use implicit TLS (handled by ldap3).
        // STARTTLS for ldap:// connections is not yet supported —
        // use ldaps:// URLs for production deployments.

        Ok(ldap)
    }

    async fn service_bind(ldap: &mut ldap3::Ldap, config: &LdapConfig) -> Result<()> {
        let result = ldap
            .simple_bind(&config.bind_dn, &config.bind_password)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("LDAP bind error: {}", e)))?;

        result.success().map_err(|e| {
            AppError::BadRequest(format!(
                "LDAP service account bind failed (check bindDn/bindPassword): {}",
                e
            ))
        })?;
        Ok(())
    }

    fn build_search_filter(config: &LdapConfig, username: &str) -> String {
        let escaped = escape_ldap_search_filter(username);
        config
            .user_search_filter
            .replace("{username}", &escaped)
    }

    fn build_search_attrs(config: &LdapConfig) -> Vec<&str> {
        let mut attrs = vec![
            "dn",
            config.attr_username.as_str(),
            config.attr_email.as_str(),
            config.attr_first_name.as_str(),
            config.attr_last_name.as_str(),
        ];
        if let Some(ref a) = config.attr_display_name {
            attrs.push(a.as_str());
        }
        if let Some(ref a) = config.attr_phone {
            attrs.push(a.as_str());
        }
        if let Some(ref a) = config.attr_groups {
            attrs.push(a.as_str());
        }
        attrs
    }

    fn extract_profile(
        config: &LdapConfig,
        entry: &ldap3::SearchEntry,
    ) -> LdapUserProfile {
        let get_first = |attr: &str| -> Option<String> {
            entry
                .attrs
                .get(attr)
                .and_then(|vals| vals.first())
                .cloned()
        };

        let groups = config
            .attr_groups
            .as_ref()
            .and_then(|attr| entry.attrs.get(attr.as_str()))
            .cloned()
            .unwrap_or_default();

        let username = get_first(&config.attr_username).unwrap_or_default();

        let display_name = config
            .attr_display_name
            .as_ref()
            .and_then(|a| get_first(a))
            .or_else(|| {
                let first = get_first(&config.attr_first_name);
                let last = get_first(&config.attr_last_name);
                match (first, last) {
                    (Some(f), Some(l)) => Some(format!("{} {}", f, l)),
                    (Some(f), None) => Some(f),
                    (None, Some(l)) => Some(l),
                    (None, None) => None,
                }
            });

        LdapUserProfile {
            dn: entry.dn.clone(),
            username,
            email: get_first(&config.attr_email),
            first_name: get_first(&config.attr_first_name),
            last_name: get_first(&config.attr_last_name),
            display_name,
            phone: config.attr_phone.as_ref().and_then(|a| get_first(a)),
            groups,
        }
    }
}

#[async_trait]
impl LdapAuthenticator for DefaultLdapAuthenticator {
    async fn authenticate(
        &self,
        config: &LdapConfig,
        username: &str,
        password: &str,
    ) -> Result<LdapUserProfile> {
        // 1. Connect and bind with service account
        let mut ldap = Self::connect(config).await?;
        Self::service_bind(&mut ldap, config).await?;

        // 2. Search for user DN
        let filter = Self::build_search_filter(config, username);
        let attrs = Self::build_search_attrs(config);
        let scope = config.user_search_scope.to_ldap3_scope();

        let (results, _result) = ldap
            .search(&config.base_dn, scope, &filter, attrs)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("LDAP search error: {}", e)))?
            .success()
            .map_err(|e| AppError::Internal(anyhow::anyhow!("LDAP search failed: {}", e)))?;

        if results.is_empty() {
            let _ = ldap.unbind().await;
            return Err(AppError::Unauthorized(
                "Invalid LDAP credentials".to_string(),
            ));
        }

        let search_entry = ldap3::SearchEntry::construct(results.into_iter().next().unwrap());

        // 3. Attempt user bind (verify password)
        let user_bind_result = ldap
            .simple_bind(&search_entry.dn, password)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("LDAP user bind error: {}", e)))?;

        if user_bind_result.rc != 0 {
            let _ = ldap.unbind().await;
            return Err(AppError::Unauthorized(
                "Invalid LDAP credentials".to_string(),
            ));
        }

        // 4. Extract profile from search result
        let profile = Self::extract_profile(config, &search_entry);
        let _ = ldap.unbind().await;

        Ok(profile)
    }

    async fn test_connection(&self, config: &LdapConfig) -> Result<ConnectorTestResult> {
        // Connect and bind with service account
        let mut ldap = match Self::connect(config).await {
            Ok(l) => l,
            Err(e) => {
                return Ok(ConnectorTestResult {
                    ok: false,
                    message: format!("Connection failed: {}", e),
                });
            }
        };

        if let Err(e) = Self::service_bind(&mut ldap, config).await {
            let _ = ldap.unbind().await;
            return Ok(ConnectorTestResult {
                ok: false,
                message: format!("Bind failed: {}", e),
            });
        }

        // Verify base DN is searchable
        let result = ldap
            .search(
                &config.base_dn,
                Scope::Base,
                "(objectClass=*)",
                vec!["dn"],
            )
            .await;

        let _ = ldap.unbind().await;

        match result {
            Ok(r) => match r.success() {
                Ok(_) => Ok(ConnectorTestResult {
                    ok: true,
                    message: "LDAP connection successful. Service account bind and base DN verified."
                        .to_string(),
                }),
                Err(e) => Ok(ConnectorTestResult {
                    ok: false,
                    message: format!("Base DN search failed: {}", e),
                }),
            },
            Err(e) => Ok(ConnectorTestResult {
                ok: false,
                message: format!("Base DN search error: {}", e),
            }),
        }
    }

    async fn search_users(
        &self,
        config: &LdapConfig,
        query: &str,
        limit: u32,
    ) -> Result<Vec<LdapUserProfile>> {
        let mut ldap = Self::connect(config).await?;
        Self::service_bind(&mut ldap, config).await?;

        let escaped_query = escape_ldap_search_filter(query);
        let filter = format!(
            "(|({}={}*)({}={}*))",
            config.attr_username, escaped_query, config.attr_email, escaped_query,
        );

        let attrs = Self::build_search_attrs(config);
        let scope = config.user_search_scope.to_ldap3_scope();

        let (results, _) = ldap
            .search(&config.base_dn, scope, &filter, attrs)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("LDAP search error: {}", e)))?
            .success()
            .map_err(|e| AppError::Internal(anyhow::anyhow!("LDAP search failed: {}", e)))?;

        let _ = ldap.unbind().await;

        let profiles: Vec<LdapUserProfile> = results
            .into_iter()
            .take(limit as usize)
            .map(|entry| Self::extract_profile(config, &ldap3::SearchEntry::construct(entry)))
            .collect();

        Ok(profiles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_scope_to_ldap3() {
        assert!(matches!(
            SearchScope::Sub.to_ldap3_scope(),
            ldap3::Scope::Subtree
        ));
        assert!(matches!(
            SearchScope::One.to_ldap3_scope(),
            ldap3::Scope::OneLevel
        ));
        assert!(matches!(
            SearchScope::Base.to_ldap3_scope(),
            ldap3::Scope::Base
        ));
    }

    #[test]
    fn build_search_filter_escapes_input() {
        let config = LdapConfig {
            server_url: "ldaps://localhost:636".into(),
            use_tls: true,
            tls_skip_verify: false,
            tls_ca_cert: None,
            connection_timeout_secs: 10,
            bind_dn: "cn=admin".into(),
            bind_password: "pass".into(), // pragma: allowlist secret
            base_dn: "dc=test".into(),
            user_search_filter: "(uid={username})".into(),
            user_search_scope: SearchScope::Sub,
            group_search_base: None,
            group_search_filter: None,
            attr_username: "uid".into(),
            attr_email: "mail".into(),
            attr_first_name: "givenName".into(),
            attr_last_name: "sn".into(),
            attr_display_name: None,
            attr_phone: None,
            attr_groups: None,
            is_active_directory: false,
            ad_domain: None,
        };

        assert_eq!(
            DefaultLdapAuthenticator::build_search_filter(&config, "john.doe"),
            "(uid=john.doe)"
        );

        // Injection attempt should be escaped
        assert_eq!(
            DefaultLdapAuthenticator::build_search_filter(&config, ")(cn=*)"),
            "(uid=\\29\\28cn=\\2a\\29)"
        );
    }
}
