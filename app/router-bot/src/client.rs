//! Solana RPC client wrapper

use crate::error::{Result, RouterError};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    account::Account,
    pubkey::Pubkey,
};
use spl_token::{
    solana_program::program_pack::Pack,
    state::Mint,
};
use std::{str::FromStr, sync::Arc};

/// Wrapper around Solana RPC client with convenience methods
#[derive(Clone)]
pub struct SolanaClient {
    client: Arc<RpcClient>,
}

impl SolanaClient {
    /// Create a new Solana client
    pub fn new(rpc_url: String) -> Self {
        Self {
            client: Arc::new(RpcClient::new(rpc_url)),
        }
    }

    /// Create a client for devnet
    pub fn new_devnet() -> Self {
        Self::new("https://api.devnet.solana.com".to_string())
    }

    /// Create a client for mainnet
    pub fn new_mainnet() -> Self {
        Self::new("https://api.mainnet-beta.solana.com".to_string())
    }

    /// Get the underlying RPC client
    pub fn rpc(&self) -> &RpcClient {
        &self.client
    }

    /// Fetch account data
    pub fn fetch_account(&self, address: &Pubkey) -> Result<Account> {
        self.client
            .get_account(address)
            .map_err(|_| RouterError::AccountNotFound(address.to_string()))
    }

    /// Fetch account data from string address
    pub fn fetch_account_str(&self, address: &str) -> Result<Account> {
        let pubkey = Pubkey::from_str(address)
            .map_err(|e| RouterError::InvalidAccountData(e.to_string()))?;
        self.fetch_account(&pubkey)
    }

    /// Fetch and parse a token mint account
    pub fn fetch_mint(&self, mint_address: &Pubkey) -> Result<Mint> {
        let account = self.fetch_account(mint_address)?;

        // Verify it's owned by the token program
        if account.owner != spl_token::id() {
            return Err(RouterError::InvalidMint);
        }

        // Parse the mint data
        Mint::unpack(&account.data)
            .map_err(|e| RouterError::InvalidAccountData(e.to_string()))
    }

    /// Fetch and parse a token mint from string address
    pub fn fetch_mint_str(&self, mint_address: &str) -> Result<Mint> {
        let pubkey = Pubkey::from_str(mint_address)
            .map_err(|e| RouterError::InvalidAccountData(e.to_string()))?;
        self.fetch_mint(&pubkey)
    }

    /// Fetch multiple accounts in parallel
    pub async fn fetch_accounts_parallel(&self, addresses: &[Pubkey]) -> Vec<Result<Account>> {
        // In a real implementation, this would use get_multiple_accounts
        // For now, we'll fetch sequentially but keep the async signature for future optimization
        addresses
            .iter()
            .map(|addr| self.fetch_account(addr))
            .collect()
    }

    /// Get network version (useful for testing connectivity)
    pub fn get_version(&self) -> Result<String> {
        self.client
            .get_version()
            .map_err(|e| RouterError::RpcError(e))
            .map(|v| format!("{}", v.solana_core))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = SolanaClient::new_devnet();
        assert!(client.rpc().url().contains("devnet"));

        let client = SolanaClient::new_mainnet();
        assert!(client.rpc().url().contains("mainnet"));
    }

    #[test]
    fn test_custom_rpc_url() {
        let custom_url = "https://custom.rpc.com";
        let client = SolanaClient::new(custom_url.to_string());
        assert_eq!(client.rpc().url(), custom_url);
    }

    // Integration tests (marked with #[ignore] to skip in regular test runs)
    #[test]
    #[ignore]
    fn test_fetch_devnet_usdc_mint() {
        let client = SolanaClient::new_devnet();

        // USDC mint on devnet
        let usdc_mint = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";

        let mint = client.fetch_mint_str(usdc_mint).unwrap();

        assert_eq!(mint.decimals, 6);
        println!("✅ USDC Mint:");
        println!("   Decimals: {}", mint.decimals);
        println!("   Supply: {}", mint.supply);
        println!(
            "   Mint Authority: {:?}",
            mint.mint_authority.map(|p| p.to_string())
        );
    }

    #[test]
    #[ignore]
    fn test_network_connectivity() {
        let client = SolanaClient::new_devnet();
        let version = client.get_version().unwrap();
        println!("✅ Connected to Solana devnet");
        println!("   Version: {}", version);
        assert!(!version.is_empty());
    }

    #[test]
    fn test_invalid_mint_address() {
        let client = SolanaClient::new_devnet();
        let result = client.fetch_mint_str("invalid_address");
        assert!(result.is_err());
    }
}
