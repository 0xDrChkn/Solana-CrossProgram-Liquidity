//! Meteora pool implementation
//!
//! Meteora offers dynamic pools with multiple pool types

use crate::calculator::{calculate_amount_out, calculate_price_impact};
use crate::error::{Result, RouterError};
use crate::types::pool::{Pool, PoolInfo};
use solana_sdk::pubkey::Pubkey;

/// Meteora pool implementation
#[derive(Debug, Clone)]
pub struct MeteoraPool {
    info: PoolInfo,
}

impl MeteoraPool {
    /// Create a new Meteora pool with default 0.25% fee
    pub fn new(
        address: Pubkey,
        token_a: Pubkey,
        token_b: Pubkey,
        reserve_a: u64,
        reserve_b: u64,
        fee_bps: u16,
    ) -> Self {
        Self {
            info: PoolInfo::new(
                address,
                "Meteora".to_string(),
                token_a,
                token_b,
                reserve_a,
                reserve_b,
                fee_bps,
            ),
        }
    }

    /// Parse Meteora pool account data
    pub fn from_account_data(_address: Pubkey, _data: &[u8]) -> Result<Self> {
        // TODO: Implement actual Meteora account parsing
        Err(RouterError::PoolParseError(
            "Meteora pool parsing not yet implemented - use new() for testing".to_string(),
        ))
    }
}

impl Pool for MeteoraPool {
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
    fn test_meteora_pool_creation() {
        let pool = MeteoraPool::new(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            1_000_000_000,
            50_000_000_000,
            20, // 0.2% fee
        );

        assert_eq!(pool.dex_name(), "Meteora");
        assert_eq!(pool.fee_bps(), 20);
    }

    #[test]
    fn test_meteora_calculate_output() {
        let pool = MeteoraPool::new(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            1_000_000_000,
            50_000_000_000,
            20,
        );

        let input = 1_000_000;
        let (output, price_impact) = pool.calculate_output(input, true).unwrap();

        assert!(output > 0);
        assert!(price_impact < 100);
    }
}
