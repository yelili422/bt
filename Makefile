
SQLX_CLI_VERSION := $(shell cargo sqlx -V 2>/dev/null)

BUILD_ARGS :=
MODE ?= debug

ifeq ($(MODE), release)
	BUILD_ARGS += --release
endif

.PHONY: build
build: sqlx-prepare
	@echo "$(MODE) build..."
	cargo build --bin cmd $(BUILD_ARGS)

.PHONE: sqlx-prepare
sqlx-prepare:
ifeq ($(SQLX_OFFLINE), true)
	@echo "SQLX_OFFLINE is set, skipping sqlx-prepare"
else
ifndef SQLX_CLI_VERSION
	@echo "sqlx-cli not found, installing..."
	@cargo install sqlx-cli --no-default-features --features sqlite
endif
	sqlx database create -D $(DATABASE_URL)
	sqlx migrate run -D $(DATABASE_URL)
	cargo sqlx prepare -D $(DATABASE_URL)
endif

.PHONY: test
test: build
	cargo test --verbose

.PHONY: migrate
migrate:
	sqlx database create && \
	sqlx migrate run

.PHONY: fmt
fmt:
	cargo fmt
