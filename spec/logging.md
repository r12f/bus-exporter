# Logging Specification

## Overview

All logging uses the [`tracing`](https://docs.rs/tracing) crate ecosystem.

## Crate Stack

| Crate | Role |
|-------|------|
| `tracing` | Spans, events, `#[instrument]` macro |
| `tracing-subscriber` | Subscriber/layer composition |

> **Note on syslog:** Native syslog support is not currently implemented. The `output` config accepts `"json"` which produces **structured JSON output on stderr**, suitable for consumption by syslog daemons (e.g., via `journald` or piped to `logger`). Native syslog support is planned for a future release.

## Guidelines

### Prefer `#[instrument]` Over Manual Macros

Use the `#[instrument]` attribute on functions instead of manually creating spans or emitting events. This ensures consistent, structured spans with automatic argument capture.

```rust
#[instrument(skip(client), fields(collector = %name, slave_id))]
async fn poll_collector(name: &str, slave_id: u8, client: &mut ModbusClient) -> Result<()> { ... }

#[instrument(fields(metric = %metric.name, address = metric.address))]
fn decode_register(metric: &MetricConfig, raw: &[u16]) -> Result<f64> { ... }

#[instrument(skip(config))]
fn load_config(path: &Path, config: &Config) -> Result<Config> { ... }

#[instrument(fields(collector = %collector_name, endpoint = %endpoint))]
async fn connect_tcp(collector_name: &str, endpoint: &str) -> Result<TcpClient> { ... }
```

### Structured Context Fields

Always include relevant context as span fields:

- `collector` — collector name
- `metric` — metric name
- `address` — register address
- `slave_id` — Modbus unit ID
- `endpoint` / `device` — connection target
- `data_type` — register data type
- `error` — error details (on failures)

### Do Not

- Use `println!` / `eprintln!` for operational messages.
- Create manual spans when `#[instrument]` suffices.
- Log sensitive data (credentials, bearer tokens).

## Log Levels

| Level | Usage | Examples |
|-------|-------|---------|
| `ERROR` | Unrecoverable failures, persistent errors | Connection failed after all retries, config parse error, export failure |
| `WARN` | Recoverable issues, degraded state | Single poll timeout (will retry), exporter temporarily unreachable |
| `INFO` | Lifecycle events | Process start/stop, config loaded, exporter ready, collector started |
| `DEBUG` | Operational detail | Poll results, decoded values, metric updates, export batch sizes |
| `TRACE` | Wire-level detail | Raw Modbus request/response frames, raw register bytes |

## Output Configuration

The output layer is initialized at startup based on the `logging` section in `config.yaml`:

| `output` value | Behavior |
|----------------|----------|
| `"stdout"` | Structured text format to stdout |
| `"stderr"` | Structured text format to stderr |
| `"json"` | Structured JSON to stderr (suitable for journald/syslog ingestion) |

## Config Reference

See [config.md](config.md) for the `logging` YAML section:

```yaml
logging:
  level: "info"              # trace|debug|info|warn|error
  output: "json"             # json|stdout|stderr
```
