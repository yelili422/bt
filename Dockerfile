FROM rust:1.76.0-bookworm AS builder

ENV APP_PATH=/bt
ENV DATABASE_URL=sqlite://$APP_PATH/data/bt.db

WORKDIR $APP_PATH

COPY . .
RUN apt-get update && apt-get install -y \
    libssl-dev \
    gcc

RUN mkdir data
RUN make install INSTALL_PATH=.

FROM debian:bookworm AS runtime

ENV DATABASE_URL="sqlite:///bt/data/bt.db"
ENV RUST_LOG="info"
ENV DOWNLOADING_PATH_MAPPING=""
ENV ARCHIVED_PATH=""

WORKDIR /bt

RUN apt-get update && apt-get install -y \
    openssl \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/cargo/bin/cmd /usr/bin
COPY --from=builder /bt/data /bt/data
COPY entrypoint.sh /entrypoint.sh

ENTRYPOINT [ "/entrypoint.sh" ]
