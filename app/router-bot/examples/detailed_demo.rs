//! Detailed demo showing exactly how the router works
//!
//! Run with: cargo run --example detailed_demo

use router_bot::*;
use solana_sdk::pubkey::Pubkey;

fn main() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║        🤖 SOLANA LIQUIDITY ROUTER - DETAILED DEMO           ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // Step 1: Create test tokens
    println!("📝 STEP 1: Creating test token pair");
    println!("─────────────────────────────────────────────────────────────");
    let token_sol = Pubkey::new_unique();
    let token_usdc = Pubkey::new_unique();
    println!("✓ Token A (SOL):  {}", token_sol);
    println!("✓ Token B (USDC): {}\n", token_usdc);

    // Step 2: Create pools with different characteristics
    println!("📊 STEP 2: Creating liquidity pools");
    println!("─────────────────────────────────────────────────────────────");

    let pools: Vec<Box<dyn types::Pool>> = vec![
        Box::new(dex::RaydiumPool::new(
            Pubkey::new_unique(),
            token_sol,
            token_usdc,
            1_000_000_000_000,   // 1,000 SOL
            50_000_000_000_000,  // 50,000 USDC
        )),
        Box::new(dex::OrcaPool::new_constant_product(
            Pubkey::new_unique(),
            token_sol,
            token_usdc,
            2_000_000_000_000,   // 2,000 SOL (better liquidity)
            100_000_000_000_000, // 100,000 USDC
        )),
        Box::new(dex::OrcaPool::new_whirlpool(
            Pubkey::new_unique(),
            token_sol,
            token_usdc,
            1_500_000_000_000,   // 1,500 SOL
            75_000_000_000_000,  // 75,000 USDC
            10, // 0.1% fee (lower than others)
        )),
        Box::new(dex::MeteoraPool::new(
            Pubkey::new_unique(),
            token_sol,
            token_usdc,
            1_200_000_000_000,   // 1,200 SOL
            60_000_000_000_000,  // 60,000 USDC
            20, // 0.2% fee
        )),
    ];

    for (i, pool) in pools.iter().enumerate() {
        let reserve_sol = pool.reserve_a() as f64 / 1_000_000_000.0;
        let reserve_usdc = pool.reserve_b() as f64 / 1_000_000.0;
        let price = reserve_usdc / reserve_sol;
        println!("Pool {}: {}", i + 1, pool.dex_name());
        println!("  ├─ Liquidity: {:.0} SOL / {:.0} USDC", reserve_sol, reserve_usdc);
        println!("  ├─ Price: {:.2} USDC per SOL", price);
        println!("  └─ Fee: {:.2}%\n", pool.fee_bps() as f64 / 100.0);
    }

    // Step 3: Test swap amount
    let amount_in = 10_000_000_000; // 10 SOL
    println!("💱 STEP 3: Swapping {} SOL for USDC", amount_in as f64 / 1_000_000_000.0);
    println!("─────────────────────────────────────────────────────────────\n");

    // Step 4: Single Pool Routing
    println!("🔍 STEP 4: Single Pool Routing Strategy");
    println!("─────────────────────────────────────────────────────────────");
    println!("How it works:");
    println!("  1. Check each pool individually");
    println!("  2. Calculate output for each pool");
    println!("  3. Choose the pool with best output\n");

    // Manually show each pool's calculation
    println!("Calculating outputs for each pool:");
    for (i, pool) in pools.iter().enumerate() {
        match pool.calculate_output(amount_in, true) {
            Ok((output, price_impact)) => {
                let output_usdc = output as f64 / 1_000_000.0;
                println!("  Pool {} ({}): {:.2} USDC (impact: {:.2}%)",
                    i + 1, pool.dex_name(), output_usdc, price_impact as f64 / 100.0);
            }
            Err(e) => {
                println!("  Pool {} ({}): ERROR - {}", i + 1, pool.dex_name(), e);
            }
        }
    }
    println!();

    let single_quote = router::SinglePoolRouter::find_best_route(
        &pools,
        &token_sol,
        &token_usdc,
        amount_in,
    ).expect("Single pool routing failed");

    let single_output = single_quote.amount_out as f64 / 1_000_000.0;
    println!("✅ Best pool selected: {}", single_quote.route.steps[0].dex);
    println!("   Output: {:.2} USDC", single_output);
    println!("   Price Impact: {:.2}%", single_quote.price_impact_bps as f64 / 100.0);
    println!("   Effective Rate: {:.2} USDC per SOL\n",
        single_output / (amount_in as f64 / 1_000_000_000.0));

    // Step 5: Split Routing
    println!("🔄 STEP 5: Split Routing Strategy");
    println!("─────────────────────────────────────────────────────────────");
    println!("How it works:");
    println!("  1. Try different split percentages (0%, 10%, 20%... 100%)");
    println!("  2. For each split, calculate total output");
    println!("  3. Choose the split that maximizes output\n");

    let split_quote = router::SplitRouter::find_best_route(
        &pools,
        &token_sol,
        &token_usdc,
        amount_in,
    ).expect("Split routing failed");

    let split_output = split_quote.amount_out as f64 / 1_000_000.0;
    println!("✅ Optimal split found:");
    println!("   Total Output: {:.2} USDC", split_output);
    println!("   Price Impact: {:.2}%", split_quote.price_impact_bps as f64 / 100.0);
    println!("   Pools Used: {}\n", split_quote.route.steps.len());

    println!("   Distribution:");
    for step in &split_quote.route.steps {
        let pct = (step.amount_in as f64 / amount_in as f64) * 100.0;
        let sol = step.amount_in as f64 / 1_000_000_000.0;
        let usdc = step.amount_out as f64 / 1_000_000.0;
        println!("     • {}: {:.1}% ({:.2} SOL → {:.2} USDC)",
            step.dex, pct, sol, usdc);
    }
    println!();

    // Step 6: Comparison
    println!("📊 STEP 6: Strategy Comparison");
    println!("─────────────────────────────────────────────────────────────");
    let improvement = ((split_output - single_output) / single_output) * 100.0;
    let usdc_gain = split_output - single_output;

    println!("Single Pool:  {:.2} USDC", single_output);
    println!("Split Route:  {:.2} USDC", split_output);
    println!("─────────────────────────────────────────");
    if improvement > 0.0 {
        println!("Improvement:  +{:.2}% (+{:.2} USDC) ✅", improvement, usdc_gain);
    } else {
        println!("Difference:   {:.2}% ({:.2} USDC)", improvement, usdc_gain);
    }
    println!();

    // Step 7: Show why split routing works
    println!("💡 STEP 7: Why Split Routing Works Better");
    println!("─────────────────────────────────────────────────────────────");
    println!("Constant product formula: x * y = k");
    println!("  • Large swaps cause more price impact");
    println!("  • Splitting reduces impact per pool");
    println!("  • Total output is higher despite using multiple pools\n");

    println!("Example with our 10 SOL swap:");
    println!("  Single Pool (all 10 SOL in one pool):");
    println!("    - High price impact on that pool");
    println!("    - Output: {:.2} USDC\n", single_output);

    println!("  Split Route (distributed across pools):");
    println!("    - Lower impact on each pool");
    println!("    - Combined output: {:.2} USDC", split_output);
    println!("    - Gain: {:.2} USDC more!\n", usdc_gain);

    // Step 8: Multi-hop example
    println!("🔀 STEP 8: Multi-Hop Routing");
    println!("─────────────────────────────────────────────────────────────");

    let token_ray = Pubkey::new_unique();
    println!("Creating intermediate token (RAY): {}\n", token_ray);

    let multihop_pools: Vec<Box<dyn types::Pool>> = vec![
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

    println!("Path: SOL → USDC → RAY");
    println!("Input: 1 SOL\n");

    match router::MultiHopRouter::find_best_route(
        &multihop_pools,
        &token_sol,
        &token_ray,
        1_000_000_000,
        2,
    ) {
        Ok(quote) => {
            println!("✅ Route found with {} hops:", quote.route.hop_count());
            for (i, step) in quote.route.steps.iter().enumerate() {
                println!("  Step {}: {}", i + 1, step.dex);
                println!("    Input:  {} units", step.amount_in);
                println!("    Output: {} units", step.amount_out);
                println!("    Impact: {:.2}%", step.price_impact_bps as f64 / 100.0);
            }
            println!("\n  Final Output: {} RAY units", quote.amount_out);
            println!("  Total Impact: {:.2}%", quote.price_impact_bps as f64 / 100.0);
        }
        Err(e) => {
            println!("❌ Multi-hop failed: {}", e);
        }
    }

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                    ✅ DEMO COMPLETE                          ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("🎯 Key Takeaways:");
    println!("  1. Router compares all available pools");
    println!("  2. Single pool works best for small swaps");
    println!("  3. Split routing reduces price impact on large swaps");
    println!("  4. Multi-hop routing enables swaps between any token pair");
    println!("  5. The router always finds the optimal route!\n");
}
