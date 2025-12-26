//! Configuration management

use crate::error::{Result, RouterError};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Command-line arguments
#[derive(Parser, Debug, Clone)]
#[command(name = "router-bot")]
#[command(about = "Solana Liquidity Router Bot", long_about = None)]
pub struct CliArgs {
    /// Solana RPC URL
    #[arg(short, long)]
    pub rpc_url: Option<String>,

    /// Network (devnet, mainnet-beta, or custom RPC)
    #[arg(short, long, default_value = "devnet")]
    pub network: String,

    /// Input token mint address
    #[arg(long)]
    pub token_in: Option<String>,

    /// Output token mint address
    #[arg(long)]
    pub token_out: Option<String>,

    /// Amount to swap (in token decimals)
    #[arg(long)]
    pub amount: Option<u64>,

    /// Routing strategy (single, split, multihop, or all)
    #[arg(long, default_value = "all")]
    pub strategy: String,

    /// Maximum number of hops for multi-hop routing
    #[arg(long, default_value = "2")]
    pub max_hops: usize,

    /// Dry run mode (don't execute, just show routes)
    #[arg(long, default_value = "true")]
    pub dry_run: bool,

    /// Config file path
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Verbose logging
    #[arg(short, long)]
    pub verbose: bool,
}

/// Configuration file format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    pub network: Option<NetworkConfig>,
    pub routing: Option<RoutingConfig>,
    pub execution: Option<ExecutionConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub rpc_url: Option<String>,
    pub network: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    pub max_hops: Option<usize>,
    pub default_strategy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    pub dry_run: Option<bool>,
    pub slippage_bps: Option<u16>,
}

/// Final configuration combining CLI args, config file, and defaults
#[derive(Debug, Clone)]
pub struct Config {
    pub rpc_url: String,
    pub network: String,
    pub max_hops: usize,
    pub strategy: String,
    pub dry_run: bool,
    pub slippage_bps: u16,
    pub verbose: bool,
}

impl Config {
    /// Create config from CLI args
    pub fn from_args(args: CliArgs) -> Result<Self> {
        // Load config file if specified
        let config_file = if let Some(config_path) = &args.config {
            Self::load_config_file(config_path)?
        } else {
            ConfigFile {
                network: None,
                routing: None,
                execution: None,
            }
        };

        // Determine RPC URL (priority: CLI > env > config file > default)
        let rpc_url = args
            .rpc_url
            .or_else(|| {
                config_file
                    .network
                    .as_ref()
                    .and_then(|n| n.rpc_url.clone())
            })
            .unwrap_or_else(|| Self::default_rpc_url(&args.network));

        // Determine max hops
        let max_hops = config_file
            .routing
            .as_ref()
            .and_then(|r| r.max_hops)
            .unwrap_or(args.max_hops);

        // Determine strategy
        let strategy = config_file
            .routing
            .as_ref()
            .and_then(|r| r.default_strategy.clone())
            .unwrap_or_else(|| args.strategy.clone());

        // Determine dry run mode
        let dry_run = config_file
            .execution
            .as_ref()
            .and_then(|e| e.dry_run)
            .unwrap_or(args.dry_run);

        // Determine slippage
        let slippage_bps = config_file
            .execution
            .as_ref()
            .and_then(|e| e.slippage_bps)
            .unwrap_or(100); // Default 1%

        // Validate max_hops
        if max_hops == 0 || max_hops > 3 {
            return Err(RouterError::ConfigError(
                "max_hops must be between 1 and 3".to_string(),
            ));
        }

        Ok(Self {
            rpc_url,
            network: args.network,
            max_hops,
            strategy,
            dry_run,
            slippage_bps,
            verbose: args.verbose,
        })
    }

    /// Load config file from path
    fn load_config_file(path: &PathBuf) -> Result<ConfigFile> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| RouterError::ConfigError(format!("Failed to read config file: {}", e)))?;

        toml::from_str(&contents)
            .map_err(|e| RouterError::ConfigError(format!("Failed to parse config file: {}", e)))
    }

    /// Get default RPC URL for network
    fn default_rpc_url(network: &str) -> String {
        match network {
            "devnet" => "https://api.devnet.solana.com",
            "mainnet-beta" | "mainnet" => "https://api.mainnet-beta.solana.com",
            "testnet" => "https://api.testnet.solana.com",
            custom => custom, // Assume it's a custom RPC URL
        }
        .to_string()
    }

    /// Create default config for testing
    pub fn default_devnet() -> Self {
        Self {
            rpc_url: "https://api.devnet.solana.com".to_string(),
            network: "devnet".to_string(),
            max_hops: 2,
            strategy: "all".to_string(),
            dry_run: true,
            slippage_bps: 100,
            verbose: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default_devnet();
        assert_eq!(config.network, "devnet");
        assert_eq!(config.max_hops, 2);
        assert!(config.dry_run);
        assert_eq!(config.slippage_bps, 100);
    }

    #[test]
    fn test_config_from_args() {
        let args = CliArgs {
            rpc_url: Some("https://custom.rpc.com".to_string()),
            network: "mainnet".to_string(),
            token_in: None,
            token_out: None,
            amount: None,
            strategy: "single".to_string(),
            max_hops: 3,
            dry_run: false,
            config: None,
            verbose: true,
        };

        let config = Config::from_args(args).unwrap();
        assert_eq!(config.rpc_url, "https://custom.rpc.com");
        assert_eq!(config.strategy, "single");
        assert_eq!(config.max_hops, 3);
        assert!(!config.dry_run);
        assert!(config.verbose);
    }

    #[test]
    fn test_invalid_max_hops() {
        let args = CliArgs {
            rpc_url: None,
            network: "devnet".to_string(),
            token_in: None,
            token_out: None,
            amount: None,
            strategy: "all".to_string(),
            max_hops: 0, // Invalid!
            dry_run: true,
            config: None,
            verbose: false,
        };

        let result = Config::from_args(args);
        assert!(result.is_err());
    }

    #[test]
    fn test_default_rpc_urls() {
        assert_eq!(
            Config::default_rpc_url("devnet"),
            "https://api.devnet.solana.com"
        );
        assert_eq!(
            Config::default_rpc_url("mainnet-beta"),
            "https://api.mainnet-beta.solana.com"
        );
    }
}
