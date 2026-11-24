# syntax=docker/dockerfile:1
#
# Container image for the REST API (`stellar-contract-platform api serve`).
# Build from the repository root: docker build -t stellar-contract-platform .

FROM rust:1.88-bookworm AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release --locked -p observatory-cli

FROM debian:bookworm-slim AS runtime
# curl is only used by the health check; ca-certificates supports outbound TLS.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --user-group --shell /usr/sbin/nologin observatory \
    && mkdir -p /app \
    && chown observatory:observatory /app
COPY --from=builder /build/target/release/stellar-contract-platform /usr/local/bin/stellar-contract-platform

USER 10001
WORKDIR /app
EXPOSE 8080
ENV RUST_LOG=info

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["curl", "--fail", "--silent", "--show-error", "http://127.0.0.1:8080/api/v1/health"]

ENTRYPOINT ["stellar-contract-platform"]
CMD ["api", "serve", "--bind", "0.0.0.0:8080"]
