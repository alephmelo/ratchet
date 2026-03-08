mod config;
mod generate;
mod results;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ratchet")]
#[command(about = "Generate AI agent instructions for autonomous code optimization loops")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate program.md from ratchet.yaml
    Init {
        /// Path to config file
        #[arg(short, long, default_value = "ratchet.yaml")]
        config: PathBuf,

        /// Output file path
        #[arg(short, long, default_value = "program.md")]
        output: PathBuf,
    },

    /// Validate ratchet.yaml without generating
    Check {
        /// Path to config file
        #[arg(short, long, default_value = "ratchet.yaml")]
        config: PathBuf,
    },

    /// Show experiment results from results.tsv
    Results {
        /// Path to config file
        #[arg(short, long, default_value = "ratchet.yaml")]
        config: PathBuf,

        /// Path to results file
        #[arg(short, long, default_value = "results.tsv")]
        results: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { config, output } => {
            let cfg = config::Config::from_file(&config)
                .with_context(|| format!("loading config from {}", config.display()))?;
            println!("Config OK: {}", cfg.name);

            generate::write_program(&cfg, &output)?;
            println!("Generated: {}", output.display());
        }
        Commands::Check { config } => {
            let cfg = config::Config::from_file(&config)
                .with_context(|| format!("loading config from {}", config.display()))?;
            println!("Config OK: {}", cfg.name);
            println!("  editable:    {:?}", cfg.editable);
            println!("  readonly:    {:?}", cfg.readonly);
            println!("  run:         {}", cfg.run);
            println!(
                "  metric:      {} ({})",
                cfg.metric.name,
                match cfg.metric.direction {
                    config::Direction::Maximize => "maximize",
                    config::Direction::Minimize => "minimize",
                }
            );
            println!("  constraints: {}", cfg.constraints.len());
            println!("  timeout:     {}s", cfg.timeout);
        }
        Commands::Results {
            config,
            results: tsv_path,
        } => {
            let cfg = config::Config::from_file(&config)
                .with_context(|| format!("loading config from {}", config.display()))?;
            results::show_results(&cfg, &tsv_path)?;
        }
    }

    Ok(())
}
