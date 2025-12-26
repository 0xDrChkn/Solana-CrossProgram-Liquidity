//! Solana Liquidity Router Bot
//!
//! This library provides functionality for finding optimal swap routes across
//! multiple Solana DEXes including Raydium, Orca, Meteora, and Phoenix.

pub mod client;
pub mod types;
pub mod dex;
pub mod calculator;
pub mod router;
pub mod executor;
pub mod config;
pub mod error;

// Re-export commonly used types
pub use client::SolanaClient;
pub use config::Config;
pub use error::{RouterError, Result};
pub use types::{Pool, Route, SwapQuote};
