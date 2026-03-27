# Metrics Store Specification

## Overview

The metric store holds the latest values for all metrics and serves as the shared state between collectors (writers) and exporters (readers).

## In-Memory Design

- A single `MetricStore` instance shared via `Arc<MetricStore>`.
- Internally uses `DashMap<MetricKey, MetricValue>` for lock-free concurrent access.
- `MetricKey`: combination of collector name + metric name.

### MetricValue

```rust
struct MetricValue {
    value: f64,
    metric_type: MetricType, // Gauge or Counter
    labels: BTreeMap<String, String>,
    description: String,
    unit: String,
    updated_at: Instant,
}
```

## Label Merging Order

Labels are merged in this order (later wins on conflict):

1. **Global labels** — from `global_labels` in config
2. **Collector labels** — from `collectors[].labels`
3. **Metric-level labels** — automatically added:
   - `collector`: collector name
   - `unit`: metric unit (if non-empty)

## Gauge vs Counter Semantics

- **Gauge**: represents a point-in-time value. Each poll overwrites the previous value.
- **Counter**: represents a monotonically increasing total. Each poll overwrites with the latest reading from the device. The exporter is responsible for communicating counter semantics (OTLP cumulative temporality, Prometheus counter type).

## Thread Safety

- `DashMap` provides concurrent read/write without a global lock.
- Collectors write from their own tokio tasks.
- Exporters read on-demand (Prometheus on HTTP request, OTLP on export interval).
- No mutex contention between collectors since each writes to distinct keys.
