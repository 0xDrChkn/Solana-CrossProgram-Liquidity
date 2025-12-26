# Solana Liquidity Router Bot 🤖

A comprehensive Solana liquidity router that finds optimal swap routes across multiple DEXes including Raydium, Orca, Meteora, and Phoenix.

## Features ✨

- **Multi-DEX Support**: Raydium, Orca, Meteora, and Phoenix
- **Advanced Routing Strategies**:
  - Single Pool: Find the best single pool for a swap
  - Split Routing: Optimize by splitting amounts across multiple pools
  - Multi-hop Routing: Route through intermediate tokens (A→B→C)
- **Comprehensive Testing**: 61 unit tests + integration tests + property-based tests
- **Dry-run Mode**: Simulate swaps without executing
- **CLI Interface**: Easy-to-use command-line interface
- **Benchmarks**: Performance testing with Criterion

## Quick Start 🚀

### Demo Mode

Run the router in demo mode to see it compare routing strategies:

```bash
cargo run
```

Output:
```
🚀 Solana Liquidity Router Bot
📡 Connecting to devnet
✅ Connected to Solana (version: 3.0.6)
🎯 Running in demo mode

📊 Example: Swapping 1000000000 units of Token A for Token B
   Created 4 example pools

🔍 Comparing routing strategies:

   1️⃣  Single Pool:
      Output: 33266599933
      DEX: Orca

   2️⃣  Split Routing:
      Output: 42094261529
      Pools used: 4

   3️⃣  Multi-hop Routing:
      Output: 33266599933
      Hops: 1

✅ Demo complete!
```

### Actual Swaps

To swap actual tokens (dry-run mode by default):

```bash
cargo run -- \
  --token-in <TOKEN_A_MINT> \
  --token-out <TOKEN_B_MINT> \
  --amount 1000000 \
  --strategy all \
  --network devnet
```

## Testing 🧪

### Run Unit Tests

```bash
cargo test --lib
```

Results: **61 tests passed** ✅

### Run Integration Tests

Integration tests connect to devnet (run with `--ignored`):

```bash
cargo test -- --ignored --test-threads=1
```

### Run Benchmarks

```bash
cargo bench
```

Benchmarks include:
- Single pool routing (2, 5, 10, 20 pools)
- Split routing optimization
- Multi-hop path finding
- Calculator performance

## CLI Options 📋

```
Options:
  -r, --rpc-url <RPC_URL>      Solana RPC URL
  -n, --network <NETWORK>      Network (devnet, mainnet-beta, or custom RPC) [default: devnet]
      --token-in <TOKEN_IN>    Input token mint address
      --token-out <TOKEN_OUT>  Output token mint address
      --amount <AMOUNT>        Amount to swap (in token decimals)
      --strategy <STRATEGY>    Routing strategy (single, split, multihop, or all) [default: all]
      --max-hops <MAX_HOPS>    Maximum number of hops for multi-hop routing [default: 2]
      --dry-run                Dry run mode (don't execute, just show routes)
  -c, --config <CONFIG>        Config file path
  -v, --verbose                Verbose logging
  -h, --help                   Print help
```

## Architecture 🏗️

### Project Structure

```
router-bot/
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library exports
│   ├── client.rs            # RPC client wrapper
│   ├── calculator.rs        # AMM calculations (constant product formula)
│   ├── error.rs             # Error types
│   ├── config.rs            # Configuration management
│   ├── executor.rs          # Transaction building & execution
│   ├── types/
│   │   ├── pool.rs          # Pool trait & common types
│   │   └── route.rs         # Route & swap quote types
│   ├── dex/
│   │   ├── raydium.rs       # Raydium pool implementation
│   │   ├── orca.rs          # Orca pool implementation
│   │   ├── meteora.rs       # Meteora pool implementation
│   │   └── phoenix.rs       # Phoenix orderbook implementation
│   └── router/
│       ├── single.rs        # Best single pool routing
│       ├── split.rs         # Split routing optimizer
│       └── multihop.rs      # Multi-hop routing (BFS)
├── tests/
│   └── integration_test.rs  # Integration tests
├── benches/
│   └── routing_benchmark.rs # Performance benchmarks
└── Cargo.toml
```

### Key Components

#### 1. Calculator (`calculator.rs`)
Implements constant product AMM formula (x * y = k):
- `calculate_amount_out()`: Calculate output given input
- `calculate_amount_in()`: Calculate required input for desired output
- `calculate_price_impact()`: Calculate price impact in basis points

#### 2. Pool Trait (`types/pool.rs`)
Common interface for all DEX pools:
```rust
pub trait Pool {
    fn calculate_output(&self, input_amount: u64, a_to_b: bool) -> Result<(u64, u16)>;
    fn has_sufficient_liquidity(&self, input_amount: u64, a_to_b: bool) -> bool;
    // ... more methods
}
```

#### 3. Routing Strategies

**Single Pool Router** (`router/single.rs`)
- Finds the pool with the best output
- O(n) complexity where n = number of pools
- Best for simple swaps

**Split Router** (`router/split.rs`)
- Splits amount across multiple pools
- Tests different allocations (0%, 10%, 20%, ..., 100%)
- Minimizes price impact for large swaps
- Best for large trades

**Multi-hop Router** (`router/multihop.rs`)
- Uses BFS to find all paths up to max_hops
- Evaluates each path to find best route
- Avoids cycles
- Best when no direct pool exists

## Test Coverage 📊

### Unit Tests (61 passing)

- **Calculator**: 11 tests
  - Basic calculations
  - Fee handling
  - Price impact
  - Reverse calculations (amount_in)
  - Edge cases
  - Property-based tests (proptest)

- **Pool Implementations**: 16 tests
  - Raydium (4 tests)
  - Orca (6 tests)
  - Meteora (2 tests)
  - Phoenix (4 tests)

- **Routing**: 15 tests
  - Single pool routing (5 tests)
  - Split routing (5 tests)
  - Multi-hop routing (5 tests)

- **Types**: 8 tests
  - Route construction
  - Quote comparison
  - Pool info

- **Config & Error**: 6 tests
- **Client**: 5 tests

### Integration Tests

- Devnet connection
- USDC mint fetching
- End-to-end routing
- Strategy comparison
- Executor dry-run

## Performance 🚀

Benchmark results (example):

```
single_pool_routing/2    time: [1.2 µs ... 1.5 µs]
single_pool_routing/10   time: [5.8 µs ... 6.2 µs]
split_routing/2          time: [15 µs ... 18 µs]
multi_hop_routing/2_hops time: [8.5 µs ... 9.2 µs]
calculator/amount_1000   time: [45 ns ... 52 ns]
```

## Future Enhancements 🔮

- [ ] Implement actual DEX account parsing (currently using test mocks)
- [ ] Add live transaction execution (currently dry-run only)
- [ ] Implement Jupiter aggregator integration
- [ ] Add support for concentrated liquidity (Orca Whirlpools)
- [ ] Implement MEV protection
- [ ] Add historical price tracking
- [ ] WebSocket support for real-time price updates
- [ ] REST API server mode

## Development 💻

### Run with verbose logging

```bash
cargo run -- --verbose
```

### Run specific test

```bash
cargo test test_split_routing -- --nocapture
```

### Check test coverage

```bash
cargo tarpaulin --out Html
```

## Dependencies 📦

Key dependencies:
- `solana-sdk` (3.0.0) - Solana blockchain SDK
- `solana-client` (3.1.1) - RPC client
- `spl-token` (9.0.0) - SPL Token program
- `anchor-client` (0.32.1) - Anchor program client
- `tokio` (1.48.0) - Async runtime
- `clap` (4.5) - CLI argument parsing
- `proptest` (1.4) - Property-based testing
- `criterion` (0.5) - Benchmarking

## License

MIT
