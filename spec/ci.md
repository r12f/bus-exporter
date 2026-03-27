# CI Workflow Specification

## GitHub Actions: `.github/workflows/ci.yml`

### Trigger

```yaml
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
```

### Job: CI

```yaml
ci:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        components: rustfmt, clippy
        targets: x86_64-unknown-linux-gnu,aarch64-unknown-linux-gnu
    - run: cargo fmt --check
    - run: cargo clippy -- -D warnings
    - run: cargo test
    - run: cargo build --release --target x86_64-unknown-linux-gnu
    - run: sudo apt-get install -y gcc-aarch64-linux-gnu
    - run: cargo build --release --target aarch64-unknown-linux-gnu
      env:
        CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER: aarch64-linux-gnu-gcc
```

### NO Docker publishing

Docker image build and push is handled exclusively in `publish.yml` (triggered on version tags).

## Pre-commit Config

`.pre-commit-config.yaml`:

```yaml
repos:
  - repo: local
    hooks:
      - id: rustfmt
        name: rustfmt
        entry: cargo fmt --check
        language: system
        pass_filenames: false
      - id: clippy
        name: clippy
        entry: cargo clippy -- -D warnings
        language: system
        pass_filenames: false
```
