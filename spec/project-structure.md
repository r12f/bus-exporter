# Project Structure Specification

## Planned File Tree

```
otel-modbus-exporter/
├── .github/
│   └── workflows/
│       ├── ci.yml
│       └── publish.yml
├── .pre-commit-config.yaml
├── Cargo.toml
├── Dockerfile
├── LICENSE
├── Makefile
├── README.md
├── config.yaml                  # Example config
├── spec/
│   ├── ci.md
│   ├── collector.md
│   ├── config.md
│   ├── decoder.md
│   ├── docker.md
│   ├── export-otlp.md
│   ├── export-prometheus.md
│   ├── logging.md
│   ├── metrics.md
│   ├── modbus.md
│   ├── project-structure.md
│   ├── publish.md
│   └── testing.md
├── src/
│   ├── main.rs                  # CLI entry point, config loading, task orchestration
│   ├── main_tests.rs
│   ├── config.rs                # Config structs, YAML deserialization, validation
│   ├── config_tests.rs
│   ├── modbus.rs                # ModbusClient trait, RTU and TCP client impls
│   ├── modbus_tests.rs
│   ├── decoder.rs               # Byte order reordering, type conversion, scale/offset
│   ├── decoder_tests.rs
│   ├── logging.rs               # Tracing subscriber init, syslog layer setup
│   ├── logging_tests.rs
│   ├── collector.rs             # Poll engine, per-collector async task
│   ├── collector_tests.rs
│   ├── metrics.rs               # MetricStore, MetricKey, MetricValue
│   ├── metrics_tests.rs
│   ├── export_otlp.rs           # OTLP protobuf/HTTP exporter
│   ├── export_otlp_tests.rs
│   ├── export_prometheus.rs     # Prometheus /metrics HTTP server
│   └── export_prometheus_tests.rs
└── tests/
    └── integration_test.rs      # End-to-end with mock Modbus server
```

## Module Dependency Graph

```
main
├── config
├── logging
├── collector
│   ├── modbus
│   ├── decoder
│   └── metrics
├── export_otlp
│   └── metrics
└── export_prometheus
    └── metrics
```
