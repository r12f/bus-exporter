# Multi-stage Dockerfile for otel-modbus-exporter
# Supports linux/amd64 and linux/arm64

FROM rust:alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /src
COPY . .
RUN cargo build --release

FROM alpine:3.20
RUN apk add --no-cache ca-certificates
COPY --from=builder /src/target/release/otel-modbus-exporter /usr/local/bin/
EXPOSE 9090
HEALTHCHECK NONE
ENTRYPOINT ["otel-modbus-exporter"]
CMD ["--config", "/etc/otel-modbus-exporter/config.yaml"]
