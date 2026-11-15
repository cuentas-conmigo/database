# syntax=docker/dockerfile:1.7
# Build: docker build --platform linux/arm64 -t cuentas-api .
FROM rust:1-slim-trixie AS build
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends cmake pkg-config \
    && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked --bin cuentas-api \
    && cp target/release/cuentas-api /usr/local/bin/cuentas-api

FROM gcr.io/distroless/cc-debian13:nonroot
COPY --from=build /usr/local/bin/cuentas-api /usr/local/bin/cuentas-api
# Postgres comes from DATABASE_URL; migrations and seeds are inside the binary.
ENV PORT=8080 RUST_LOG=info,sqlx=warn
EXPOSE 8080
USER nonroot
ENTRYPOINT ["/usr/local/bin/cuentas-api"]
