//! gRPC authentication trait and interceptor
//!
//! Provides the core authentication abstraction for gRPC services.

use tonic::{Request, Status};

/// Authentication context passed to gRPC handlers after successful authentication
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// Client identifier (API key name, certificate CN, etc.)
    pub client_id: String,
    /// Authentication method used
    pub auth_method: AuthMethod,
    /// Additional metadata from authentication
    pub metadata: std::collections::HashMap<String, String>,
}

/// Authentication method used
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMethod {
    /// API Key authentication
    ApiKey,
    /// mTLS certificate authentication
    Mtls,
    /// No authentication (when auth is disabled)
    None,
}

impl AuthContext {
    /// Create a new auth context for API key authentication
    pub fn api_key(client_id: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            auth_method: AuthMethod::ApiKey,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create a new auth context for mTLS authentication
    pub fn mtls(client_id: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            auth_method: AuthMethod::Mtls,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create a new auth context for unauthenticated requests
    pub fn none() -> Self {
        Self {
            client_id: "anonymous".to_string(),
            auth_method: AuthMethod::None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Add metadata to the auth context
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Trait for gRPC authenticators
///
/// Implementations of this trait handle different authentication mechanisms
/// (API keys, mTLS, etc.) for gRPC services.
///
/// Note: This trait uses `Request<()>` to be object-safe. The actual request
/// body type doesn't matter for authentication since we only examine metadata.
pub trait GrpcAuthenticator: Send + Sync {
    /// Authenticate a gRPC request using its metadata
    ///
    /// Returns an `AuthContext` on success, or a `Status` error on failure.
    #[allow(clippy::result_large_err)]
    fn authenticate(&self, metadata: &tonic::metadata::MetadataMap) -> Result<AuthContext, Status>;

    /// Get the name of this authenticator
    fn name(&self) -> &'static str;
}

/// No-op authenticator that allows all requests
///
/// Used when authentication is disabled.
pub struct NoOpAuthenticator;

impl GrpcAuthenticator for NoOpAuthenticator {
    fn authenticate(
        &self,
        _metadata: &tonic::metadata::MetadataMap,
    ) -> Result<AuthContext, Status> {
        Ok(AuthContext::none())
    }

    fn name(&self) -> &'static str {
        "none"
    }
}

/// Enum-based authenticator that supports different authentication modes
///
/// This is used instead of `dyn GrpcAuthenticator` for better performance
/// and to avoid object-safety issues.
#[derive(Clone)]
pub enum AuthenticatorMode {
    /// No authentication required
    None,
    /// API Key authentication
    ApiKey(super::api_key::ApiKeyAuthenticator),
}

impl AuthenticatorMode {
    /// Authenticate a request
    #[allow(clippy::result_large_err)]
    pub fn authenticate(
        &self,
        metadata: &tonic::metadata::MetadataMap,
    ) -> Result<AuthContext, Status> {
        match self {
            AuthenticatorMode::None => Ok(AuthContext::none()),
            AuthenticatorMode::ApiKey(auth) => auth.authenticate(metadata),
        }
    }
}

/// Authentication interceptor for gRPC services
///
/// Applies authentication to incoming requests.
#[derive(Clone)]
pub struct AuthInterceptor {
    mode: AuthenticatorMode,
}

impl AuthInterceptor {
    /// Create a new authentication interceptor with the given mode
    pub fn new(mode: AuthenticatorMode) -> Self {
        Self { mode }
    }

    /// Create a no-op interceptor that allows all requests
    pub fn noop() -> Self {
        Self {
            mode: AuthenticatorMode::None,
        }
    }

    /// Create an API key interceptor
    pub fn api_key(authenticator: super::api_key::ApiKeyAuthenticator) -> Self {
        Self {
            mode: AuthenticatorMode::ApiKey(authenticator),
        }
    }
}

impl tonic::service::Interceptor for AuthInterceptor {
    fn call(&mut self, request: Request<()>) -> Result<Request<()>, Status> {
        // Authenticate the request
        let auth_context = self.mode.authenticate(request.metadata())?;

        // Add auth context to request extensions
        let mut request = request;
        request.extensions_mut().insert(auth_context);

        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_context_api_key() {
        let ctx = AuthContext::api_key("test-client");
        assert_eq!(ctx.client_id, "test-client");
        assert_eq!(ctx.auth_method, AuthMethod::ApiKey);
        assert!(ctx.metadata.is_empty());
    }

    #[test]
    fn test_auth_context_mtls() {
        let ctx = AuthContext::mtls("CN=my-service");
        assert_eq!(ctx.client_id, "CN=my-service");
        assert_eq!(ctx.auth_method, AuthMethod::Mtls);
    }

    #[test]
    fn test_auth_context_none() {
        let ctx = AuthContext::none();
        assert_eq!(ctx.client_id, "anonymous");
        assert_eq!(ctx.auth_method, AuthMethod::None);
    }

    #[test]
    fn test_auth_context_with_metadata() {
        let ctx = AuthContext::api_key("client")
            .with_metadata("tenant_id", "tenant-123")
            .with_metadata("service", "my-service");

        assert_eq!(
            ctx.metadata.get("tenant_id"),
            Some(&"tenant-123".to_string())
        );
        assert_eq!(ctx.metadata.get("service"), Some(&"my-service".to_string()));
    }

    #[test]
    fn test_noop_authenticator() {
        let authenticator = NoOpAuthenticator;
        let metadata = tonic::metadata::MetadataMap::new();

        let result = authenticator.authenticate(&metadata);
        assert!(result.is_ok());

        let ctx = result.unwrap();
        assert_eq!(ctx.auth_method, AuthMethod::None);
    }

    #[test]
    fn test_noop_authenticator_name() {
        let authenticator = NoOpAuthenticator;
        assert_eq!(authenticator.name(), "none");
    }

    #[test]
    fn test_auth_interceptor_noop() {
        let interceptor = AuthInterceptor::noop();
        let metadata = tonic::metadata::MetadataMap::new();

        let result = interceptor.mode.authenticate(&metadata);
        assert!(result.is_ok());
    }

    #[test]
    fn test_auth_method_equality() {
        assert_eq!(AuthMethod::ApiKey, AuthMethod::ApiKey);
        assert_eq!(AuthMethod::Mtls, AuthMethod::Mtls);
        assert_eq!(AuthMethod::None, AuthMethod::None);
        assert_ne!(AuthMethod::ApiKey, AuthMethod::Mtls);
    }

    #[test]
    fn test_auth_context_clone() {
        let ctx = AuthContext::api_key("client").with_metadata("key", "value");
        let cloned = ctx.clone();

        assert_eq!(ctx.client_id, cloned.client_id);
        assert_eq!(ctx.auth_method, cloned.auth_method);
        assert_eq!(ctx.metadata, cloned.metadata);
    }

    #[test]
    fn test_auth_context_debug() {
        let ctx = AuthContext::api_key("test");
        let debug_str = format!("{:?}", ctx);
        assert!(debug_str.contains("AuthContext"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_authenticator_mode_none() {
        let mode = AuthenticatorMode::None;
        let metadata = tonic::metadata::MetadataMap::new();

        let result = mode.authenticate(&metadata);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().auth_method, AuthMethod::None);
    }

    #[test]
    fn test_auth_interceptor_api_key() {
        let authenticator =
            super::super::api_key::ApiKeyAuthenticator::new(vec!["test-key".to_string()]);
        let interceptor = AuthInterceptor::api_key(authenticator);

        // Should fail without API key
        let metadata = tonic::metadata::MetadataMap::new();
        let result = interceptor.mode.authenticate(&metadata);
        assert!(result.is_err());

        // Should succeed with valid API key
        let mut metadata = tonic::metadata::MetadataMap::new();
        metadata.insert("x-api-key", "test-key".parse().unwrap());
        let result = interceptor.mode.authenticate(&metadata);
        assert!(result.is_ok());
    }
}
