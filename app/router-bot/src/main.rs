//! Solana Liquidity Router Bot
//!
//! A bot that finds optimal swap routes across multiple Solana DEXes

use clap::Parser;
use log::{error, info};
use router_bot::*;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

fn main() {
    // Parse CLI arguments
    let args = config::CliArgs::parse();

    // Initialize logger
    if args.verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    // Run the bot
    if let Err(e) = run(args) {
        error!("❌ Error: {}", e);
        std::process::exit(1);
    }
}

fn run(args: config::CliArgs) -> Result<()> {
    info!("🚀 Solana Liquidity Router Bot");

    // Load configuration
    let config = Config::from_args(args.clone())?;
    info!("📡 Connecting to {}", config.network);

    // Create client
    let client = SolanaClient::new(config.rpc_url.clone());

    // Test connection
    match client.get_version() {
        Ok(version) => info!("✅ Connected to Solana (version: {})", version),
        Err(e) => {
            error!("❌ Failed to connect to Solana: {}", e);
            return Err(e.into());
        }
    }

    // Check if we're running in demo mode or actual swap mode
    if args.token_in.is_some() && args.token_out.is_some() && args.amount.is_some() {
        // Actual swap mode
        run_swap(&client, &config, &args)
    } else {
        // Demo mode - show example routes
        run_demo(&client, &config)
    }
}

fn run_swap(client: &SolanaClient, config: &Config, args: &config::CliArgs) -> Result<()> {
    let token_in = Pubkey::from_str(args.token_in.as_ref().unwrap())
        .map_err(|e| RouterError::InvalidAccountData(e.to_string()))?;
    let token_out = Pubkey::from_str(args.token_out.as_ref().unwrap())
        .map_err(|e| RouterError::InvalidAccountData(e.to_string()))?;
    let amount_in = args.amount.unwrap();

    info!("💱 Finding routes for swap:");
    info!("   Token In:  {}", token_in);
    info!("   Token Out: {}", token_out);
    info!("   Amount:    {}", amount_in);
    info!("   Strategy:  {}", config.strategy);

    // Create example pools (in production, these would be fetched from chain)
    let pools = create_example_pools(&token_in, &token_out);

    if pools.is_empty() {
        error!("❌ No pools found for this token pair");
        return Err(RouterError::NoRouteFound.into());
    }

    info!("📊 Found {} pools", pools.len());

    // Find best route based on strategy
    let quote = match config.strategy.as_str() {
        "single" => {
            info!("🔍 Using single pool strategy");
            router::SinglePoolRouter::find_best_route(&pools, &token_in, &token_out, amount_in)?
        }
        "split" => {
            info!("🔍 Using split routing strategy");
            router::SplitRouter::find_best_route(&pools, &token_in, &token_out, amount_in)?
        }
        "multihop" => {
            info!("🔍 Using multi-hop routing strategy");
            router::MultiHopRouter::find_best_route(
                &pools,
                &token_in,
                &token_out,
                amount_in,
                config.max_hops,
            )?
        }
        "all" => {
            info!("🔍 Comparing all routing strategies");
            find_best_overall_route(&pools, &token_in, &token_out, amount_in, config.max_hops)?
        }
        _ => {
            error!("❌ Unknown strategy: {}", config.strategy);
            return Err(
                RouterError::ConfigError(format!("Unknown strategy: {}", config.strategy)).into(),
            );
        }
    };

    // Display results
    print_quote(&quote);

    // Execute if not dry run
    let executor = executor::Executor::new(client.clone(), config.dry_run);
    let result = executor.execute(&quote)?;

    if result.success {
        info!("✅ Swap completed successfully!");
        if let Some(sig) = result.signature {
            info!("   Transaction: {}", sig);
        }
    } else {
        error!("❌ Swap failed: {:?}", result.error);
    }

    Ok(())
}

fn run_demo(_client: &SolanaClient, config: &Config) -> Result<()> {
    info!("🎯 Running in demo mode");
    info!("   Use --token-in, --token-out, and --amount for actual swaps");

    // Create example token pair
    let token_a = Pubkey::new_unique();
    let token_b = Pubkey::new_unique();
    let amount = 1_000_000_000; // 1 token

    info!("\n📊 Example: Swapping {} units of Token A for Token B", amount);

    let pools = create_example_pools(&token_a, &token_b);
    info!("   Created {} example pools", pools.len());

    // Compare strategies
    info!("\n🔍 Comparing routing strategies:");

    if let Ok(single_quote) =
        router::SinglePoolRouter::find_best_route(&pools, &token_a, &token_b, amount)
    {
        info!("\n   1️⃣  Single Pool:");
        info!("      Output: {}", single_quote.amount_out);
        info!("      DEX: {}", single_quote.route.steps[0].dex);
    }

    if let Ok(split_quote) =
        router::SplitRouter::find_best_route(&pools, &token_a, &token_b, amount)
    {
        info!("\n   2️⃣  Split Routing:");
        info!("      Output: {}", split_quote.amount_out);
        info!("      Pools used: {}", split_quote.route.steps.len());
    }

    if let Ok(multihop_quote) = router::MultiHopRouter::find_best_route(
        &pools,
        &token_a,
        &token_b,
        amount,
        config.max_hops,
    ) {
        info!("\n   3️⃣  Multi-hop Routing:");
        info!("      Output: {}", multihop_quote.amount_out);
        info!("      Hops: {}", multihop_quote.route.hop_count());
    }

    info!("\n✅ Demo complete!");
    Ok(())
}

fn create_example_pools(token_a: &Pubkey, token_b: &Pubkey) -> Vec<Box<dyn types::Pool>> {
    use dex::*;

    vec![
        Box::new(RaydiumPool::new(
            Pubkey::new_unique(),
            *token_a,
            *token_b,
            1_000_000_000,
            50_000_000_000,
        )),
        Box::new(OrcaPool::new_constant_product(
            Pubkey::new_unique(),
            *token_a,
            *token_b,
            2_000_000_000,
            100_000_000_000,
        )),
        Box::new(OrcaPool::new_whirlpool(
            Pubkey::new_unique(),
            *token_a,
            *token_b,
            1_500_000_000,
            75_000_000_000,
            10,
        )),
        Box::new(MeteoraPool::new(
            Pubkey::new_unique(),
            *token_a,
            *token_b,
            1_200_000_000,
            60_000_000_000,
            20,
        )),
    ]
}

fn find_best_overall_route(
    pools: &[Box<dyn types::Pool>],
    token_in: &Pubkey,
    token_out: &Pubkey,
    amount_in: u64,
    max_hops: usize,
) -> Result<types::SwapQuote> {
    let mut best_quote: Option<types::SwapQuote> = None;

    // Try single pool
    if let Ok(quote) = router::SinglePoolRouter::find_best_route(pools, token_in, token_out, amount_in) {
        info!("   Single pool: {} output", quote.amount_out);
        best_quote = Some(quote);
    }

    // Try split routing
    if let Ok(quote) = router::SplitRouter::find_best_route(pools, token_in, token_out, amount_in) {
        info!("   Split routing: {} output", quote.amount_out);
        best_quote = match best_quote {
            None => Some(quote),
            Some(current) => {
                if quote.better_than(&current) {
                    Some(quote)
                } else {
                    Some(current)
                }
            }
        };
    }

    // Try multi-hop
    if let Ok(quote) =
        router::MultiHopRouter::find_best_route(pools, token_in, token_out, amount_in, max_hops)
    {
        info!("   Multi-hop: {} output", quote.amount_out);
        best_quote = match best_quote {
            None => Some(quote),
            Some(current) => {
                if quote.better_than(&current) {
                    Some(quote)
                } else {
                    Some(current)
                }
            }
        };
    }

    best_quote.ok_or_else(|| RouterError::NoRouteFound.into())
}

fn print_quote(quote: &types::SwapQuote) {
    info!("\n💰 Best Route Found:");
    info!("   Strategy:      {}", quote.strategy);
    info!("   Input Amount:  {}", quote.amount_in);
    info!("   Output Amount: {}", quote.amount_out);
    info!(
        "   Price Impact:  {:.2}%",
        quote.price_impact_bps as f64 / 100.0
    );
    info!("   Hops:          {}", quote.route.hop_count());

    for (idx, step) in quote.route.steps.iter().enumerate() {
        info!("\n   Step {}:", idx + 1);
        info!("      DEX:           {}", step.dex);
        info!("      Pool:          {}", step.pool_address);
        info!("      Amount In:     {}", step.amount_in);
        info!("      Amount Out:    {}", step.amount_out);
        info!("      Fee:           {:.2}%", step.fee_bps as f64 / 100.0);
        info!(
            "      Price Impact:  {:.2}%",
            step.price_impact_bps as f64 / 100.0
        );
    }
}
