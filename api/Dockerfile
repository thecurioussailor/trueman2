# ---- builder ----
    FROM rust:1.80-bookworm AS builder
    RUN apt-get update && apt-get install -y --no-install-recommends libssl-dev libpq-dev pkg-config && rm -rf /var/lib/apt/lists/*
    WORKDIR /app
    COPY . .
    WORKDIR /app/api
    RUN cargo build --release
    
    # ---- runtime ----
    FROM debian:bookworm-slim
    RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 libpq5 && rm -rf /var/lib/apt/lists/*
    COPY --from=builder /app/api/target/release/api /usr/local/bin/api
    ENV HOST=0.0.0.0 PORT=8080 RUST_LOG=info
    EXPOSE 8080
    CMD ["api"]