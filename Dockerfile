FROM debian:bookworm AS builder

SHELL ["/bin/bash", "--login", "-c"]

# Use SQLx offline mode to avoid building with the database
ENV SQLX_OFFLINE="true"
ENV MODE="release"

RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    libssl-dev \
    gcc \
    pkg-config

# Install Rust
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install nvm and node
ENV NVM_DIR="/root/.nvm"
ENV NODE_VERSION="20"
ENV REACT_APP_API_URL="http://localhost:8081"

RUN curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.7/install.sh | bash
RUN nvm install $NODE_VERSION && nvm use $NODE_VERSION

WORKDIR /bt/ui

# Copy package.json and pnpm-lock.yaml only to cache dependencies
COPY ui/package.json ui/pnpm-lock.yaml ./

RUN corepack enable pnpm && corepack use pnpm@8.15.0
RUN pnpm install

COPY ui/ ./

RUN pnpm run build

WORKDIR /bt

COPY . .

RUN make build

FROM debian:bookworm AS runtime

SHELL ["/bin/bash", "--login", "-c"]

ENV DATABASE_URL="sqlite:///bt/data/bt.db"
ENV RUST_LOG="info"
ENV DOWNLOADING_PATH_MAPPING=""
ENV ARCHIVED_PATH=""

WORKDIR /bt

RUN apt-get update && apt-get install -y \
    openssl \
    ca-certificates \
    nginx \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /bt/target/release/cmd /usr/bin/bt
COPY --from=builder /bt/migrations/ /bt/migrations/
COPY --from=builder /bt/ui/dist /usr/share/nginx/html
COPY entrypoint.sh /entrypoint.sh

EXPOSE 80/tcp
ENTRYPOINT [ "/entrypoint.sh" ]
