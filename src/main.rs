mod config;
mod diff;
mod generate;
mod results;
mod run;

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

    /// Run the benchmark and display parsed metrics
    Run {
        /// Path to config file
        #[arg(short, long, default_value = "ratchet.yaml")]
        config: PathBuf,
    },

    /// Show diff of editable files
    Diff {
        /// Path to config file
        #[arg(short, long, default_value = "ratchet.yaml")]
        config: PathBuf,

        /// Show diff at a specific commit
        #[arg(long)]
        commit: Option<String>,

        /// Show diff at the best result from results.tsv
        #[arg(long)]
        best: bool,

        /// Path to results file (used with --best)
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
        Commands::Run { config } => {
            let cfg = config::Config::from_file(&config)
                .with_context(|| format!("loading config from {}", config.display()))?;
            run::run_benchmark(&cfg)?;
        }
        Commands::Diff {
            config,
            commit,
            best,
            results: tsv_path,
        } => {
            let cfg = config::Config::from_file(&config)
                .with_context(|| format!("loading config from {}", config.display()))?;
            diff::show_diff(&cfg, commit.as_deref(), best, &tsv_path)?;
        }
    }

    Ok(())
}
