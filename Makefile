
TARGET :=
BUILD_ARGS :=
INSTALL_PATH :=
INSTALL_ARGS :=

ifneq ($(strip $(TARGET)),)
	BUILD_ARGS += --target $(TARGET)
endif

ifneq ($(strip $(INSTALL_PATH)),)
	INSTALL_ARGS += --path $(INSTALL_PATH)
endif

SQLX_CLI_VERSION := $(shell cargo sqlx -V 2>/dev/null)

.PHONY: build
build: sqlx-prepare
	cargo build $(BUILD_ARGS)

.PHONY: install
install: sqlx-prepare
	cargo install $(BUILD_ARGS) $(INSTALL_ARGS)

.PHONE: sqlx-prepare
sqlx-prepare:
ifndef SQLX_CLI_VERSION
	@echo "sqlx-cli not found, installing..."
	@cargo install sqlx-cli --no-default-features --features sqlite
endif
	sqlx database create -D $(DATABASE_URL)
	sqlx migrate run -D $(DATABASE_URL)
	cargo sqlx prepare -D $(DATABASE_URL)

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
