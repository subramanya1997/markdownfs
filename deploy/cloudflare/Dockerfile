FROM rust:1.85 AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY docs ./docs
COPY examples ./examples

RUN cargo build --release --bin mdfs-server

FROM debian:bookworm-slim

WORKDIR /app
COPY --from=builder /app/target/release/mdfs-server /usr/local/bin/mdfs-server

ENV MARKDOWNFS_LISTEN=0.0.0.0:8080
ENV MARKDOWNFS_DATA_DIR=/data

EXPOSE 8080

CMD ["mdfs-server"]
