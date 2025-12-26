//! Benchmarks for routing algorithms
//!
//! Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use router_bot::*;
use solana_sdk::pubkey::Pubkey;

fn create_test_pools(count: usize) -> Vec<Box<dyn types::Pool>> {
    let token_a = Pubkey::new_unique();
    let token_b = Pubkey::new_unique();

    (0..count)
        .map(|i| {
            let dex_type = i % 4;
            match dex_type {
                0 => Box::new(dex::RaydiumPool::new(
                    Pubkey::new_unique(),
                    token_a,
                    token_b,
                    1_000_000_000 + i as u64 * 100_000_000,
                    50_000_000_000 + i as u64 * 5_000_000_000,
                )) as Box<dyn types::Pool>,
                1 => Box::new(dex::OrcaPool::new_constant_product(
                    Pubkey::new_unique(),
                    token_a,
                    token_b,
                    1_000_000_000 + i as u64 * 100_000_000,
                    50_000_000_000 + i as u64 * 5_000_000_000,
                )) as Box<dyn types::Pool>,
                2 => Box::new(dex::OrcaPool::new_whirlpool(
                    Pubkey::new_unique(),
                    token_a,
                    token_b,
                    1_000_000_000 + i as u64 * 100_000_000,
                    50_000_000_000 + i as u64 * 5_000_000_000,
                    10,
                )) as Box<dyn types::Pool>,
                _ => Box::new(dex::MeteoraPool::new(
                    Pubkey::new_unique(),
                    token_a,
                    token_b,
                    1_000_000_000 + i as u64 * 100_000_000,
                    50_000_000_000 + i as u64 * 5_000_000_000,
                    20,
                )) as Box<dyn types::Pool>,
            }
        })
        .collect()
}

fn benchmark_single_pool_routing(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_pool_routing");

    for pool_count in [2, 5, 10, 20] {
        let pools = create_test_pools(pool_count);
        let token_a = *pools[0].token_a();
        let token_b = *pools[0].token_b();
        let amount = 1_000_000u64;

        group.bench_with_input(
            BenchmarkId::from_parameter(pool_count),
            &pool_count,
            |b, _| {
                b.iter(|| {
                    router::SinglePoolRouter::find_best_route(
                        black_box(&pools),
                        black_box(&token_a),
                        black_box(&token_b),
                        black_box(amount),
                    )
                });
            },
        );
    }

    group.finish();
}

fn benchmark_split_routing(c: &mut Criterion) {
    let mut group = c.benchmark_group("split_routing");

    for pool_count in [2, 5, 10] {
        let pools = create_test_pools(pool_count);
        let token_a = *pools[0].token_a();
        let token_b = *pools[0].token_b();
        let amount = 10_000_000u64;

        group.bench_with_input(
            BenchmarkId::from_parameter(pool_count),
            &pool_count,
            |b, _| {
                b.iter(|| {
                    router::SplitRouter::find_best_route(
                        black_box(&pools),
                        black_box(&token_a),
                        black_box(&token_b),
                        black_box(amount),
                    )
                });
            },
        );
    }

    group.finish();
}

fn benchmark_multi_hop_routing(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_hop_routing");

    // Create a chain of pools: A-B-C-D
    let token_a = Pubkey::new_unique();
    let token_b = Pubkey::new_unique();
    let token_c = Pubkey::new_unique();
    let token_d = Pubkey::new_unique();

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
        Box::new(dex::RaydiumPool::new(
            Pubkey::new_unique(),
            token_c,
            token_d,
            2_000_000_000,
            100_000_000_000,
        )),
    ];

    let amount = 1_000_000u64;

    for max_hops in [1, 2, 3] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_hops", max_hops)),
            &max_hops,
            |b, &hops| {
                b.iter(|| {
                    router::MultiHopRouter::find_best_route(
                        black_box(&pools),
                        black_box(&token_a),
                        black_box(&token_d),
                        black_box(amount),
                        black_box(hops),
                    )
                });
            },
        );
    }

    group.finish();
}

fn benchmark_calculator(c: &mut Criterion) {
    let mut group = c.benchmark_group("calculator");

    let reserve_in = 1_000_000_000u64;
    let reserve_out = 50_000_000_000u64;
    let fee_bps = 25u16;

    for amount_in in [1_000u64, 100_000, 10_000_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("amount_{}", amount_in)),
            &amount_in,
            |b, &amount| {
                b.iter(|| {
                    calculator::calculate_amount_out(
                        black_box(amount),
                        black_box(reserve_in),
                        black_box(reserve_out),
                        black_box(fee_bps),
                    )
                });
            },
        );
    }

    group.finish();
}

fn benchmark_pool_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("pool_operations");

    let pool = dex::RaydiumPool::new(
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        1_000_000_000,
        50_000_000_000,
    );

    group.bench_function("calculate_output", |b| {
        b.iter(|| pool.calculate_output(black_box(1_000_000), black_box(true)));
    });

    group.bench_function("calculate_price_impact", |b| {
        b.iter(|| pool.calculate_price_impact(black_box(1_000_000), black_box(true)));
    });

    group.bench_function("has_sufficient_liquidity", |b| {
        b.iter(|| pool.has_sufficient_liquidity(black_box(1_000_000), black_box(true)));
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_single_pool_routing,
    benchmark_split_routing,
    benchmark_multi_hop_routing,
    benchmark_calculator,
    benchmark_pool_operations
);
criterion_main!(benches);
