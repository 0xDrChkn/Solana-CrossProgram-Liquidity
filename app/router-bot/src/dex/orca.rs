//! Orca pool implementation
//!
//! Orca supports both constant product and concentrated liquidity pools

use crate::calculator::{calculate_amount_out, calculate_price_impact};
use crate::error::{Result, RouterError};
use crate::types::pool::{Pool, PoolInfo};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Orca Whirlpool program ID (concentrated liquidity)
pub const ORCA_WHIRLPOOL_PROGRAM: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";

/// Orca pool implementation
#[derive(Debug, Clone)]
pub struct OrcaPool {
    info: PoolInfo,
    pool_type: OrcaPoolType,
}

#[derive(Debug, Clone)]
pub enum OrcaPoolType {
    /// Constant product AMM (similar to Uniswap V2)
    ConstantProduct,
    /// Concentrated liquidity (Whirlpool)
    ConcentratedLiquidity,
}

impl OrcaPool {
    /// Create a new Orca pool
    pub fn new(
        address: Pubkey,
        token_a: Pubkey,
        token_b: Pubkey,
        reserve_a: u64,
        reserve_b: u64,
        pool_type: OrcaPoolType,
        fee_bps: u16,
    ) -> Self {
        Self {
            info: PoolInfo::new(
                address,
                "Orca".to_string(),
                token_a,
                token_b,
                reserve_a,
                reserve_b,
                fee_bps,
            ),
            pool_type,
        }
    }

    /// Create a new Orca constant product pool with default 0.3% fee
    pub fn new_constant_product(
        address: Pubkey,
        token_a: Pubkey,
        token_b: Pubkey,
        reserve_a: u64,
        reserve_b: u64,
    ) -> Self {
        Self::new(
            address,
            token_a,
            token_b,
            reserve_a,
            reserve_b,
            OrcaPoolType::ConstantProduct,
            30, // 0.3% fee
        )
    }

    /// Create a new Orca Whirlpool (concentrated liquidity)
    pub fn new_whirlpool(
        address: Pubkey,
        token_a: Pubkey,
        token_b: Pubkey,
        reserve_a: u64,
        reserve_b: u64,
        fee_bps: u16,
    ) -> Self {
        Self::new(
            address,
            token_a,
            token_b,
            reserve_a,
            reserve_b,
            OrcaPoolType::ConcentratedLiquidity,
            fee_bps,
        )
    }

    /// Parse Orca pool account data
    pub fn from_account_data(_address: Pubkey, _data: &[u8]) -> Result<Self> {
        // TODO: Implement actual Orca account parsing
        Err(RouterError::PoolParseError(
            "Orca pool parsing not yet implemented - use new() for testing".to_string(),
        ))
    }

    /// Get the Orca Whirlpool program ID
    pub fn whirlpool_program_id() -> Pubkey {
        Pubkey::from_str(ORCA_WHIRLPOOL_PROGRAM).unwrap()
    }

    pub fn pool_type(&self) -> &OrcaPoolType {
        &self.pool_type
    }
}

impl Pool for OrcaPool {
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

        // For concentrated liquidity, we'd use a different formula
        // For now, we'll use constant product for both types
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
        let (_, price_impact) = self.calculate_output(input_amount, a_to_b)?;
        Ok(price_impact)
    }

    fn has_sufficient_liquidity(&self, input_amount: u64, a_to_b: bool) -> bool {
        let (_, reserve_out) = self.info.get_reserves(a_to_b);
        match self.calculate_output(input_amount, a_to_b) {
            Ok((output, _)) => output < reserve_out / 2,
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orca_constant_product_pool() {
        let pool = OrcaPool::new_constant_product(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            1_000_000_000,
            50_000_000_000,
        );

        assert_eq!(pool.dex_name(), "Orca");
        assert_eq!(pool.fee_bps(), 30); // 0.3% fee
        assert!(matches!(pool.pool_type(), OrcaPoolType::ConstantProduct));
    }

    #[test]
    fn test_orca_whirlpool() {
        let pool = OrcaPool::new_whirlpool(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            1_000_000_000,
            50_000_000_000,
            10, // 0.1% fee
        );

        assert_eq!(pool.dex_name(), "Orca");
        assert_eq!(pool.fee_bps(), 10);
        assert!(matches!(pool.pool_type(), OrcaPoolType::ConcentratedLiquidity));
    }

    #[test]
    fn test_orca_calculate_output() {
        let pool = OrcaPool::new_constant_product(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            1_000_000_000,
            50_000_000_000,
        );

        let input = 1_000_000;
        let (output, price_impact) = pool.calculate_output(input, true).unwrap();

        assert!(output > 0);
        assert!(price_impact < 100);
    }

    #[test]
    fn test_orca_different_fees() {
        let pool_high_fee = OrcaPool::new_constant_product(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            1_000_000_000,
            50_000_000_000,
        );

        let pool_low_fee = OrcaPool::new_whirlpool(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            1_000_000_000,
            50_000_000_000,
            10,
        );

        let input = 1_000_000;
        let (output_high_fee, _) = pool_high_fee.calculate_output(input, true).unwrap();
        let (output_low_fee, _) = pool_low_fee.calculate_output(input, true).unwrap();

        // Lower fee pool should give better output
        assert!(output_low_fee > output_high_fee);
    }

    #[test]
    fn test_whirlpool_program_id() {
        let program_id = OrcaPool::whirlpool_program_id();
        assert_eq!(program_id.to_string(), ORCA_WHIRLPOOL_PROGRAM);
    }
}
