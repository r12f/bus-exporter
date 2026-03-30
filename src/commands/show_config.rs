use anyhow::Result;
use std::path::Path;

use crate::config::{self, OutputFormat};

pub fn show_config_command(
    config_path: Option<&Path>,
    collector_filter: Option<&str>,
    metric_filter: Option<&str>,
    format: &OutputFormat,
) -> Result<()> {
    let path = config::find_config_file(config_path)?;
    let mut cfg = config::Config::load_for_pull(&path)?;

    let filtered = super::filter_collectors(&cfg.collectors, collector_filter, metric_filter)?;

    if filtered.is_empty() && (collector_filter.is_some() || metric_filter.is_some()) {
        eprintln!("warning: no collectors matched the filter");
    }

    cfg.collectors = filtered;

    match format {
        OutputFormat::Yaml => {
            let output = serde_yaml::to_string(&cfg)?;
            println!("{}", output.trim_end());
        }
        OutputFormat::Json => {
            let output = serde_json::to_string_pretty(&cfg)?;
            println!("{}", output);
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "show_config_tests.rs"]
mod tests;
