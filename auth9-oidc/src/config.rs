use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub http_host: String,
    pub http_port: u16,
    pub database_url: String,
    pub identity_backend: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            http_host: env::var("HTTP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            http_port: env::var("HTTP_PORT")
                .unwrap_or_else(|_| "8090".to_string())
                .parse()
                .context("Invalid HTTP_PORT")?,
            database_url: env::var("DATABASE_URL").context("DATABASE_URL is required")?,
            identity_backend: env::var("IDENTITY_BACKEND")
                .unwrap_or_else(|_| "auth9_oidc".to_string()),
        })
    }

    pub fn http_addr(&self) -> String {
        format!("{}:{}", self.http_host, self.http_port)
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn config_uses_defaults() {
        unsafe {
            std::env::set_var("DATABASE_URL", "mysql://root@localhost/auth9");
            std::env::remove_var("HTTP_HOST");
            std::env::remove_var("HTTP_PORT");
            std::env::remove_var("IDENTITY_BACKEND");
        }

        let config = Config::from_env().unwrap();
        assert_eq!(config.http_host, "0.0.0.0");
        assert_eq!(config.http_port, 8090);
        assert_eq!(config.identity_backend, "auth9_oidc");
    }
}
