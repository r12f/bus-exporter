use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde_json::json;
use std::path::Path;
use std::time::Duration;
use tokio::select;
use tokio_util::sync::CancellationToken;

use crate::config::{find_config_file, CollectorConfig, Config};
use crate::logging::{init_logging, map_logging_config, LogOutput, LoggingConfig};
use crate::reader::{MetricReaderFactory, MetricReaderFactoryImpl};

use super::filter_collectors;

/// Entry point for the `trace` subcommand.
pub async fn trace_command(
    cli_config: Option<&Path>,
    collector: Option<&str>,
    metric: Option<&str>,
    interval: Option<&str>,
) -> Result<()> {
    let config_path = find_config_file(cli_config).context("failed to find configuration file");
    let config_path = match config_path {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Fatal: {e:#}");
            std::process::exit(1);
        }
    };
    let config = match Config::load_for_pull(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Fatal: failed to load configuration: {e:#}");
            std::process::exit(1);
        }
    };
    let logging_cfg = map_logging_config(&config.logging);
    let trace_logging = LoggingConfig {
        level: logging_cfg.level,
        output: LogOutput::Stderr,
    };
    init_logging(&trace_logging).context("failed to initialize logging")?;

    let mut filtered_collectors = filter_collectors(&config.collectors, collector, metric)?;
    if filtered_collectors.is_empty() {
        eprintln!("Fatal: no collectors/metrics match the given filters");
        std::process::exit(1);
    }

    // Parse and apply interval override
    let interval_override = match interval {
        Some(s) => {
            let dur: Duration = s
                .parse::<humantime::Duration>()
                .map_err(|e| anyhow::anyhow!("invalid --interval '{}': {}", s, e))?
                .into();
            if dur.is_zero() {
                bail!("--interval must be > 0");
            }
            Some(dur)
        }
        None => None,
    };

    if let Some(dur) = interval_override {
        for c in &mut filtered_collectors {
            c.polling_interval = dur;
        }
    }

    // Use the smallest polling_interval among filtered collectors as the loop interval
    let loop_interval = filtered_collectors
        .iter()
        .map(|c| c.polling_interval)
        .min()
        .unwrap_or(Duration::from_secs(10));

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        cancel_clone.cancel();
    });

    let exit_code = run_trace(&filtered_collectors, loop_interval, &cancel).await;
    std::process::exit(exit_code);
}

async fn run_trace(
    collectors: &[CollectorConfig],
    interval: Duration,
    cancel: &CancellationToken,
) -> i32 {
    let factory = MetricReaderFactoryImpl;
    let mut iteration: u64 = 0;
    let mut total_successful: usize = 0;
    let mut total_failed: usize = 0;

    loop {
        if cancel.is_cancelled() {
            break;
        }

        iteration += 1;
        let timestamp = Utc::now().to_rfc3339();
        let inner_cancel = CancellationToken::new();

        let mut total_metrics: usize = 0;
        let mut successful: usize = 0;
        let mut failed: usize = 0;
        let mut collectors_json = Vec::new();

        for collector in collectors {
            let mut reader = match factory.create(collector) {
                Ok(r) => r,
                Err(e) => {
                    let mut metrics_json = Vec::new();
                    for metric_cfg in &collector.metrics {
                        total_metrics += 1;
                        failed += 1;
                        metrics_json.push(json!({
                            "name": metric_cfg.name,
                            "value": null,
                            "raw_value": null,
                            "error": format!("collector create failed: {e}")
                        }));
                    }
                    collectors_json.push(json!({
                        "name": collector.name,
                        "protocol": collector.protocol.to_string(),
                        "metrics": metrics_json
                    }));
                    continue;
                }
            };
            reader.set_metrics(collector.metrics.clone());
            if let Err(e) = reader.connect().await {
                let mut metrics_json = Vec::new();
                for metric_cfg in &collector.metrics {
                    total_metrics += 1;
                    failed += 1;
                    metrics_json.push(json!({
                        "name": metric_cfg.name,
                        "value": null,
                        "raw_value": null,
                        "error": format!("connect failed: {e}")
                    }));
                }
                collectors_json.push(json!({
                    "name": collector.name,
                    "protocol": collector.protocol.to_string(),
                    "metrics": metrics_json
                }));
                continue;
            }
            let results = reader.read(&inner_cancel).await;
            let _ = reader.disconnect().await;

            let mut metrics_json = Vec::new();
            for metric_cfg in &collector.metrics {
                total_metrics += 1;
                match results.metrics.get(&metric_cfg.name) {
                    Some(Ok((raw_value, scaled_value))) => {
                        successful += 1;
                        metrics_json.push(json!({
                            "name": metric_cfg.name,
                            "value": scaled_value,
                            "raw_value": raw_value,
                            "error": null
                        }));
                    }
                    Some(Err(e)) => {
                        failed += 1;
                        metrics_json.push(json!({
                            "name": metric_cfg.name,
                            "value": null,
                            "raw_value": null,
                            "error": e.to_string()
                        }));
                    }
                    None => {
                        failed += 1;
                        metrics_json.push(json!({
                            "name": metric_cfg.name,
                            "value": null,
                            "raw_value": null,
                            "error": "metric not in results"
                        }));
                    }
                }
            }

            collectors_json.push(json!({
                "name": collector.name,
                "protocol": collector.protocol.to_string(),
                "metrics": metrics_json
            }));
        }

        total_successful += successful;
        total_failed += failed;

        let output = json!({
            "timestamp": timestamp,
            "iteration": iteration,
            "collectors": collectors_json,
            "summary": {
                "total_collectors": collectors.len(),
                "total_metrics": total_metrics,
                "successful": successful,
                "failed": failed
            }
        });

        if let Ok(s) = serde_json::to_string(&output) {
            println!("{}", s);
        }

        // Sleep, but break early on cancellation
        select! {
            _ = cancel.cancelled() => { break; }
            _ = tokio::time::sleep(interval) => {}
        }
    }

    // Final summary
    let summary = json!({
        "trace_summary": {
            "total_iterations": iteration,
            "total_successful": total_successful,
            "total_failed": total_failed
        }
    });
    eprintln!(
        "{}",
        serde_json::to_string_pretty(&summary).unwrap_or_default()
    );

    if total_failed > 0 {
        2
    } else {
        0
    }
}
