//! Pool trait and common pool types

use crate::error::Result;
use solana_sdk::pubkey::Pubkey;

/// Represents a liquidity pool on any DEX
pub trait Pool: Send + Sync {
    /// Get the pool's address
    fn address(&self) -> &Pubkey;

    /// Get the DEX name (e.g., "Raydium", "Orca")
    fn dex_name(&self) -> &str;

    /// Get token A mint address
    fn token_a(&self) -> &Pubkey;

    /// Get token B mint address
    fn token_b(&self) -> &Pubkey;

    /// Get reserve amount for token A
    fn reserve_a(&self) -> u64;

    /// Get reserve amount for token B
    fn reserve_b(&self) -> u64;

    /// Get trading fee in basis points (e.g., 25 = 0.25%)
    fn fee_bps(&self) -> u16;

    /// Calculate output amount for a given input
    /// Returns (output_amount, price_impact_bps)
    fn calculate_output(&self, input_amount: u64, a_to_b: bool) -> Result<(u64, u16)>;

    /// Calculate price impact in basis points
    fn calculate_price_impact(&self, input_amount: u64, a_to_b: bool) -> Result<u16>;

    /// Check if pool has sufficient liquidity for the swap
    fn has_sufficient_liquidity(&self, input_amount: u64, a_to_b: bool) -> bool;
}

/// Common pool information shared across DEXes
#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub address: Pubkey,
    pub dex: String,
    pub token_a: Pubkey,
    pub token_b: Pubkey,
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub fee_bps: u16,
}

impl PoolInfo {
    pub fn new(
        address: Pubkey,
        dex: String,
        token_a: Pubkey,
        token_b: Pubkey,
        reserve_a: u64,
        reserve_b: u64,
        fee_bps: u16,
    ) -> Self {
        Self {
            address,
            dex,
            token_a,
            token_b,
            reserve_a,
            reserve_b,
            fee_bps,
        }
    }

    /// Get reserves for a given direction
    pub fn get_reserves(&self, a_to_b: bool) -> (u64, u64) {
        if a_to_b {
            (self.reserve_a, self.reserve_b)
        } else {
            (self.reserve_b, self.reserve_a)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_info_creation() {
        let addr = Pubkey::new_unique();
        let token_a = Pubkey::new_unique();
        let token_b = Pubkey::new_unique();

        let pool = PoolInfo::new(
            addr,
            "TestDex".to_string(),
            token_a,
            token_b,
            1_000_000,
            50_000_000,
            25,
        );

        assert_eq!(pool.address, addr);
        assert_eq!(pool.dex, "TestDex");
        assert_eq!(pool.reserve_a, 1_000_000);
        assert_eq!(pool.reserve_b, 50_000_000);
        assert_eq!(pool.fee_bps, 25);
    }

    #[test]
    fn test_get_reserves() {
        let pool = PoolInfo::new(
            Pubkey::new_unique(),
            "TestDex".to_string(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            1_000_000,
            50_000_000,
            25,
        );

        let (reserve_in, reserve_out) = pool.get_reserves(true);
        assert_eq!(reserve_in, 1_000_000);
        assert_eq!(reserve_out, 50_000_000);

        let (reserve_in, reserve_out) = pool.get_reserves(false);
        assert_eq!(reserve_in, 50_000_000);
        assert_eq!(reserve_out, 1_000_000);
    }
}
