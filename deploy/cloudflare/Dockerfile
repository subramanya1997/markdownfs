FROM rust:1.85 AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY docs ./docs
COPY examples ./examples

RUN cargo build --release --bin markdownfs-server

FROM debian:bookworm-slim

WORKDIR /app
COPY --from=builder /app/target/release/markdownfs-server /usr/local/bin/markdownfs-server

ENV MARKDOWNFS_LISTEN=0.0.0.0:8080
ENV MARKDOWNFS_DATA_DIR=/data

EXPOSE 8080

CMD ["markdownfs-server"]
