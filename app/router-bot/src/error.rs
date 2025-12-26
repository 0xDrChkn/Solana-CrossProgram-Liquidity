//! Error types for the router bot

use thiserror::Error;

pub type Result<T> = std::result::Result<T, RouterError>;

#[derive(Error, Debug)]
pub enum RouterError {
    #[error("RPC client error: {0}")]
    RpcError(#[from] solana_client::client_error::ClientError),

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Invalid account data: {0}")]
    InvalidAccountData(String),

    #[error("Pool parsing error: {0}")]
    PoolParseError(String),

    #[error("Invalid mint account")]
    InvalidMint,

    #[error("Insufficient liquidity")]
    InsufficientLiquidity,

    #[error("No route found for token pair")]
    NoRouteFound,

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Transaction build error: {0}")]
    TransactionError(String),

    #[error("Math overflow in calculation")]
    MathOverflow,

    #[error("Invalid pool reserves")]
    InvalidReserves,

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = RouterError::AccountNotFound("test".to_string());
        assert_eq!(err.to_string(), "Account not found: test");
    }

    #[test]
    fn test_error_conversion() {
        let anyhow_err = anyhow::anyhow!("test error");
        let router_err: RouterError = anyhow_err.into();
        assert!(matches!(router_err, RouterError::Other(_)));
    }
}
