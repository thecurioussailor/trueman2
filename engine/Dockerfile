FROM rust:1.80-bookworm AS builder
RUN apt-get update && apt-get install -y --no-install-recommends libssl-dev libpq-dev pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
WORKDIR /app/engine
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 libpq5 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/engine/target/release/engine /usr/local/bin/engine
ENV RUST_LOG=info REDIS_URL=redis://redis:6379
CMD ["engine"]