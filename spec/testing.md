# Testing Specification

## Convention

Every source file `src/xxx.rs` has a corresponding test file `src/xxx_tests.rs`.

Each source file includes the test module:

```rust
#[cfg(test)]
#[path = "xxx_tests.rs"]
mod tests;
```

## Unit Test Expectations

| Module | Test Focus |
|--------|------------|
| `config` | YAML parsing, validation rules, defaults, error messages |
| `decoder` | All data types × all byte orders, scale/offset, bool, edge cases (NaN, overflow) |
| `modbus` | Mock client responses, error handling, register count calculation |
| `collector` | Poll loop logic (mocked client), error counting, interval timing |
| `metrics` | Store read/write, label merging, concurrent access |
| `export_otlp` | Metric-to-OTLP mapping, serialization, retry logic (mocked HTTP) |
| `export_prometheus` | Metric formatting, naming conventions, label escaping |

## Integration Tests

Located in `tests/` directory.

### Mock Modbus Server

- Use `tokio-modbus` server capabilities or a custom mock.
- Simulate a Modbus TCP device with known register values.
- Test end-to-end: config → collector → metric store → exporter output.

### Test Scenarios

1. **Happy path**: Poll mock device, verify Prometheus output matches expected values.
2. **Reconnect**: Kill mock server mid-poll, verify reconnect and recovery.
3. **Multiple collectors**: Run 2+ collectors simultaneously against different mock servers.
4. **Invalid config**: Verify startup fails with clear error messages.
5. **Scale/offset**: Verify decoded values match expected transformed output.

## E2E Tests

E2E tests validate the full pipeline using real Docker containers rather than in-process mocks. They use `oitc/modbus-server` as a Modbus TCP simulator with pre-loaded register values, and assert against the Prometheus `/metrics` endpoint.

See [e2e-testing.md](e2e-testing.md) for the full E2E testing specification.
