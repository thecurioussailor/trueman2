FROM rust:1.83-bookworm AS builder
RUN apt-get update && apt-get install -y --no-install-recommends libssl-dev libpq-dev pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
WORKDIR /app/ws
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 libpq5 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/ws/target/release/ws /usr/local/bin/ws
ENV RUST_LOG=info HOST=0.0.0.0 PORT=9000
EXPOSE 9000
CMD ["ws"]