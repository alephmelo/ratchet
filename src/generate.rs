use anyhow::{Context, Result};
use minijinja::Environment;
use std::path::Path;

use crate::config::{Config, Direction};

const TEMPLATE: &str = include_str!("../templates/program.md.j2");

pub fn render(config: &Config) -> Result<String> {
    let mut env = Environment::new();
    env.add_template("program.md", TEMPLATE)
        .context("loading template")?;

    let tmpl = env.get_template("program.md").unwrap();

    let first_metric = config.first_metric();

    let ctx = minijinja::context! {
        name => config.name,
        editable => config.editable,
        readonly => config.readonly,
        run_command => config.run,
        metric_name => first_metric.name,
        metric_grep => first_metric.grep,
        direction => match first_metric.direction {
            Direction::Maximize => "maximize",
            Direction::Minimize => "minimize",
        },
        direction_word => match first_metric.direction {
            Direction::Maximize => "higher",
            Direction::Minimize => "lower",
        },
        direction_worse => match first_metric.direction {
            Direction::Maximize => "lower",
            Direction::Minimize => "higher",
        },
        constraints => config.constraints.iter().map(|c| {
            minijinja::context! {
                name => c.name,
                grep => c.grep,
                warn_above => c.warn_above,
                warn_below => c.warn_below,
                fail_above => c.fail_above,
                fail_below => c.fail_below,
            }
        }).collect::<Vec<_>>(),
        has_constraints => !config.constraints.is_empty(),
        grep_pattern => config.grep_pattern(),
        tsv_columns => config.tsv_columns(),
        tsv_header => config.tsv_columns().join("\t"),
        timeout => config.timeout,
        timeout_kill => config.timeout * 2,
        context => config.context,
        baseline => config.baseline,
        has_baseline => config.baseline.is_some(),
        is_multi_metric => config.is_multi_metric(),
        all_metrics => config.primary_metrics().iter().map(|m| {
            minijinja::context! {
                name => m.name,
                grep => m.grep,
                direction => match m.direction {
                    Direction::Maximize => "maximize",
                    Direction::Minimize => "minimize",
                },
            }
        }).collect::<Vec<_>>(),
    };

    let rendered = tmpl.render(&ctx).context("rendering template")?;
    Ok(rendered)
}

pub fn write_program(config: &Config, output: &Path) -> Result<()> {
    let content = render(config)?;
    std::fs::write(output, &content).with_context(|| format!("writing {}", output.display()))?;
    Ok(())
}
