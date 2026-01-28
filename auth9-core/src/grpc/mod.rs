//! gRPC services

pub mod token_exchange;

pub use token_exchange::TokenExchangeService;

// Include generated protobuf code
pub mod proto {
    tonic::include_proto!("auth9");
}
