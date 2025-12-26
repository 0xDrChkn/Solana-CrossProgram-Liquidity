//! Phoenix pool implementation
//!
//! Phoenix is an orderbook-based DEX (not AMM), but we can approximate
//! pricing based on best bid/ask

use crate::error::{Result, RouterError};
use crate::types::pool::{Pool, PoolInfo};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Phoenix program ID
pub const PHOENIX_PROGRAM: &str = "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY";

/// Phoenix market implementation
/// Note: Phoenix uses an orderbook model, not AMM, so this is a simplified adapter
#[derive(Debug, Clone)]
pub struct PhoenixPool {
    info: PoolInfo,
    /// Best bid price (for selling token A)
    best_bid: u64,
    /// Best ask price (for buying token A)
    best_ask: u64,
}

impl PhoenixPool {
    /// Create a new Phoenix market adapter
    ///
    /// For orderbook markets, reserves represent available liquidity at best prices
    pub fn new(
        address: Pubkey,
        token_a: Pubkey,
        token_b: Pubkey,
        liquidity_a: u64,
        liquidity_b: u64,
        best_bid: u64,
        best_ask: u64,
    ) -> Self {
        Self {
            info: PoolInfo::new(
                address,
                "Phoenix".to_string(),
                token_a,
                token_b,
                liquidity_a,
                liquidity_b,
                0, // No fixed fee, spread is the "fee"
            ),
            best_bid,
            best_ask,
        }
    }

    /// Parse Phoenix market account data
    pub fn from_account_data(_address: Pubkey, _data: &[u8]) -> Result<Self> {
        // TODO: Implement actual Phoenix market parsing
        Err(RouterError::PoolParseError(
            "Phoenix market parsing not yet implemented - use new() for testing".to_string(),
        ))
    }

    /// Get the Phoenix program ID
    pub fn program_id() -> Pubkey {
        Pubkey::from_str(PHOENIX_PROGRAM).unwrap()
    }

    pub fn best_bid(&self) -> u64 {
        self.best_bid
    }

    pub fn best_ask(&self) -> u64 {
        self.best_ask
    }

    /// Calculate spread in basis points
    pub fn spread_bps(&self) -> u16 {
        if self.best_bid == 0 {
            return 10000; // 100% spread if no bid
        }
        let spread = self.best_ask.saturating_sub(self.best_bid);
        ((spread as u128 * 10000) / self.best_bid as u128)
            .min(10000) as u16
    }
}

impl Pool for PhoenixPool {
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
        // For orderbooks, the "fee" is the spread
        self.spread_bps()
    }

    fn calculate_output(&self, input_amount: u64, a_to_b: bool) -> Result<(u64, u16)> {
        // For orderbook: if selling A for B, use best_bid; if buying A with B, use best_ask
        let (available_liquidity, price) = if a_to_b {
            (self.info.reserve_b, self.best_bid)
        } else {
            (self.info.reserve_a, self.best_ask)
        };

        if price == 0 {
            return Err(RouterError::InsufficientLiquidity);
        }

        // Simple calculation: output = input * price
        // (In reality, you'd walk the orderbook)
        let output_amount = ((input_amount as u128 * price as u128) / 1_000_000)
            .try_into()
            .map_err(|_| RouterError::MathOverflow)?;

        // Check if we have enough liquidity
        if output_amount > available_liquidity {
            return Err(RouterError::InsufficientLiquidity);
        }

        // Price impact for orderbooks is approximated by spread
        let price_impact = self.spread_bps();

        Ok((output_amount, price_impact))
    }

    fn calculate_price_impact(&self, _input_amount: u64, _a_to_b: bool) -> Result<u16> {
        // For orderbooks, price impact is approximated by the spread
        Ok(self.spread_bps())
    }

    fn has_sufficient_liquidity(&self, input_amount: u64, a_to_b: bool) -> bool {
        match self.calculate_output(input_amount, a_to_b) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phoenix_market_creation() {
        let market = PhoenixPool::new(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            1_000_000_000, // liquidity A
            50_000_000_000, // liquidity B
            49_500, // best bid (49.5 per unit)
            50_500, // best ask (50.5 per unit)
        );

        assert_eq!(market.dex_name(), "Phoenix");
        assert_eq!(market.best_bid(), 49_500);
        assert_eq!(market.best_ask(), 50_500);
    }

    #[test]
    fn test_phoenix_spread_calculation() {
        let market = PhoenixPool::new(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            1_000_000_000,
            50_000_000_000,
            49_000, // bid
            51_000, // ask
        );

        let spread = market.spread_bps();
        // Spread = (51000 - 49000) / 49000 * 10000 ≈ 408 bps (4.08%)
        assert!(spread > 400 && spread < 420);
    }

    #[test]
    fn test_phoenix_calculate_output() {
        let market = PhoenixPool::new(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            1_000_000_000,
            50_000_000_000,
            50_000_000, // bid (price in microunits)
            50_000_000, // ask (same for simplicity)
        );

        let input = 1_000_000; // 1 unit of A
        let (output, _) = market.calculate_output(input, true).unwrap();

        // Should get approximately 50 units of B
        assert!(output > 0);
    }

    #[test]
    fn test_phoenix_insufficient_liquidity() {
        let market = PhoenixPool::new(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            100, // very low liquidity
            100,
            50_000_000,
            50_000_000,
        );

        let input = 1_000_000_000; // huge amount
        let result = market.calculate_output(input, true);

        assert!(result.is_err());
    }

    #[test]
    fn test_phoenix_program_id() {
        let program_id = PhoenixPool::program_id();
        assert_eq!(program_id.to_string(), PHOENIX_PROGRAM);
    }
}
