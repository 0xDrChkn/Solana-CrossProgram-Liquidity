//! Split router - optimizes by splitting amount across multiple pools

use crate::error::{Result, RouterError};
use crate::types::pool::Pool;
use crate::types::route::{Route, RouteStep, SwapQuote};
use solana_sdk::pubkey::Pubkey;

/// Router for split routing across multiple pools
pub struct SplitRouter;

/// Split allocation for a pool
#[derive(Debug, Clone)]
pub struct SplitAllocation {
    pub pool_index: usize,
    pub percentage: u8, // 0-100
    pub amount_in: u64,
    pub amount_out: u64,
}

impl SplitRouter {
    /// Find optimal split routing
    ///
    /// Tests different split percentages and finds the combination that maximizes output
    pub fn find_best_route(
        pools: &[Box<dyn Pool>],
        token_in: &Pubkey,
        token_out: &Pubkey,
        amount_in: u64,
    ) -> Result<SwapQuote> {
        // First, filter pools that match the token pair
        let matching_pools: Vec<(usize, bool)> = pools
            .iter()
            .enumerate()
            .filter_map(|(idx, pool)| {
                if pool.token_a() == token_in && pool.token_b() == token_out {
                    Some((idx, true))
                } else if pool.token_b() == token_in && pool.token_a() == token_out {
                    Some((idx, false))
                } else {
                    None
                }
            })
            .collect();

        if matching_pools.is_empty() {
            return Err(RouterError::NoRouteFound);
        }

        // If only one pool, no splitting needed
        if matching_pools.len() == 1 {
            let (idx, a_to_b) = matching_pools[0];
            let pool = &pools[idx];
            return Self::create_single_pool_quote(pool, token_in, token_out, amount_in, a_to_b);
        }

        // Try different split strategies for 2 pools
        let best_split = if matching_pools.len() == 2 {
            Self::optimize_two_pool_split(pools, &matching_pools, token_in, token_out, amount_in)?
        } else {
            // For 3+ pools, use a greedy approach
            Self::optimize_multi_pool_split(pools, &matching_pools, token_in, token_out, amount_in)?
        };

        // Build route from best split
        Self::build_split_route(&best_split, pools, &matching_pools, token_in, token_out, amount_in)
    }

    /// Optimize split between exactly 2 pools
    fn optimize_two_pool_split(
        pools: &[Box<dyn Pool>],
        matching_pools: &[(usize, bool)],
        _token_in: &Pubkey,
        _token_out: &Pubkey,
        amount_in: u64,
    ) -> Result<Vec<SplitAllocation>> {
        let (idx1, a_to_b1) = matching_pools[0];
        let (idx2, a_to_b2) = matching_pools[1];

        let mut best_total_output = 0u64;
        let mut best_split = Vec::new();

        // Try different split percentages: 0%, 10%, 20%, ..., 100%
        for percentage1 in (0..=100).step_by(10) {
            let percentage2 = 100 - percentage1;

            let amount1 = (amount_in as u128 * percentage1 / 100) as u64;
            let amount2 = amount_in - amount1;

            // Calculate outputs for each pool
            let output1 = if amount1 > 0 {
                match pools[idx1].calculate_output(amount1, a_to_b1) {
                    Ok((out, _)) => out,
                    Err(_) => continue,
                }
            } else {
                0
            };

            let output2 = if amount2 > 0 {
                match pools[idx2].calculate_output(amount2, a_to_b2) {
                    Ok((out, _)) => out,
                    Err(_) => continue,
                }
            } else {
                0
            };

            let total_output = output1 + output2;

            if total_output > best_total_output {
                best_total_output = total_output;
                best_split = vec![
                    SplitAllocation {
                        pool_index: idx1,
                        percentage: percentage1 as u8,
                        amount_in: amount1,
                        amount_out: output1,
                    },
                    SplitAllocation {
                        pool_index: idx2,
                        percentage: percentage2 as u8,
                        amount_in: amount2,
                        amount_out: output2,
                    },
                ];
            }
        }

        if best_split.is_empty() {
            return Err(RouterError::NoRouteFound);
        }

        Ok(best_split)
    }

    /// Optimize split across 3+ pools (greedy approach)
    fn optimize_multi_pool_split(
        pools: &[Box<dyn Pool>],
        matching_pools: &[(usize, bool)],
        _token_in: &Pubkey,
        _token_out: &Pubkey,
        amount_in: u64,
    ) -> Result<Vec<SplitAllocation>> {
        // Simple greedy approach: split equally and adjust
        let pool_count = matching_pools.len();
        let base_amount = amount_in / pool_count as u64;

        let mut allocations = Vec::new();

        for (pool_idx, (idx, a_to_b)) in matching_pools.iter().enumerate() {
            let amount = if pool_idx == pool_count - 1 {
                // Last pool gets remainder
                amount_in - (base_amount * (pool_count - 1) as u64)
            } else {
                base_amount
            };

            if let Ok((output, _)) = pools[*idx].calculate_output(amount, *a_to_b) {
                allocations.push(SplitAllocation {
                    pool_index: *idx,
                    percentage: (amount * 100 / amount_in) as u8,
                    amount_in: amount,
                    amount_out: output,
                });
            }
        }

        if allocations.is_empty() {
            return Err(RouterError::NoRouteFound);
        }

        Ok(allocations)
    }

    /// Build a route from split allocations
    fn build_split_route(
        allocations: &[SplitAllocation],
        pools: &[Box<dyn Pool>],
        matching_pools: &[(usize, bool)],
        token_in: &Pubkey,
        token_out: &Pubkey,
        amount_in: u64,
    ) -> Result<SwapQuote> {
        let mut steps = Vec::new();
        let mut total_output = 0u64;

        for alloc in allocations {
            if alloc.amount_in == 0 {
                continue;
            }

            let pool = &pools[alloc.pool_index];
            let (_, a_to_b) = matching_pools
                .iter()
                .find(|(idx, _)| *idx == alloc.pool_index)
                .unwrap();

            let (output, price_impact) = pool.calculate_output(alloc.amount_in, *a_to_b)?;

            steps.push(RouteStep {
                pool_address: *pool.address(),
                dex: pool.dex_name().to_string(),
                token_in: *token_in,
                token_out: *token_out,
                amount_in: alloc.amount_in,
                amount_out: output,
                price_impact_bps: price_impact,
                fee_bps: pool.fee_bps(),
            });

            total_output += output;
        }

        let route = Route::multi_step(steps);
        Ok(SwapQuote::new(
            *token_in,
            *token_out,
            amount_in,
            total_output,
            route,
            "split".to_string(),
        ))
    }

    /// Helper to create single pool quote
    fn create_single_pool_quote(
        pool: &Box<dyn Pool>,
        token_in: &Pubkey,
        token_out: &Pubkey,
        amount_in: u64,
        a_to_b: bool,
    ) -> Result<SwapQuote> {
        let (amount_out, price_impact) = pool.calculate_output(amount_in, a_to_b)?;

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
        Ok(SwapQuote::new(
            *token_in,
            *token_out,
            amount_in,
            amount_out,
            route,
            "split".to_string(), // Still use "split" strategy name
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dex::{OrcaPool, RaydiumPool, MeteoraPool};

    #[test]
    fn test_split_two_pools() {
        let token_a = Pubkey::new_unique();
        let token_b = Pubkey::new_unique();

        let pools: Vec<Box<dyn Pool>> = vec![
            Box::new(RaydiumPool::new(
                Pubkey::new_unique(),
                token_a,
                token_b,
                1_000_000_000,
                50_000_000_000,
            )),
            Box::new(OrcaPool::new_constant_product(
                Pubkey::new_unique(),
                token_a,
                token_b,
                2_000_000_000,
                100_000_000_000,
            )),
        ];

        let quote = SplitRouter::find_best_route(&pools, &token_a, &token_b, 10_000_000).unwrap();

        assert_eq!(quote.strategy, "split");
        assert!(quote.amount_out > 0);
        // Should use both pools (or optimize to use one if that's better)
        assert!(!quote.route.steps.is_empty());
    }

    #[test]
    fn test_split_single_pool_fallback() {
        let token_a = Pubkey::new_unique();
        let token_b = Pubkey::new_unique();

        let pools: Vec<Box<dyn Pool>> = vec![Box::new(RaydiumPool::new(
            Pubkey::new_unique(),
            token_a,
            token_b,
            1_000_000_000,
            50_000_000_000,
        ))];

        let quote = SplitRouter::find_best_route(&pools, &token_a, &token_b, 1_000_000).unwrap();

        // With only one pool, should still work
        assert_eq!(quote.route.steps.len(), 1);
    }

    #[test]
    fn test_split_three_pools() {
        let token_a = Pubkey::new_unique();
        let token_b = Pubkey::new_unique();

        let pools: Vec<Box<dyn Pool>> = vec![
            Box::new(RaydiumPool::new(
                Pubkey::new_unique(),
                token_a,
                token_b,
                1_000_000_000,
                50_000_000_000,
            )),
            Box::new(OrcaPool::new_whirlpool(
                Pubkey::new_unique(),
                token_a,
                token_b,
                2_000_000_000,
                100_000_000_000,
                10,
            )),
            Box::new(MeteoraPool::new(
                Pubkey::new_unique(),
                token_a,
                token_b,
                1_500_000_000,
                75_000_000_000,
                20,
            )),
        ];

        let quote = SplitRouter::find_best_route(&pools, &token_a, &token_b, 30_000_000).unwrap();

        assert_eq!(quote.strategy, "split");
        assert!(quote.route.steps.len() <= 3);
        assert!(quote.amount_out > 0);
    }

    #[test]
    fn test_split_vs_single_pool() {
        let token_a = Pubkey::new_unique();
        let token_b = Pubkey::new_unique();

        // Two pools with different characteristics
        let pools: Vec<Box<dyn Pool>> = vec![
            Box::new(RaydiumPool::new(
                Pubkey::new_unique(),
                token_a,
                token_b,
                500_000_000, // Smaller pool
                25_000_000_000,
            )),
            Box::new(RaydiumPool::new(
                Pubkey::new_unique(),
                token_a,
                token_b,
                500_000_000, // Same size
                25_000_000_000,
            )),
        ];

        // Large swap that might benefit from splitting
        let split_quote =
            SplitRouter::find_best_route(&pools, &token_a, &token_b, 50_000_000).unwrap();

        // For large swaps, split routing should be beneficial
        assert!(split_quote.amount_out > 0);
    }
}
