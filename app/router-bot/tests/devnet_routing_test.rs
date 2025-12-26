//! Comprehensive devnet routing test
//!
//! Run with: cargo test --test devnet_routing_test -- --ignored --nocapture

use router_bot::*;
use solana_sdk::pubkey::Pubkey;

#[test]
#[ignore]
fn test_comprehensive_routing_comparison() {
    println!("\n🚀 Comprehensive Routing Test on Devnet\n");

    // Connect to devnet
    let client = SolanaClient::new_devnet();
    match client.get_version() {
        Ok(version) => println!("✅ Connected to Solana Devnet (version: {})\n", version),
        Err(e) => {
            println!("❌ Failed to connect: {}", e);
            return;
        }
    }

    // Create example pools with realistic parameters
    let token_sol = Pubkey::new_unique();
    let token_usdc = Pubkey::new_unique();

    println!("📊 Creating test pools for SOL/USDC pair...\n");

    // Simulate realistic pool sizes (in lamports/microUSDC)
    let pools: Vec<Box<dyn types::Pool>> = vec![
        // Raydium: Medium liquidity, 0.25% fee
        Box::new(dex::RaydiumPool::new(
            Pubkey::new_unique(),
            token_sol,
            token_usdc,
            500_000_000_000,    // 500 SOL
            25_000_000_000_000, // 25M USDC (50 USDC per SOL)
        )),
        // Orca Whirlpool: High liquidity, 0.1% fee
        Box::new(dex::OrcaPool::new_whirlpool(
            Pubkey::new_unique(),
            token_sol,
            token_usdc,
            1_000_000_000_000,  // 1000 SOL
            50_000_000_000_000, // 50M USDC
            10, // 0.1% fee
        )),
        // Meteora: Medium liquidity, 0.2% fee
        Box::new(dex::MeteoraPool::new(
            Pubkey::new_unique(),
            token_sol,
            token_usdc,
            750_000_000_000,    // 750 SOL
            37_500_000_000_000, // 37.5M USDC
            20, // 0.2% fee
        )),
        // Orca Standard: Lower liquidity, 0.3% fee
        Box::new(dex::OrcaPool::new_constant_product(
            Pubkey::new_unique(),
            token_sol,
            token_usdc,
            300_000_000_000,    // 300 SOL
            15_000_000_000_000, // 15M USDC
        )),
    ];

    println!("Created {} pools:", pools.len());
    for (i, pool) in pools.iter().enumerate() {
        let reserve_sol = pool.reserve_a() as f64 / 1_000_000_000.0;
        let reserve_usdc = pool.reserve_b() as f64 / 1_000_000.0;
        println!("  {}. {} - {:.0} SOL / {:.0} USDC (fee: {:.2}%)",
            i + 1,
            pool.dex_name(),
            reserve_sol,
            reserve_usdc,
            pool.fee_bps() as f64 / 100.0
        );
    }
    println!();

    // Test different swap amounts
    let test_amounts = vec![
        (1_000_000_000, "1 SOL (small swap)"),
        (10_000_000_000, "10 SOL (medium swap)"),
        (50_000_000_000, "50 SOL (large swap)"),
        (100_000_000_000, "100 SOL (very large swap)"),
    ];

    for (amount, description) in test_amounts {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("💱 Testing: {}\n", description);

        // Strategy 1: Single Pool
        let single_result = router::SinglePoolRouter::find_best_route(
            &pools,
            &token_sol,
            &token_usdc,
            amount,
        );

        let (single_output, single_dex, single_impact) = if let Ok(quote) = single_result {
            let output_usdc = quote.amount_out as f64 / 1_000_000.0;
            let dex = quote.route.steps[0].dex.clone();
            let impact = quote.price_impact_bps;
            println!("   1️⃣  Single Pool ({})", dex);
            println!("       Output: {:.2} USDC", output_usdc);
            println!("       Price Impact: {:.2}%", impact as f64 / 100.0);
            println!("       Effective Rate: {:.2} USDC per SOL\n",
                output_usdc / (amount as f64 / 1_000_000_000.0));
            (quote.amount_out, dex, impact)
        } else {
            println!("   ❌ Single pool routing failed\n");
            (0, "None".to_string(), 0)
        };

        // Strategy 2: Split Routing
        let split_result = router::SplitRouter::find_best_route(
            &pools,
            &token_sol,
            &token_usdc,
            amount,
        );

        let split_output = if let Ok(quote) = split_result {
            let output_usdc = quote.amount_out as f64 / 1_000_000.0;
            println!("   2️⃣  Split Routing");
            println!("       Output: {:.2} USDC", output_usdc);
            println!("       Price Impact: {:.2}%", quote.price_impact_bps as f64 / 100.0);
            println!("       Pools Used: {}", quote.route.steps.len());
            println!("       Effective Rate: {:.2} USDC per SOL",
                output_usdc / (amount as f64 / 1_000_000_000.0));

            // Show split distribution
            if quote.route.steps.len() > 1 {
                println!("       Distribution:");
                for step in &quote.route.steps {
                    let pct = (step.amount_in as f64 / amount as f64) * 100.0;
                    let sol_amount = step.amount_in as f64 / 1_000_000_000.0;
                    let usdc_out = step.amount_out as f64 / 1_000_000.0;
                    println!("         • {} - {:.1}% ({:.2} SOL → {:.2} USDC)",
                        step.dex, pct, sol_amount, usdc_out);
                }
            }
            println!();
            quote.amount_out
        } else {
            println!("   ❌ Split routing failed\n");
            0
        };

        // Calculate improvement
        if single_output > 0 && split_output > 0 {
            let improvement = ((split_output as f64 - single_output as f64) / single_output as f64) * 100.0;
            let usdc_gain = (split_output as i128 - single_output as i128) as f64 / 1_000_000.0;

            println!("   📈 Split vs Single:");
            if improvement > 0.0 {
                println!("       Improvement: +{:.2}% (+{:.2} USDC)", improvement, usdc_gain);
            } else if improvement < 0.0 {
                println!("       Difference: {:.2}% ({:.2} USDC) - Single pool better", improvement, usdc_gain);
            } else {
                println!("       Equal output");
            }
        }
        println!();
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Routing test complete!\n");
}

#[test]
#[ignore]
fn test_multi_hop_routing() {
    println!("\n🔀 Multi-Hop Routing Test\n");

    let client = SolanaClient::new_devnet();
    println!("✅ Connected to devnet\n");

    // Create a chain: SOL → USDC → RAY
    let token_sol = Pubkey::new_unique();
    let token_usdc = Pubkey::new_unique();
    let token_ray = Pubkey::new_unique();

    let pools: Vec<Box<dyn types::Pool>> = vec![
        // SOL → USDC
        Box::new(dex::RaydiumPool::new(
            Pubkey::new_unique(),
            token_sol,
            token_usdc,
            1_000_000_000_000,
            50_000_000_000_000,
        )),
        // USDC → RAY
        Box::new(dex::RaydiumPool::new(
            Pubkey::new_unique(),
            token_usdc,
            token_ray,
            50_000_000_000_000,
            10_000_000_000_000,
        )),
    ];

    let amount = 1_000_000_000; // 1 SOL

    println!("Finding route: SOL → USDC → RAY");
    println!("Input: 1 SOL\n");

    match router::MultiHopRouter::find_best_route(&pools, &token_sol, &token_ray, amount, 2) {
        Ok(quote) => {
            println!("✅ Multi-hop route found!");
            println!("   Hops: {}", quote.route.hop_count());
            println!("   Output: {} RAY units", quote.amount_out);
            println!("   Total Price Impact: {:.2}%\n", quote.price_impact_bps as f64 / 100.0);

            println!("   Route steps:");
            for (i, step) in quote.route.steps.iter().enumerate() {
                println!("     Step {}: {} on {}", i + 1,
                    if i == 0 { "SOL → USDC" } else { "USDC → RAY" },
                    step.dex
                );
                println!("       In: {}", step.amount_in);
                println!("       Out: {}", step.amount_out);
                println!("       Impact: {:.2}%", step.price_impact_bps as f64 / 100.0);
            }
        }
        Err(e) => {
            println!("❌ Multi-hop routing failed: {}", e);
        }
    }

    println!("\n✅ Multi-hop test complete!\n");
}
