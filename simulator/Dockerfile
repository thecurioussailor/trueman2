FROM rust:1.80-bookworm AS builder
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
WORKDIR /app/simulator
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/simulator/target/release/simulator /usr/local/bin/simulator
ENV RUST_LOG=info
CMD ["simulator"]