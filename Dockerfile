FROM rust:1.76.0-bookworm AS builder

# Use SQLx offline mode to avoid building with the database
ENV SQLX_OFFLINE="true"
ENV MODE="release"

WORKDIR /bt

COPY . .
RUN apt-get update && apt-get install -y \
    libssl-dev \
    gcc

RUN make build

FROM debian:bookworm AS runtime

ENV DATABASE_URL="sqlite:///bt/data/bt.db"
ENV RUST_LOG="info"
ENV DOWNLOADING_PATH_MAPPING=""
ENV ARCHIVED_PATH=""

WORKDIR /bt

RUN apt-get update && apt-get install -y \
    openssl \
    ca-certificates \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /bt/target/release/cmd /usr/bin/bt
COPY --from=builder /bt/migrations/ /bt/migrations/
COPY entrypoint.sh /entrypoint.sh

ENTRYPOINT [ "/entrypoint.sh" ]
