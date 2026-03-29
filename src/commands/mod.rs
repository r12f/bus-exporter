pub mod install;
pub mod pull;
pub mod run;
pub mod trace;

use anyhow::Result;
use regex::Regex;

use crate::config;

/// Filter collectors by name regex and metrics by metric regex.
/// Returns a new list of collectors with only matching metrics.
pub fn filter_collectors(
    collectors: &[config::CollectorConfig],
    collector_filter: Option<&str>,
    metric_filter: Option<&str>,
) -> Result<Vec<config::CollectorConfig>> {
    let collector_re = collector_filter
        .map(Regex::new)
        .transpose()
        .map_err(|e| anyhow::anyhow!("invalid --collector regex: {e}"))?;
    let metric_re = metric_filter
        .map(Regex::new)
        .transpose()
        .map_err(|e| anyhow::anyhow!("invalid --metric regex: {e}"))?;

    let mut filtered: Vec<config::CollectorConfig> = Vec::new();
    for c in collectors {
        if let Some(ref re) = collector_re {
            if !re.is_match(&c.name) {
                continue;
            }
        }
        let mut cc = c.clone();
        if let Some(ref re) = metric_re {
            cc.metrics.retain(|m| re.is_match(&m.name));
        }
        if !cc.metrics.is_empty() {
            filtered.push(cc);
        }
    }
    Ok(filtered)
}
