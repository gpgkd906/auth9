use crate::config::Config;
use crate::health;
use crate::identity_engine::Auth9OidcIdentityEngine;
use anyhow::Result;
use axum::{routing::get, Router};
use sqlx::MySqlPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db_pool: MySqlPool,
    pub identity_engine: Auth9OidcIdentityEngine,
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new().route("/health", get(health::health)).with_state(state)
}

pub async fn run(config: Config, db_pool: MySqlPool) -> Result<()> {
    let state = Arc::new(AppState {
        config: config.clone(),
        db_pool,
        identity_engine: Auth9OidcIdentityEngine::new(),
    });
    let listener = tokio::net::TcpListener::bind(config.http_addr()).await?;
    axum::serve(listener, router(state)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn router_exposes_health_route() {
        let pool = sqlx::mysql::MySqlPoolOptions::new()
            .connect_lazy("mysql://root@localhost/auth9")
            .unwrap();
        let state = Arc::new(AppState {
            config: Config {
                http_host: "127.0.0.1".to_string(),
                http_port: 8090,
                database_url: "mysql://root@localhost/auth9".to_string(),
                identity_backend: "auth9_oidc".to_string(),
            },
            db_pool: pool,
            identity_engine: Auth9OidcIdentityEngine::new(),
        });

        let app = router(state);
        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
