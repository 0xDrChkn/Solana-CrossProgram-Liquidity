//! Route and swap quote types

use solana_sdk::pubkey::Pubkey;

/// Represents a single step in a swap route
#[derive(Debug, Clone)]
pub struct RouteStep {
    /// The pool address to use for this step
    pub pool_address: Pubkey,
    /// DEX name
    pub dex: String,
    /// Input token for this step
    pub token_in: Pubkey,
    /// Output token for this step
    pub token_out: Pubkey,
    /// Amount to swap in this step
    pub amount_in: u64,
    /// Expected output amount
    pub amount_out: u64,
    /// Price impact in basis points
    pub price_impact_bps: u16,
    /// Fee in basis points
    pub fee_bps: u16,
}

/// Represents a complete swap route (can be multi-hop)
#[derive(Debug, Clone)]
pub struct Route {
    /// All steps in the route
    pub steps: Vec<RouteStep>,
    /// Total input amount
    pub total_input: u64,
    /// Total output amount
    pub total_output: u64,
    /// Overall price impact
    pub total_price_impact_bps: u16,
}

impl Route {
    /// Create a simple single-step route
    pub fn single_step(step: RouteStep, input: u64, output: u64) -> Self {
        let price_impact = step.price_impact_bps;
        Self {
            steps: vec![step],
            total_input: input,
            total_output: output,
            total_price_impact_bps: price_impact,
        }
    }

    /// Create a multi-step route
    pub fn multi_step(steps: Vec<RouteStep>) -> Self {
        let total_input = steps.first().map(|s| s.amount_in).unwrap_or(0);
        let total_output = steps.last().map(|s| s.amount_out).unwrap_or(0);

        // Calculate total price impact (approximate)
        let total_price_impact_bps = steps
            .iter()
            .map(|s| s.price_impact_bps as u32)
            .sum::<u32>()
            .min(10000) as u16;

        Self {
            steps,
            total_input,
            total_output,
            total_price_impact_bps,
        }
    }

    /// Get the number of hops in the route
    pub fn hop_count(&self) -> usize {
        self.steps.len()
    }

    /// Check if this is a direct swap (single hop)
    pub fn is_direct(&self) -> bool {
        self.steps.len() == 1
    }

    /// Calculate the effective price (output/input ratio)
    pub fn effective_price(&self) -> f64 {
        if self.total_input == 0 {
            0.0
        } else {
            self.total_output as f64 / self.total_input as f64
        }
    }
}

/// Represents a swap quote with routing information
#[derive(Debug, Clone)]
pub struct SwapQuote {
    /// Input token mint
    pub token_in: Pubkey,
    /// Output token mint
    pub token_out: Pubkey,
    /// Input amount
    pub amount_in: u64,
    /// Expected output amount
    pub amount_out: u64,
    /// Price impact in basis points
    pub price_impact_bps: u16,
    /// The route to execute
    pub route: Route,
    /// Strategy used (e.g., "single_pool", "split", "multi_hop")
    pub strategy: String,
}

impl SwapQuote {
    pub fn new(
        token_in: Pubkey,
        token_out: Pubkey,
        amount_in: u64,
        amount_out: u64,
        route: Route,
        strategy: String,
    ) -> Self {
        Self {
            token_in,
            token_out,
            amount_in,
            amount_out,
            price_impact_bps: route.total_price_impact_bps,
            route,
            strategy,
        }
    }

    /// Compare quotes and return the better one (higher output)
    pub fn better_than(&self, other: &SwapQuote) -> bool {
        self.amount_out > other.amount_out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_step(amount_in: u64, amount_out: u64) -> RouteStep {
        RouteStep {
            pool_address: Pubkey::new_unique(),
            dex: "TestDex".to_string(),
            token_in: Pubkey::new_unique(),
            token_out: Pubkey::new_unique(),
            amount_in,
            amount_out,
            price_impact_bps: 50,
            fee_bps: 25,
        }
    }

    #[test]
    fn test_single_step_route() {
        let step = create_test_step(1_000_000, 50_000_000);
        let route = Route::single_step(step, 1_000_000, 50_000_000);

        assert_eq!(route.hop_count(), 1);
        assert!(route.is_direct());
        assert_eq!(route.total_input, 1_000_000);
        assert_eq!(route.total_output, 50_000_000);
    }

    #[test]
    fn test_multi_step_route() {
        let step1 = create_test_step(1_000_000, 50_000_000);
        let step2 = create_test_step(50_000_000, 100_000);

        let route = Route::multi_step(vec![step1, step2]);

        assert_eq!(route.hop_count(), 2);
        assert!(!route.is_direct());
        assert_eq!(route.total_input, 1_000_000);
        assert_eq!(route.total_output, 100_000);
    }

    #[test]
    fn test_effective_price() {
        let step = create_test_step(1_000_000, 50_000_000);
        let route = Route::single_step(step, 1_000_000, 50_000_000);

        assert_eq!(route.effective_price(), 50.0);
    }

    #[test]
    fn test_swap_quote_comparison() {
        let token_in = Pubkey::new_unique();
        let token_out = Pubkey::new_unique();

        let step1 = create_test_step(1_000_000, 50_000_000);
        let route1 = Route::single_step(step1, 1_000_000, 50_000_000);
        let quote1 = SwapQuote::new(
            token_in,
            token_out,
            1_000_000,
            50_000_000,
            route1,
            "single_pool".to_string(),
        );

        let step2 = create_test_step(1_000_000, 51_000_000);
        let route2 = Route::single_step(step2, 1_000_000, 51_000_000);
        let quote2 = SwapQuote::new(
            token_in,
            token_out,
            1_000_000,
            51_000_000,
            route2,
            "single_pool".to_string(),
        );

        assert!(quote2.better_than(&quote1));
        assert!(!quote1.better_than(&quote2));
    }
}
