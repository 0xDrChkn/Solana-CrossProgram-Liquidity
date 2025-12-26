//! Raydium AMM pool implementation
//!
//! Raydium uses a constant product AMM similar to Uniswap V2

use crate::calculator::{calculate_amount_out, calculate_price_impact};
use crate::error::{Result, RouterError};
use crate::types::pool::{Pool, PoolInfo};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Raydium AMM program ID
pub const RAYDIUM_AMM_PROGRAM: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

/// Raydium pool implementation
#[derive(Debug, Clone)]
pub struct RaydiumPool {
    info: PoolInfo,
}

impl RaydiumPool {
    /// Create a new Raydium pool
    pub fn new(
        address: Pubkey,
        token_a: Pubkey,
        token_b: Pubkey,
        reserve_a: u64,
        reserve_b: u64,
    ) -> Self {
        Self {
            info: PoolInfo::new(
                address,
                "Raydium".to_string(),
                token_a,
                token_b,
                reserve_a,
                reserve_b,
                25, // Raydium uses 0.25% fee
            ),
        }
    }

    /// Parse Raydium pool account data
    ///
    /// Note: This is a simplified version. In production, you'd need to parse
    /// the actual Raydium account layout which includes:
    /// - Pool state
    /// - Coin vault address
    /// - PC vault address
    /// - LP mint, etc.
    pub fn from_account_data(_address: Pubkey, _data: &[u8]) -> Result<Self> {
        // TODO: Implement actual Raydium account parsing
        // For now, return error indicating not implemented
        Err(RouterError::PoolParseError(
            "Raydium pool parsing not yet implemented - use new() for testing".to_string(),
        ))
    }

    /// Get the Raydium program ID
    pub fn program_id() -> Pubkey {
        Pubkey::from_str(RAYDIUM_AMM_PROGRAM).unwrap()
    }
}

impl Pool for RaydiumPool {
    fn address(&self) -> &Pubkey {
        &self.info.address
    }

    fn dex_name(&self) -> &str {
        &self.info.dex
    }

    fn token_a(&self) -> &Pubkey {
        &self.info.token_a
    }

    fn token_b(&self) -> &Pubkey {
        &self.info.token_b
    }

    fn reserve_a(&self) -> u64 {
        self.info.reserve_a
    }

    fn reserve_b(&self) -> u64 {
        self.info.reserve_b
    }

    fn fee_bps(&self) -> u16 {
        self.info.fee_bps
    }

    fn calculate_output(&self, input_amount: u64, a_to_b: bool) -> Result<(u64, u16)> {
        let (reserve_in, reserve_out) = self.info.get_reserves(a_to_b);

        let output_amount = calculate_amount_out(
            input_amount,
            reserve_in,
            reserve_out,
            self.fee_bps(),
        )?;

        let price_impact = calculate_price_impact(
            input_amount,
            output_amount,
            reserve_in,
            reserve_out,
        )?;

        Ok((output_amount, price_impact))
    }

    fn calculate_price_impact(&self, input_amount: u64, a_to_b: bool) -> Result<u16> {
        let (_output_amount, price_impact) = self.calculate_output(input_amount, a_to_b)?;
        Ok(price_impact)
    }

    fn has_sufficient_liquidity(&self, input_amount: u64, a_to_b: bool) -> bool {
        let (_, reserve_out) = self.info.get_reserves(a_to_b);
        // Simple check: ensure we're not trying to drain more than 50% of reserves
        match self.calculate_output(input_amount, a_to_b) {
            Ok((output, _)) => output < reserve_out / 2,
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pool() -> RaydiumPool {
        RaydiumPool::new(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            1_000_000_000, // 1000 SOL (9 decimals)
            50_000_000_000, // 50000 USDC (6 decimals)
        )
    }

    #[test]
    fn test_raydium_pool_creation() {
        let pool = create_test_pool();
        assert_eq!(pool.dex_name(), "Raydium");
        assert_eq!(pool.fee_bps(), 25);
        assert_eq!(pool.reserve_a(), 1_000_000_000);
        assert_eq!(pool.reserve_b(), 50_000_000_000);
    }

    #[test]
    fn test_raydium_calculate_output() {
        let pool = create_test_pool();

        // Swap 1 SOL for USDC
        let input = 1_000_000; // 0.001 SOL
        let (output, price_impact) = pool.calculate_output(input, true).unwrap();

        // Should get approximately 50 USDC (minus fee)
        assert!(output > 0);
        assert!(output < 50_000_000); // Less than input at 50:1 ratio due to fee
        assert!(price_impact < 100); // Should be < 1% impact for small trade
    }

    #[test]
    fn test_raydium_reverse_swap() {
        let pool = create_test_pool();

        // Swap USDC for SOL
        let input = 50_000_000; // 50 USDC
        let (output, _) = pool.calculate_output(input, false).unwrap();

        // Should get approximately 1 SOL
        assert!(output > 0);
        assert!(output < 1_000_000); // Less than 1:50 ratio due to fee
    }

    #[test]
    fn test_raydium_large_trade_impact() {
        let pool = create_test_pool();

        // Swap 100 SOL (10% of reserves)
        let input = 100_000_000;
        let (output, price_impact) = pool.calculate_output(input, true).unwrap();

        assert!(output > 0);
        // Large trade should have significant impact
        assert!(price_impact > 100); // Should be > 1%
    }

    #[test]
    fn test_raydium_liquidity_check() {
        let pool = create_test_pool();

        // Small trade should have sufficient liquidity
        assert!(pool.has_sufficient_liquidity(1_000_000, true));

        // Huge trade should fail liquidity check
        assert!(!pool.has_sufficient_liquidity(u64::MAX, true));
    }

    #[test]
    fn test_raydium_program_id() {
        let program_id = RaydiumPool::program_id();
        assert_eq!(program_id.to_string(), RAYDIUM_AMM_PROGRAM);
    }
}
