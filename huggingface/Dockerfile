# syntax=docker/dockerfile:1.6
#
# Hugging Face Space image for MarkdownFS.
# Builds the `mdfs-server` REST binary and runs it on port 7860
# (HF Spaces routes external traffic to this port by convention).

FROM rust:1.83-slim-bookworm AS builder

WORKDIR /build

RUN apt-get update \
 && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
 && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY tests ./tests
COPY examples ./examples

RUN cargo build --release --bin mdfs-server

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --create-home --uid 1000 mdfs

COPY --from=builder /build/target/release/mdfs-server /usr/local/bin/mdfs-server

USER mdfs
WORKDIR /home/mdfs

ENV MARKDOWNFS_DATA_DIR=/home/mdfs/data \
    MARKDOWNFS_LISTEN=0.0.0.0:7860 \
    RUST_LOG=markdownfs=info,tower_http=info

RUN mkdir -p /home/mdfs/data

EXPOSE 7860

CMD ["mdfs-server"]
