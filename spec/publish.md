# Publish Workflow Specification

## GitHub Actions: `.github/workflows/publish.yml`

### Trigger

```yaml
on:
  push:
    tags:
      - 'v*'
```

### Job 1: Publish to crates.io

```yaml
publish-crate:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - run: cargo publish
      env:
        CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

### Job 2: Docker Image

```yaml
publish-docker:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: docker/setup-qemu-action@v3
    - uses: docker/setup-buildx-action@v3
    - uses: docker/login-action@v3
      with:
        username: ${{ secrets.DOCKERHUB_USERNAME }}
        password: ${{ secrets.DOCKERHUB_TOKEN }}
    - uses: docker/build-push-action@v5
      with:
        push: true
        platforms: linux/amd64,linux/arm64
        tags: |
          r12f/otel-modbus-exporter:${{ github.ref_name }}
          r12f/otel-modbus-exporter:latest
```

### Required Secrets

| Secret | Purpose |
|--------|---------|
| `CARGO_REGISTRY_TOKEN` | crates.io publish token |
| `DOCKERHUB_USERNAME` | Docker Hub username |
| `DOCKERHUB_TOKEN` | Docker Hub access token |
