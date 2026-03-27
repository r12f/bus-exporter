# Collector Specification

## Overview

Each collector runs as an independent async task (tokio::spawn) that periodically polls a Modbus device and updates the in-memory metric store.

## Poll Engine Design

### One Task Per Collector

- On startup, spawn one `tokio::task` per configured collector.
- Each task owns its Modbus client connection.
- Tasks are independent — a failure in one collector does not affect others.

### Polling Loop

```
loop {
    let start = Instant::now();
    for metric in &collector.metrics {
        match read_metric(&mut client, metric).await {
            Ok(value) => store.update(metric, value),
            Err(e) => {
                log per-metric error;
                increment error counter;
            }
        }
    }
    let elapsed = start.elapsed();
    if elapsed < polling_interval {
        sleep(polling_interval - elapsed).await;
    }
}
```

### Polling Interval

- Measured from the **start** of each poll cycle, not the end.
- If a poll cycle exceeds the interval, the next cycle starts immediately (no negative sleep).
- A warning is logged if poll duration exceeds 80% of the interval.

## Reconnect and Backoff Strategy

When a connection fails or is lost:

1. Log the error with collector name and endpoint/device.
2. Wait with exponential backoff: 1s → 2s → 4s → 8s → … → max 60s.
3. Reset backoff to 1s after a successful poll cycle.
4. During backoff, the collector task is sleeping (not consuming CPU).

## Error Handling

### Per-Metric Errors

- A single metric read failure does NOT abort the poll cycle.
- The failed metric retains its previous value (stale).
- An error counter is incremented per metric.
- Errors are logged at `warn` level.

### Per-Collector Errors

- Connection-level failures (timeout, disconnect) affect all metrics.
- The entire poll cycle is aborted and reconnect logic kicks in.
- Errors are logged at `error` level.

## Graceful Shutdown

- On SIGTERM/SIGINT, all collector tasks receive a cancellation signal via `tokio::sync::watch` or `CancellationToken`.
- Each task completes its current poll cycle (or aborts within 5s), then exits.
- The main task waits for all collector tasks to finish before shutting down exporters.
