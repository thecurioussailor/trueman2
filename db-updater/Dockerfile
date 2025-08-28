FROM rust:1.83-bookworm AS builder
RUN apt-get update && apt-get install -y --no-install-recommends libssl-dev libpq-dev pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
WORKDIR /app/db-updater
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 libpq5 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/db-updater/target/release/db-updater /usr/local/bin/db-updater
ENV RUST_LOG=info
CMD ["db-updater"]