# Használjuk a legfrissebb Rust verziót (ami már 1.88 felett van)
FROM rust:1.88-slim as builder

# Alapvető build eszközök telepítése
RUN apt-get update && apt-get install -y pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/g-bot
COPY . .

# Fordítás
RUN cargo build --release

# Második stage a futtatáshoz
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Másoljuk át a binárist
COPY --from=builder /usr/src/g-bot/target/release/g-bot /usr/local/bin/g-bot

# Indítás
CMD ["g-bot"]