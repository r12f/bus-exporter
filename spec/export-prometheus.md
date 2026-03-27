# Prometheus Export Specification

## Overview

An HTTP server (using `axum`) serves a Prometheus-compatible `/metrics` endpoint for scraping.

## HTTP Server

- Built with `axum`.
- Listens on the configured `listen` address (default `0.0.0.0:9090`).
- Serves metrics at the configured `path` (default `/metrics`).
- Returns `200 OK` with `Content-Type: text/plain; version=0.0.4; charset=utf-8`.

## Metric Naming

- Metric names are formatted as: `otel_modbus_{metric_name}_{unit}` (if unit is non-empty) or `otel_modbus_{metric_name}`.
- All names are snake_case.
- Invalid characters replaced with `_`.
- Unit suffixes follow Prometheus conventions (e.g., `_volts`, `_kilowatt_hours`, `_celsius`).

## Metric Types

| Internal Type | Prometheus Type |
|---------------|-----------------|
| Gauge | gauge |
| Counter | counter |

## Label Mapping

- All merged labels (global → collector → metric-level) become Prometheus labels.
- Label names must match `[a-zA-Z_][a-zA-Z0-9_]*`.
- Invalid characters in label names are replaced with `_`.

## HELP and TYPE

Each metric includes:
```
# HELP otel_modbus_voltage_phase_a_volts Phase A voltage
# TYPE otel_modbus_voltage_phase_a_volts gauge
otel_modbus_voltage_phase_a_volts{collector="power-meter-01",building="A",floor="2"} 23.1
```

## Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | false | Enable Prometheus endpoint |
| `listen` | string | `0.0.0.0:9090` | Listen address |
| `path` | string | `/metrics` | Metrics path |
