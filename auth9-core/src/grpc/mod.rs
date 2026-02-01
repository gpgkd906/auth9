//! gRPC services

pub mod interceptor;
pub mod token_exchange;

pub use interceptor::{ApiKeyAuthenticator, AuthContext, AuthInterceptor, GrpcAuthenticator};
pub use token_exchange::TokenExchangeService;

// Include generated protobuf code
pub mod proto {
    tonic::include_proto!("auth9");
}

#[cfg(test)]
mod tests {
    use super::proto::*;

    #[test]
    fn test_proto_module_exports() {
        // Verify proto types are accessible
        let _ = ExchangeTokenRequest {
            identity_token: String::new(),
            tenant_id: String::new(),
            service_id: String::new(),
        };

        let _ = ValidateTokenRequest {
            access_token: String::new(),
            audience: String::new(),
        };
    }
}
