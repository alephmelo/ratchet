mod bandit;
mod config;
mod diff;
mod generate;
mod instruct;
mod loop_cmd;
mod plot;
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

    /// Run the autonomous optimization loop
    Loop {
        /// Path to config file
        #[arg(short, long, default_value = "ratchet.yaml")]
        config: PathBuf,

        /// Agent command (overrides config). Use {prompt} for prompt file path.
        #[arg(short, long)]
        agent: Option<String>,

        /// Path to results file
        #[arg(short, long, default_value = "results.tsv")]
        results: PathBuf,

        /// Maximum number of iterations (default: unlimited)
        #[arg(short = 'n', long)]
        max: Option<usize>,

        /// Stop after N consecutive iterations without improvement
        #[arg(short, long)]
        patience: Option<usize>,
    },

    /// Plot metric progression from results.tsv
    Plot {
        /// Path to config file
        #[arg(short, long, default_value = "ratchet.yaml")]
        config: PathBuf,

        /// Path to results file
        #[arg(short, long, default_value = "results.tsv")]
        results: PathBuf,
    },

    /// Print setup instructions for an AI agent to help configure ratchet
    Instruct {
        /// Path to config file (used for context if it exists)
        #[arg(short, long, default_value = "ratchet.yaml")]
        config: PathBuf,
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
            for m in cfg.primary_metrics() {
                println!(
                    "  metric:      {} ({})",
                    m.name,
                    match m.direction {
                        config::Direction::Maximize => "maximize",
                        config::Direction::Minimize => "minimize",
                    }
                );
            }
            if cfg.is_multi_metric() {
                println!("  mode:        multi-metric (Pareto)");
            }
            println!("  constraints: {}", cfg.constraints.len());
            println!("  timeout:     {}s", cfg.timeout);
            if let Some(agent) = &cfg.agent {
                println!("  agent:       {}", agent);
                println!("  agent_tout:  {}s", cfg.agent_timeout);
            }
            if let Some(max) = cfg.max_iterations {
                println!("  max_iter:    {}", max);
            }
            if let Some(p) = cfg.patience {
                println!("  patience:    {}", p);
            }
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
        Commands::Loop {
            config,
            agent,
            results: tsv_path,
            max,
            patience,
        } => {
            let cfg = config::Config::from_file(&config)
                .with_context(|| format!("loading config from {}", config.display()))?;

            // Determine agent command: CLI flag > config file > error
            let agent_cmd = match agent {
                Some(a) => a,
                None => match &cfg.agent {
                    Some(a) => a.clone(),
                    None => {
                        anyhow::bail!(
                            "no agent specified. Use --agent or set 'agent' in {}",
                            config.display()
                        );
                    }
                },
            };

            // CLI flags override config values
            let max_iterations = max.or(cfg.max_iterations);
            let patience = patience.or(cfg.patience);

            loop_cmd::run_loop(&cfg, &agent_cmd, &tsv_path, max_iterations, patience)?;
        }
        Commands::Plot {
            config,
            results: tsv_path,
        } => {
            let cfg = config::Config::from_file(&config)
                .with_context(|| format!("loading config from {}", config.display()))?;
            plot::show_plot(&cfg, &tsv_path)?;
        }
        Commands::Instruct { config } => {
            instruct::print_instructions(&config);
        }
    }

    Ok(())
}
