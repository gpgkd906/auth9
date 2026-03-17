use anyhow::{anyhow, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct StubIdentityProvider {
    pub alias: String,
}

#[derive(Debug, Default, Clone)]
pub struct Auth9OidcIdentityEngine;

impl Auth9OidcIdentityEngine {
    pub fn new() -> Self {
        Self
    }

    pub async fn health_probe(&self) -> Result<()> {
        Ok(())
    }

    pub async fn list_identity_providers(&self) -> Result<Vec<StubIdentityProvider>> {
        Ok(vec![])
    }

    pub async fn create_identity_provider(&self, _alias: &str) -> Result<()> {
        Err(anyhow!("not implemented"))
    }
}

#[cfg(test)]
mod tests {
    use super::Auth9OidcIdentityEngine;

    #[tokio::test]
    async fn stub_engine_returns_empty_provider_list() {
        let engine = Auth9OidcIdentityEngine::new();
        let providers = engine.list_identity_providers().await.unwrap();
        assert!(providers.is_empty());
    }
}
