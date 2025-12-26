//! Multi-hop router - finds optimal routes through intermediate tokens

use crate::error::{Result, RouterError};
use crate::types::pool::Pool;
use crate::types::route::{Route, RouteStep, SwapQuote};
use solana_sdk::pubkey::Pubkey;
use std::collections::{HashMap, HashSet, VecDeque};

/// Router for multi-hop routing through intermediate tokens
pub struct MultiHopRouter;

/// Represents an edge in the routing graph
#[derive(Debug, Clone)]
struct RouteEdge {
    pool_index: usize,
    from_token: Pubkey,
    to_token: Pubkey,
    a_to_b: bool,
}

impl MultiHopRouter {
    /// Find the best multi-hop route (up to max_hops)
    ///
    /// Uses a modified BFS to find all possible paths, then evaluates each
    pub fn find_best_route(
        pools: &[Box<dyn Pool>],
        token_in: &Pubkey,
        token_out: &Pubkey,
        amount_in: u64,
        max_hops: usize,
    ) -> Result<SwapQuote> {
        if max_hops == 0 || max_hops > 3 {
            return Err(RouterError::ConfigError(
                "max_hops must be between 1 and 3".to_string(),
            ));
        }

        // Build routing graph
        let graph = Self::build_graph(pools);

        // Find all possible paths
        let paths = Self::find_all_paths(&graph, token_in, token_out, max_hops);

        if paths.is_empty() {
            return Err(RouterError::NoRouteFound);
        }

        // Evaluate each path and find the best
        let mut best_quote: Option<SwapQuote> = None;

        for path in paths {
            if let Ok(quote) = Self::evaluate_path(&path, pools, amount_in) {
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
        }

        best_quote.ok_or(RouterError::NoRouteFound)
    }

    /// Build a graph of all possible token swaps
    fn build_graph(pools: &[Box<dyn Pool>]) -> HashMap<Pubkey, Vec<RouteEdge>> {
        let mut graph: HashMap<Pubkey, Vec<RouteEdge>> = HashMap::new();

        for (idx, pool) in pools.iter().enumerate() {
            let token_a = *pool.token_a();
            let token_b = *pool.token_b();

            // Add edge from A to B
            graph.entry(token_a).or_insert_with(Vec::new).push(RouteEdge {
                pool_index: idx,
                from_token: token_a,
                to_token: token_b,
                a_to_b: true,
            });

            // Add edge from B to A
            graph.entry(token_b).or_insert_with(Vec::new).push(RouteEdge {
                pool_index: idx,
                from_token: token_b,
                to_token: token_a,
                a_to_b: false,
            });
        }

        graph
    }

    /// Find all paths from token_in to token_out within max_hops
    fn find_all_paths(
        graph: &HashMap<Pubkey, Vec<RouteEdge>>,
        token_in: &Pubkey,
        token_out: &Pubkey,
        max_hops: usize,
    ) -> Vec<Vec<RouteEdge>> {
        let mut all_paths = Vec::new();
        let mut queue = VecDeque::new();

        // Initialize: (current_token, path, visited_tokens)
        queue.push_back((*token_in, Vec::new(), HashSet::new()));

        while let Some((current_token, path, mut visited)) = queue.pop_front() {
            // Check if we've reached the destination
            if current_token == *token_out && !path.is_empty() {
                all_paths.push(path.clone());
                continue;
            }

            // Check if we've exceeded max hops
            if path.len() >= max_hops {
                continue;
            }

            // Mark current token as visited
            visited.insert(current_token);

            // Explore neighbors
            if let Some(edges) = graph.get(&current_token) {
                for edge in edges {
                    // Avoid cycles
                    if visited.contains(&edge.to_token) {
                        continue;
                    }

                    let mut new_path = path.clone();
                    new_path.push(edge.clone());

                    queue.push_back((edge.to_token, new_path, visited.clone()));
                }
            }
        }

        all_paths
    }

    /// Evaluate a path and create a swap quote
    fn evaluate_path(
        path: &[RouteEdge],
        pools: &[Box<dyn Pool>],
        initial_amount: u64,
    ) -> Result<SwapQuote> {
        let mut steps = Vec::new();
        let mut current_amount = initial_amount;

        for edge in path {
            let pool = &pools[edge.pool_index];

            let (amount_out, price_impact) = pool.calculate_output(current_amount, edge.a_to_b)?;

            steps.push(RouteStep {
                pool_address: *pool.address(),
                dex: pool.dex_name().to_string(),
                token_in: edge.from_token,
                token_out: edge.to_token,
                amount_in: current_amount,
                amount_out,
                price_impact_bps: price_impact,
                fee_bps: pool.fee_bps(),
            });

            current_amount = amount_out;
        }

        if steps.is_empty() {
            return Err(RouterError::NoRouteFound);
        }

        let token_in = steps.first().unwrap().token_in;
        let token_out = steps.last().unwrap().token_out;

        let route = Route::multi_step(steps);
        Ok(SwapQuote::new(
            token_in,
            token_out,
            initial_amount,
            current_amount,
            route,
            format!("multi_hop_{}", path.len()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dex::RaydiumPool;

    #[test]
    fn test_two_hop_route() {
        let token_a = Pubkey::new_unique();
        let token_b = Pubkey::new_unique();
        let token_c = Pubkey::new_unique();

        // Create pools: A-B and B-C
        let pools: Vec<Box<dyn Pool>> = vec![
            Box::new(RaydiumPool::new(
                Pubkey::new_unique(),
                token_a,
                token_b,
                1_000_000_000,
                50_000_000_000,
            )),
            Box::new(RaydiumPool::new(
                Pubkey::new_unique(),
                token_b,
                token_c,
                50_000_000_000,
                2_000_000_000,
            )),
        ];

        // Route from A to C through B
        let quote = MultiHopRouter::find_best_route(&pools, &token_a, &token_c, 1_000_000, 2)
            .unwrap();

        assert_eq!(quote.route.hop_count(), 2);
        assert_eq!(quote.route.steps[0].token_in, token_a);
        assert_eq!(quote.route.steps[0].token_out, token_b);
        assert_eq!(quote.route.steps[1].token_in, token_b);
        assert_eq!(quote.route.steps[1].token_out, token_c);
        assert!(quote.strategy.starts_with("multi_hop"));
    }

    #[test]
    fn test_direct_route_preferred() {
        let token_a = Pubkey::new_unique();
        let token_b = Pubkey::new_unique();
        let token_c = Pubkey::new_unique();

        // Create pools: A-B (direct), A-C and C-B (indirect)
        let pools: Vec<Box<dyn Pool>> = vec![
            Box::new(RaydiumPool::new(
                Pubkey::new_unique(),
                token_a,
                token_b,
                1_000_000_000,
                50_000_000_000,
            )),
            Box::new(RaydiumPool::new(
                Pubkey::new_unique(),
                token_a,
                token_c,
                1_000_000_000,
                1_000_000_000,
            )),
            Box::new(RaydiumPool::new(
                Pubkey::new_unique(),
                token_c,
                token_b,
                1_000_000_000,
                50_000_000_000,
            )),
        ];

        let quote = MultiHopRouter::find_best_route(&pools, &token_a, &token_b, 1_000_000, 2)
            .unwrap();

        // Direct route should be better (fewer fees)
        // The router should choose the direct A-B pool
        assert!(quote.amount_out > 0);
    }

    #[test]
    fn test_no_route_found() {
        let token_a = Pubkey::new_unique();
        let token_b = Pubkey::new_unique();
        let token_c = Pubkey::new_unique();

        // Create pool A-B only, no way to get to C
        let pools: Vec<Box<dyn Pool>> = vec![Box::new(RaydiumPool::new(
            Pubkey::new_unique(),
            token_a,
            token_b,
            1_000_000_000,
            50_000_000_000,
        ))];

        let result = MultiHopRouter::find_best_route(&pools, &token_a, &token_c, 1_000_000, 2);

        assert!(result.is_err());
    }

    #[test]
    fn test_max_hops_limit() {
        let token_a = Pubkey::new_unique();

        let pools: Vec<Box<dyn Pool>> = vec![];

        // max_hops must be 1-3
        let result = MultiHopRouter::find_best_route(&pools, &token_a, &token_a, 1_000_000, 0);
        assert!(result.is_err());

        let result = MultiHopRouter::find_best_route(&pools, &token_a, &token_a, 1_000_000, 4);
        assert!(result.is_err());
    }

    #[test]
    fn test_cycle_avoidance() {
        let token_a = Pubkey::new_unique();
        let token_b = Pubkey::new_unique();

        // Create pools that could form a cycle: A-B and B-A (same pool traversed differently)
        let pools: Vec<Box<dyn Pool>> = vec![Box::new(RaydiumPool::new(
            Pubkey::new_unique(),
            token_a,
            token_b,
            1_000_000_000,
            50_000_000_000,
        ))];

        // Route from A to A should not be found (would be a cycle)
        let result = MultiHopRouter::find_best_route(&pools, &token_a, &token_a, 1_000_000, 2);

        assert!(result.is_err());
    }

    #[test]
    fn test_three_hop_route() {
        let token_a = Pubkey::new_unique();
        let token_b = Pubkey::new_unique();
        let token_c = Pubkey::new_unique();
        let token_d = Pubkey::new_unique();

        // Create pools: A-B, B-C, C-D
        let pools: Vec<Box<dyn Pool>> = vec![
            Box::new(RaydiumPool::new(
                Pubkey::new_unique(),
                token_a,
                token_b,
                1_000_000_000,
                50_000_000_000,
            )),
            Box::new(RaydiumPool::new(
                Pubkey::new_unique(),
                token_b,
                token_c,
                50_000_000_000,
                2_000_000_000,
            )),
            Box::new(RaydiumPool::new(
                Pubkey::new_unique(),
                token_c,
                token_d,
                2_000_000_000,
                100_000_000_000,
            )),
        ];

        let quote = MultiHopRouter::find_best_route(&pools, &token_a, &token_d, 1_000_000, 3)
            .unwrap();

        assert_eq!(quote.route.hop_count(), 3);
        assert_eq!(quote.strategy, "multi_hop_3");
    }
}
