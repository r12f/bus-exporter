# CLI Subcommands

## Overview

`bus-exporter` supports five modes of operation via subcommands:

- **`run`** (default) — Start as a daemon, continuously polling collectors and exporting metrics.
- **`pull`** — Single-shot read: connect, read metrics once, print JSON, exit.
- **`watch`** — Continuous metric tracing: pull in a loop, print NDJSON to stdout.
- **`show-config`** — Display the fully resolved configuration (merged metrics_files, with optional filters).
- **`install`** — Install bus-exporter as a system service (systemd).

When no subcommand is given, `run` is assumed (backward-compatible).

## Global Options

```
bus-exporter [OPTIONS] <COMMAND>

Options:
  -c, --config <PATH>    Path to configuration file
  -h, --help             Print help
  -V, --version          Print version
```

## `pull` Subcommand

Single-shot metric read. Connects to devices, reads once, prints JSON to stdout, exits.

```
bus-exporter pull [OPTIONS]

Options:
  --collector <REGEX>    Filter collectors by name (regex, partial match)
  --metric <REGEX>       Filter metrics by name (regex, partial match)
```

### Behavior

1. Load config file (same search path as `run`).
2. Filter collectors: if `--collector` is set, keep only collectors whose name matches the regex.
3. For matching collectors, filter metrics: if `--metric` is set, keep only metrics whose name matches the regex.
4. If no collectors or metrics remain after filtering, exit with error.
5. For each matching collector:
   a. Create reader via `MetricReaderFactory`.
   b. Call `set_metrics()` with filtered metrics.
   c. Call `connect()`.
   d. Call `read()` once.
   e. Call `disconnect()`.
6. Print JSON to stdout.
7. Exit code 0 if all reads succeed, 2 if any read fails.

### Regex Behavior

- Uses the `regex` crate.
- Partial match (contains semantics) — pattern `volt` matches `voltage_l1`.
- Case-sensitive by default. User can use `(?i)` prefix for case-insensitive.
- Invalid regex → exit with error message immediately.

### JSON Output Format

```json
{
  "collectors": [
    {
      "name": "sdm630",
      "protocol": "modbus-tcp",
      "metrics": [
        {
          "name": "voltage_l1",
          "value": 230.5,
          "raw_value": 2305,
          "error": null
        },
        {
          "name": "voltage_l2",
          "value": null,
          "raw_value": null,
          "error": "connection refused"
        }
      ]
    }
  ],
  "summary": {
    "total_collectors": 1,
    "total_metrics": 2,
    "successful": 1,
    "failed": 1
  }
}
```

Field definitions:
- `name` — Metric name from config.
- `value` — Scaled value (`raw * scale + offset`), `null` on error.
- `raw_value` — Raw value before scale/offset, `null` on error.
- `error` — Error message string, `null` on success.
- `summary` — Aggregated counts for quick status check.

### Logging

- Logs go to stderr (same logging config as `run`).
- JSON output goes to stdout only.
- This allows `bus-exporter pull 2>/dev/null` for clean JSON.

## `watch` Subcommand

Continuous metric tracing. Pulls metrics in a loop, printing NDJSON to stdout.

```
bus-exporter watch [OPTIONS]

Options:
  --collector <REGEX>    Filter collectors by name (regex, partial match)
  --metric <REGEX>       Filter metrics by name (regex, partial match)
  --interval <DURATION>  Override polling interval (e.g. "1s", "500ms")
```

### Behavior

1. Load config file (same search path as `run`).
2. Filter collectors and metrics (same logic as `pull`).
3. Loop:
   a. For each matching collector: connect → read → print JSON → disconnect.
   b. Sleep for `--interval` (or collector's `polling_interval` from config).
   c. Repeat until Ctrl+C (SIGINT/SIGTERM) for graceful stop.

### Output

NDJSON (one JSON object per line per iteration) to stdout. Each object has the same collector/metric JSON structure as `pull`, plus:

- `timestamp` — RFC 3339 timestamp of the iteration start.
- `iteration` — 1-based counter.

On exit, a `watch_summary` is printed to stderr with total iterations, successes, and failures.

### Logging

- Logs go to stderr (same logging config as `run`).
- NDJSON output goes to stdout only.
- `watch_summary` goes to stderr.

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All iterations completed successfully |
| 2 | Any iteration had failures |

## `install` Subcommand

Install bus-exporter as a systemd service.

```
bus-exporter install [OPTIONS]

Options:
  --user              Install as user service (systemctl --user) instead of system service
  --config <PATH>     Config file path to embed in service file (default: /etc/bus-exporter/config.yaml)
  --bin <PATH>        Path to bus-exporter binary (default: auto-detect from current executable)
  --uninstall         Remove the service instead of installing
```

### Behavior

1. Generate a systemd unit file from template.
2. Write to `/etc/systemd/system/bus-exporter.service` (system) or `~/.config/systemd/user/bus-exporter.service` (user).
3. Run `systemctl daemon-reload`.
4. Run `systemctl enable bus-exporter`.
5. Print instructions to start: `systemctl start bus-exporter`.

### Systemd Unit Template

```ini
[Unit]
Description=Bus Exporter - Industrial bus metrics exporter
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart={bin_path} run -c {config_path}
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=bus-exporter

[Install]
WantedBy=multi-user.target
```

### `--uninstall` Behavior

1. Run `systemctl stop bus-exporter` (ignore if not running).
2. Run `systemctl disable bus-exporter`.
3. Remove the unit file.
4. Run `systemctl daemon-reload`.

### Platform Check

- If not on Linux or systemd is not available, print error and exit.
- Future: support other init systems (OpenRC, launchd) via `--type` flag.

## `show-config` Subcommand

Display the fully resolved configuration after merging all `metrics_files` and inline `metrics`. Useful for debugging config issues when metrics are split across multiple files.

```
bus-exporter show-config [OPTIONS]

Options:
  --collector <REGEX>    Filter collectors by name (regex, partial match)
  --metric <REGEX>       Filter metrics by name (regex, partial match)
  --format <FORMAT>      Output format: yaml (default) or json
```

### Behavior

1. Load config file (same search path as `run`).
2. Resolve `metrics_files` — merge into inline `metrics` per collector. If a `metrics_files` path does not exist or is unreadable, print an error to stderr and exit 1.
3. Apply filters (same regex semantics as `pull`/`watch` — reuse shared `filter_collectors()`):
   - `--collector <REGEX>` — keep only matching collectors.
   - `--metric <REGEX>` — keep only matching metrics within displayed collectors. Collectors with zero matching metrics are removed.
   - Invalid regex → print error to stderr and exit 1 (same as `pull`/`watch`).
4. If filters match nothing, print warning to stderr: `"warning: no collectors matched the filter"` and output the full config with `collectors: []`. **Note:** This differs from `pull`/`watch` which exit with error code 1 when filters match nothing. `show-config` exits 0 because it is a diagnostic tool — showing an empty result is still a valid answer (e.g., confirming a collector name doesn't exist).
5. Serialize the **full** resolved `Config` to stdout (including `logging`, `exporters`, `global_labels`, and filtered `collectors`).
6. `metrics_files` field is omitted from output (already resolved into `metrics`). Use `#[serde(skip_serializing)]` on the `metrics_files` field.

### Validation

- Validation is **relaxed** compared to `run`. The command parses and resolves the config but skips exporter-specific validation (e.g., missing exporter endpoints, no exporters configured). This allows users to inspect partially complete or work-in-progress configs.
- If the config file cannot be found, is not valid YAML, or has missing required fields (e.g., `collectors` not present), print errors to stderr and exit 1.
- Use a dedicated `Config::load_for_display()` or reuse `Config::load_for_pull()` which already skips exporter validation.

### Output Formats

- **YAML** (default, `--format yaml`) — serialized via a YAML serializer. Output ends with a trailing newline.
- **JSON** (`--format json`) — serialized via `serde_json::to_string_pretty`. Output ends with a trailing newline.

Both formats must produce consistent trailing newline behavior. Use `println!` for both paths to ensure this.

### `OutputFormat` Enum

```rust
#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Yaml,
    Json,
}
```

### Credential Redaction

Sensitive fields are **redacted** in the output using a custom serializer that emits `"***"` instead of the actual value. This approach is preferred over `#[serde(skip_serializing)]` because it makes the presence of the credential field visible (users can see that auth *is* configured, just redacted).

Fields that must be redacted:
- `exporters.mqtt.auth.password`

**General rule:** Any field containing passwords, tokens, or secrets must use the same `serialize_redacted` custom serializer. When adding new credential fields in the future, apply `#[serde(serialize_with = "serialize_redacted")]`.

Users running `show-config` may pipe output to logs, paste in issues, or share with teammates — plaintext credentials must never appear.

### Output Example

```yaml
logging:
  level: "info"
  output: "syslog"
  syslog_facility: "daemon"

global_labels:
  host: "r12f-pi5"

exporters:
  prometheus:
    enabled: true
    listen: "0.0.0.0:9091"
  mqtt:
    enabled: true
    broker: "mqtt://localhost:1883"
    auth:
      username: "exporter"
      password: "***"

collectors:
  - name: "bme680"
    protocol:
      type: i2c
      bus: "/dev/i2c-1"
      address: 0x76
    polling_interval: "5s"
    init_writes:
      - address: 0x72
        value: 0x01
    pre_poll:
      - address: 0x74
        value: 0x25
      - delay: "50ms"
    metrics:
      - name: temperature
        type: gauge
        address: 0x22
        data_type: u16
        byte_order: big_endian
        scale: 0.01
        offset: -40.0
        unit: "°C"
```

### Logging

- Warnings and errors go to stderr.
- Resolved config goes to stdout only.
- This allows `bus-exporter show-config 2>/dev/null` for clean output.

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Config parsed and displayed successfully (even if filters matched nothing) |
| 1 | Config file not found, unparseable YAML, unresolvable `metrics_files` paths, or invalid regex |

## `run` Subcommand

Default behavior — start as daemon. No changes from current behavior.

```
bus-exporter run [OPTIONS]
```

This is the implicit default when no subcommand is given.

## Implementation Notes

### CLI Structure (clap)

```rust
#[derive(Parser)]
#[command(name = "bus-exporter", version, about)]
struct Cli {
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Start as daemon (default)
    Run,
    /// Single-shot metric read
    Pull {
        /// Filter collectors by name (regex)
        #[arg(long)]
        collector: Option<String>,
        /// Filter metrics by name (regex)
        #[arg(long)]
        metric: Option<String>,
    },
    /// Continuous metric watch (NDJSON loop)
    Watch {
        /// Filter collectors by name (regex)
        #[arg(long)]
        collector: Option<String>,
        /// Filter metrics by name (regex)
        #[arg(long)]
        metric: Option<String>,
        /// Override polling interval (e.g. "1s", "500ms")
        #[arg(long)]
        interval: Option<String>,
    },
    /// Display resolved configuration
    ShowConfig {
        /// Filter collectors by name (regex, partial match)
        #[arg(long)]
        collector: Option<String>,
        /// Filter metrics by name (regex, partial match)
        #[arg(long)]
        metric: Option<String>,
        /// Output format: yaml or json
        #[arg(long, default_value = "yaml")]
        format: OutputFormat,
    },
    /// Install as system service
    Install {
        /// Install as user service
        #[arg(long)]
        user: bool,
        /// Config file path for service
        #[arg(long)]
        config: Option<PathBuf>,
        /// Binary path for service
        #[arg(long)]
        bin: Option<PathBuf>,
        /// Remove service instead of installing
        #[arg(long)]
        uninstall: bool,
    },
}
```

### Dependencies

- `regex` crate (add to Cargo.toml)
- `serde_json` for pull output (already a transitive dep, add as direct)
- `humantime` for parsing duration strings in watch `--interval`

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success (all reads OK for pull) |
| 1 | Fatal error (bad config, bad regex, no matches, no systemd, platform errors) |
| 2 | Partial failure (some/all metric reads failed for pull) |
