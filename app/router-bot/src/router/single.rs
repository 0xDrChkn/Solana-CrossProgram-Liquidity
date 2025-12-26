//! Single pool router - finds the best single pool for a swap

use crate::error::{Result, RouterError};
use crate::types::pool::Pool;
use crate::types::route::{Route, RouteStep, SwapQuote};
use solana_sdk::pubkey::Pubkey;

/// Router for finding the best single pool
pub struct SinglePoolRouter;

impl SinglePoolRouter {
    /// Find the best pool for a swap
    ///
    /// # Arguments
    /// * `pools` - List of available pools
    /// * `token_in` - Input token mint
    /// * `token_out` - Output token mint
    /// * `amount_in` - Amount to swap
    ///
    /// # Returns
    /// The best swap quote, or error if no suitable pool found
    pub fn find_best_route(
        pools: &[Box<dyn Pool>],
        token_in: &Pubkey,
        token_out: &Pubkey,
        amount_in: u64,
    ) -> Result<SwapQuote> {
        let mut best_quote: Option<SwapQuote> = None;

        for pool in pools {
            // Check if pool matches token pair
            let (matches, a_to_b) = if pool.token_a() == token_in && pool.token_b() == token_out {
                (true, true)
            } else if pool.token_b() == token_in && pool.token_a() == token_out {
                (true, false)
            } else {
                (false, false)
            };

            if !matches {
                continue;
            }

            // Check liquidity
            if !pool.has_sufficient_liquidity(amount_in, a_to_b) {
                continue;
            }

            // Calculate output
            match pool.calculate_output(amount_in, a_to_b) {
                Ok((amount_out, price_impact)) => {
                    let step = RouteStep {
                        pool_address: *pool.address(),
                        dex: pool.dex_name().to_string(),
                        token_in: *token_in,
                        token_out: *token_out,
                        amount_in,
                        amount_out,
                        price_impact_bps: price_impact,
                        fee_bps: pool.fee_bps(),
                    };

                    let route = Route::single_step(step, amount_in, amount_out);
                    let quote = SwapQuote::new(
                        *token_in,
                        *token_out,
                        amount_in,
                        amount_out,
                        route,
                        "single_pool".to_string(),
                    );

                    // Keep if this is better than current best
                    best_quote = match best_quote {
                        None => Some(quote),
                        Some(current_best) => {
                            if quote.better_than(&current_best) {
                                Some(quote)
                            } else {
                                Some(current_best)
                            }
                        }
                    };
                }
                Err(_) => continue,
            }
        }

        best_quote.ok_or(RouterError::NoRouteFound)
    }

    /// Find all viable pools for a token pair (for analysis/debugging)
    pub fn find_all_routes(
        pools: &[Box<dyn Pool>],
        token_in: &Pubkey,
        token_out: &Pubkey,
        amount_in: u64,
    ) -> Vec<SwapQuote> {
        let mut quotes = Vec::new();

        for pool in pools {
            let (matches, a_to_b) = if pool.token_a() == token_in && pool.token_b() == token_out {
                (true, true)
            } else if pool.token_b() == token_in && pool.token_a() == token_out {
                (true, false)
            } else {
                (false, false)
            };

            if !matches || !pool.has_sufficient_liquidity(amount_in, a_to_b) {
                continue;
            }

            if let Ok((amount_out, price_impact)) = pool.calculate_output(amount_in, a_to_b) {
                let step = RouteStep {
                    pool_address: *pool.address(),
                    dex: pool.dex_name().to_string(),
                    token_in: *token_in,
                    token_out: *token_out,
                    amount_in,
                    amount_out,
                    price_impact_bps: price_impact,
                    fee_bps: pool.fee_bps(),
                };

                let route = Route::single_step(step, amount_in, amount_out);
                let quote = SwapQuote::new(
                    *token_in,
                    *token_out,
                    amount_in,
                    amount_out,
                    route,
                    "single_pool".to_string(),
                );
                quotes.push(quote);
            }
        }

        // Sort by output amount (descending)
        quotes.sort_by(|a, b| b.amount_out.cmp(&a.amount_out));
        quotes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dex::{OrcaPool, RaydiumPool};

    fn create_test_pools() -> Vec<Box<dyn Pool>> {
        let token_a = Pubkey::new_unique();
        let token_b = Pubkey::new_unique();

        vec![
            Box::new(RaydiumPool::new(
                Pubkey::new_unique(),
                token_a,
                token_b,
                1_000_000_000, // Good liquidity
                50_000_000_000,
            )) as Box<dyn Pool>,
            Box::new(OrcaPool::new_constant_product(
                Pubkey::new_unique(),
                token_a,
                token_b,
                2_000_000_000, // Better liquidity, but higher fee
                100_000_000_000,
            )) as Box<dyn Pool>,
        ]
    }

    #[test]
    fn test_find_best_route() {
        let pools = create_test_pools();
        let token_a = *pools[0].token_a();
        let token_b = *pools[0].token_b();

        let quote = SinglePoolRouter::find_best_route(&pools, &token_a, &token_b, 1_000_000)
            .unwrap();

        assert_eq!(quote.token_in, token_a);
        assert_eq!(quote.token_out, token_b);
        assert_eq!(quote.amount_in, 1_000_000);
        assert!(quote.amount_out > 0);
        assert_eq!(quote.strategy, "single_pool");
    }

    #[test]
    fn test_no_route_found() {
        let pools = create_test_pools();
        let wrong_token = Pubkey::new_unique();
        let token_b = *pools[0].token_b();

        let result = SinglePoolRouter::find_best_route(&pools, &wrong_token, &token_b, 1_000_000);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RouterError::NoRouteFound));
    }

    #[test]
    fn test_find_all_routes() {
        let pools = create_test_pools();
        let token_a = *pools[0].token_a();
        let token_b = *pools[0].token_b();

        let quotes = SinglePoolRouter::find_all_routes(&pools, &token_a, &token_b, 1_000_000);

        assert_eq!(quotes.len(), 2); // Should find both pools
        // Should be sorted by output (best first)
        assert!(quotes[0].amount_out >= quotes[1].amount_out);
    }

    #[test]
    fn test_reverse_direction() {
        let pools = create_test_pools();
        let token_a = *pools[0].token_a();
        let token_b = *pools[0].token_b();

        // Swap B to A instead of A to B
        let quote = SinglePoolRouter::find_best_route(&pools, &token_b, &token_a, 1_000_000)
            .unwrap();

        assert_eq!(quote.token_in, token_b);
        assert_eq!(quote.token_out, token_a);
        assert!(quote.amount_out > 0);
    }

    #[test]
    fn test_choose_pool_with_better_output() {
        let token_a = Pubkey::new_unique();
        let token_b = Pubkey::new_unique();

        let pools: Vec<Box<dyn Pool>> = vec![
            Box::new(RaydiumPool::new(
                Pubkey::new_unique(),
                token_a,
                token_b,
                1_000_000_000,
                50_000_000_000,
            )), // 0.25% fee
            Box::new(OrcaPool::new_whirlpool(
                Pubkey::new_unique(),
                token_a,
                token_b,
                1_000_000_000,
                50_000_000_000,
                10, // 0.1% fee - should give better output
            )),
        ];

        let quote = SinglePoolRouter::find_best_route(&pools, &token_a, &token_b, 1_000_000)
            .unwrap();

        // Should choose Orca due to lower fee
        assert_eq!(quote.route.steps[0].dex, "Orca");
    }
}
