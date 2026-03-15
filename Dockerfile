FROM rust:1-bookworm AS builder
WORKDIR /app
COPY . .

RUN rm -f Cargo.lock && cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
#  Cargo.toml name
COPY --from=builder /app/target/release/g-bot /usr/local/bin/g-bot
CMD ["/usr/local/bin/g-bot"]