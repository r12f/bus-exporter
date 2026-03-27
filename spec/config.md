# Configuration Specification

## Overview

Configuration is loaded from a YAML file specified via `--config` CLI flag. Default: `config.yaml` in the working directory.

## Schema

### Top-level

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `global_labels` | `map<string, string>` | No | `{}` | Labels applied to all metrics |
| `logging` | `Logging` | No | See below | Logging configuration |
| `exporters` | `Exporters` | Yes | — | Export configuration |
| `collectors` | `list<Collector>` | Yes | — | At least one collector required |

### Exporters

#### `exporters.otlp`

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `enabled` | `bool` | No | `false` | Enable OTLP export |
| `endpoint` | `string` | Yes (if enabled) | — | OTLP HTTP endpoint (e.g., `http://host:4318`) |
| `timeout` | `string` | No | `"10s"` | Request timeout (duration string) |
| `headers` | `map<string, string>` | No | `{}` | Additional HTTP headers |

#### `exporters.prometheus`

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `enabled` | `bool` | No | `false` | Enable Prometheus endpoint |
| `listen` | `string` | No | `"0.0.0.0:9090"` | Listen address |
| `path` | `string` | No | `"/metrics"` | Metrics path |

### Collector

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | `string` | Yes | — | Unique collector name (used as label) |
| `protocol` | `Protocol` | Yes | — | Connection protocol |
| `slave_id` | `u8` | Yes | — | Modbus slave/unit ID (1-247) |
| `polling_interval` | `string` | No | `"10s"` | Poll interval (duration string) |
| `labels` | `map<string, string>` | No | `{}` | Labels for all metrics in this collector |
| `metrics` | `list<Metric>` | Yes | — | At least one metric required |

### Protocol

#### TCP

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `type` | `string` | Yes | — | Must be `"tcp"` |
| `endpoint` | `string` | Yes | — | `host:port` |

#### RTU

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `type` | `string` | Yes | — | Must be `"rtu"` |
| `device` | `string` | Yes | — | Serial device path (e.g., `/dev/ttyUSB0`) |
| `bps` | `u32` | No | `9600` | Baud rate |
| `data_bits` | `u8` | No | `8` | Data bits (5-8) |
| `stop_bits` | `u8` | No | `1` | Stop bits (1-2) |
| `parity` | `string` | No | `"none"` | `"none"`, `"even"`, or `"odd"` |

### Metric

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | `string` | Yes | — | Metric name (snake_case recommended) |
| `description` | `string` | No | `""` | Human-readable description |
| `type` | `string` | Yes | — | `"counter"` or `"gauge"` |
| `register_type` | `string` | Yes | — | `"holding"`, `"input"`, `"coil"`, or `"discrete"` |
| `address` | `u16` | Yes | — | Starting register address (0-based) |
| `data_type` | `string` | Yes | — | One of: `u16`, `i16`, `u32`, `i32`, `f32`, `u64`, `i64`, `f64`, `bool` |
| `byte_order` | `string` | No | `"big_endian"` | `"big_endian"`, `"little_endian"`, `"mid_big_endian"`, `"mid_little_endian"` |
| `scale` | `f64` | No | `1.0` | Multiplicative scale factor |
| `offset` | `f64` | No | `0.0` | Additive offset |
| `unit` | `string` | No | `""` | Unit label (e.g., `"V"`, `"kWh"`, `"°C"`) |

### Logging

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `level` | `string` | No | `"info"` | Log level: `trace`, `debug`, `info`, `warn`, `error` |
| `output` | `string` | No | `"syslog"` | Output target: `syslog`, `stdout`, `stderr` |
| `syslog_facility` | `string` | No | `"daemon"` | Syslog facility (e.g., `daemon`, `local0`–`local7`) |

```yaml
logging:
  level: "info"              # trace|debug|info|warn|error
  output: "syslog"           # syslog|stdout|stderr
  syslog_facility: "daemon"
```

## Validation Rules

1. At least one exporter must be enabled.
2. At least one collector must be defined.
3. Each collector must have at least one metric.
4. Collector names must be unique.
5. Metric names must be unique within a collector.
6. `slave_id` must be 1-247.
7. `coil` and `discrete` register types must use `data_type: bool`.
8. `bool` data type must use `coil` or `discrete` register types.
9. Duration strings must parse (e.g., `"5s"`, `"1m"`, `"500ms"`).
10. `byte_order` is ignored for `u16`, `i16`, and `bool` (single register).

## Scale Formula

```
output_value = raw_value * scale + offset
```

Example: raw register value `245` with `scale: 0.1` and `offset: -40.0` → `245 * 0.1 + (-40.0) = -15.5`
