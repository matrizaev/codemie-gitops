COMPOSE ?= podman-compose
CODEMIE_COMPOSE_DIR := codemie
CODEMIE_COMPOSE_FILE := $(CODEMIE_COMPOSE_DIR)/docker-compose.yml
CODEMIE_COMPOSE_OVERRIDE := ops/dev/podman-compose.yml
CODEMIE_COMPOSE_FILES := -f $(CODEMIE_COMPOSE_FILE) -f $(CODEMIE_COMPOSE_OVERRIDE)
CODEMIE_DEV_DEPENDENCIES := postgres elasticsearch

.PHONY: format lint test run dev-server

format:
	cargo fmt --all

lint:
	cargo clippy --all-targets -- -D warnings

test:
	cargo test --locked --all-targets

run:
	cargo run -- $(ARGS)

dev-server:
	$(COMPOSE) $(CODEMIE_COMPOSE_FILES) up --build -d $(CODEMIE_DEV_DEPENDENCIES)
	bash scripts/wait-for-dev-dependencies.sh
	$(COMPOSE) $(CODEMIE_COMPOSE_FILES) up --build --no-deps codemie
