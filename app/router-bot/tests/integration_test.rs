//! Integration tests for the router bot
//!
//! Run with: cargo test -- --ignored --test-threads=1

use router_bot::*;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[test]
#[ignore] // Requires network access
fn test_devnet_connection() {
    let client = SolanaClient::new_devnet();
    let version = client.get_version().expect("Failed to connect to devnet");
    println!("✅ Connected to Solana devnet: {}", version);
    assert!(!version.is_empty());
}

#[test]
#[ignore] // Requires network access
fn test_fetch_usdc_mint() {
    let client = SolanaClient::new_devnet();

    // USDC mint on devnet
    let usdc_mint = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";

    let mint = client
        .fetch_mint_str(usdc_mint)
        .expect("Failed to fetch USDC mint");

    println!("✅ USDC Mint fetched successfully");
    println!("   Decimals: {}", mint.decimals);
    println!("   Supply: {}", mint.supply);

    assert_eq!(mint.decimals, 6);
}

#[test]
fn test_single_pool_routing() {
    let token_a = Pubkey::new_unique();
    let token_b = Pubkey::new_unique();

    let pools: Vec<Box<dyn types::Pool>> = vec![
        Box::new(dex::RaydiumPool::new(
            Pubkey::new_unique(),
            token_a,
            token_b,
            1_000_000_000,
            50_000_000_000,
        )),
        Box::new(dex::OrcaPool::new_whirlpool(
            Pubkey::new_unique(),
            token_a,
            token_b,
            1_000_000_000,
            50_000_000_000,
            10, // Lower fee than Raydium
        )),
    ];

    let quote = router::SinglePoolRouter::find_best_route(&pools, &token_a, &token_b, 1_000_000)
        .expect("Failed to find route");

    println!("✅ Single pool routing test passed");
    println!("   Strategy: {}", quote.strategy);
    println!("   Output: {}", quote.amount_out);
    println!("   DEX: {}", quote.route.steps[0].dex);

    assert_eq!(quote.strategy, "single_pool");
    assert!(quote.amount_out > 0);
    // Should choose Orca due to lower fee
    assert_eq!(quote.route.steps[0].dex, "Orca");
}

#[test]
fn test_split_routing() {
    let token_a = Pubkey::new_unique();
    let token_b = Pubkey::new_unique();

    let pools: Vec<Box<dyn types::Pool>> = vec![
        Box::new(dex::RaydiumPool::new(
            Pubkey::new_unique(),
            token_a,
            token_b,
            1_000_000_000,
            50_000_000_000,
        )),
        Box::new(dex::OrcaPool::new_constant_product(
            Pubkey::new_unique(),
            token_a,
            token_b,
            1_000_000_000,
            50_000_000_000,
        )),
    ];

    let quote = router::SplitRouter::find_best_route(&pools, &token_a, &token_b, 100_000_000)
        .expect("Failed to find split route");

    println!("✅ Split routing test passed");
    println!("   Strategy: {}", quote.strategy);
    println!("   Output: {}", quote.amount_out);
    println!("   Pools used: {}", quote.route.steps.len());

    assert_eq!(quote.strategy, "split");
    assert!(quote.amount_out > 0);
}

#[test]
fn test_multi_hop_routing() {
    let token_a = Pubkey::new_unique();
    let token_b = Pubkey::new_unique();
    let token_c = Pubkey::new_unique();

    let pools: Vec<Box<dyn types::Pool>> = vec![
        Box::new(dex::RaydiumPool::new(
            Pubkey::new_unique(),
            token_a,
            token_b,
            1_000_000_000,
            50_000_000_000,
        )),
        Box::new(dex::RaydiumPool::new(
            Pubkey::new_unique(),
            token_b,
            token_c,
            50_000_000_000,
            2_000_000_000,
        )),
    ];

    let quote = router::MultiHopRouter::find_best_route(&pools, &token_a, &token_c, 1_000_000, 2)
        .expect("Failed to find multi-hop route");

    println!("✅ Multi-hop routing test passed");
    println!("   Strategy: {}", quote.strategy);
    println!("   Output: {}", quote.amount_out);
    println!("   Hops: {}", quote.route.hop_count());

    assert!(quote.strategy.starts_with("multi_hop"));
    assert_eq!(quote.route.hop_count(), 2);
    assert!(quote.amount_out > 0);
}

#[test]
fn test_strategy_comparison() {
    let token_a = Pubkey::new_unique();
    let token_b = Pubkey::new_unique();

    // Create pools with different characteristics
    let pools: Vec<Box<dyn types::Pool>> = vec![
        Box::new(dex::RaydiumPool::new(
            Pubkey::new_unique(),
            token_a,
            token_b,
            500_000_000, // Smaller pool
            25_000_000_000,
        )),
        Box::new(dex::RaydiumPool::new(
            Pubkey::new_unique(),
            token_a,
            token_b,
            500_000_000, // Smaller pool
            25_000_000_000,
        )),
        Box::new(dex::OrcaPool::new_whirlpool(
            Pubkey::new_unique(),
            token_a,
            token_b,
            2_000_000_000, // Larger pool, lower fee
            100_000_000_000,
            10,
        )),
    ];

    let amount = 50_000_000; // Large swap

    // Single pool
    let single_quote =
        router::SinglePoolRouter::find_best_route(&pools, &token_a, &token_b, amount)
            .expect("Single pool routing failed");

    // Split routing
    let split_quote = router::SplitRouter::find_best_route(&pools, &token_a, &token_b, amount)
        .expect("Split routing failed");

    println!("✅ Strategy comparison test passed");
    println!("   Single pool output: {}", single_quote.amount_out);
    println!("   Split routing output: {}", split_quote.amount_out);

    // Both should produce valid outputs
    assert!(single_quote.amount_out > 0);
    assert!(split_quote.amount_out > 0);

    // For large swaps, split might be better (but not guaranteed with our simple pools)
    println!(
        "   Improvement: {:.2}%",
        (split_quote.amount_out as f64 / single_quote.amount_out as f64 - 1.0) * 100.0
    );
}

#[test]
fn test_executor_dry_run() {
    let client = SolanaClient::new_devnet();
    let executor = executor::Executor::new(client, true);

    let token_a = Pubkey::new_unique();
    let token_b = Pubkey::new_unique();

    let pools: Vec<Box<dyn types::Pool>> = vec![Box::new(dex::RaydiumPool::new(
        Pubkey::new_unique(),
        token_a,
        token_b,
        1_000_000_000,
        50_000_000_000,
    ))];

    let quote = router::SinglePoolRouter::find_best_route(&pools, &token_a, &token_b, 1_000_000)
        .expect("Failed to find route");

    let result = executor.execute(&quote).expect("Execution failed");

    println!("✅ Executor dry run test passed");
    println!("   Success: {}", result.success);
    println!("   Simulated output: {:?}", result.simulated_output);

    assert!(result.success);
    assert!(result.signature.is_none()); // Dry run shouldn't have signature
    assert_eq!(result.simulated_output, Some(quote.amount_out));
}

#[test]
fn test_config_creation() {
    use config::CliArgs;

    let args = CliArgs {
        rpc_url: Some("https://custom.rpc.com".to_string()),
        network: "mainnet-beta".to_string(),
        token_in: None,
        token_out: None,
        amount: None,
        strategy: "single".to_string(),
        max_hops: 2,
        dry_run: true,
        config: None,
        verbose: false,
    };

    let config = Config::from_args(args).expect("Failed to create config");

    println!("✅ Config creation test passed");
    println!("   RPC URL: {}", config.rpc_url);
    println!("   Strategy: {}", config.strategy);

    assert_eq!(config.rpc_url, "https://custom.rpc.com");
    assert_eq!(config.strategy, "single");
    assert_eq!(config.max_hops, 2);
}

#[test]
fn test_calculator_accuracy() {
    // Test constant product formula with known values
    let reserve_in = 1_000_000_000u64;
    let reserve_out = 50_000_000_000u64;
    let amount_in = 10_000_000u64; // 1% of reserve
    let fee_bps = 25u16; // 0.25%

    let amount_out =
        calculator::calculate_amount_out(amount_in, reserve_in, reserve_out, fee_bps)
            .expect("Calculation failed");

    println!("✅ Calculator accuracy test passed");
    println!("   Input: {}", amount_in);
    println!("   Output: {}", amount_out);

    // Verify output is reasonable
    assert!(amount_out > 0);
    assert!(amount_out < reserve_out);

    // Calculate expected output manually
    // amount_out ≈ (amount_in * (1 - 0.0025) * reserve_out) / (reserve_in + amount_in * (1 - 0.0025))
    let expected_approx = ((amount_in as f64 * 0.9975 * reserve_out as f64)
        / (reserve_in as f64 + amount_in as f64 * 0.9975)) as u64;

    println!("   Expected (approx): {}", expected_approx);

    // Should be within 1% of expected
    let diff = (amount_out as i64 - expected_approx as i64).abs() as u64;
    assert!(diff < expected_approx / 100);
}
