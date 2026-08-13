COMPOSE ?= podman-compose
CODEMIE_COMPOSE_DIR := codemie
CODEMIE_COMPOSE_FILE := $(CODEMIE_COMPOSE_DIR)/docker-compose.yml
CODEMIE_COMPOSE_OVERRIDE := ops/dev/podman-compose.yml
CODEMIE_COMPOSE_FILES := -f $(CODEMIE_COMPOSE_FILE) -f $(CODEMIE_COMPOSE_OVERRIDE)
CODEMIE_DEV_DEPENDENCIES := postgres elasticsearch

.PHONY: format lint test o001-check run dev-server

format:
	cargo fmt --all

lint:
	cargo clippy --all-targets -- -D warnings

test:
	cargo test --locked --all-targets
	PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p 'test_*.py'
	PYTHONDONTWRITEBYTECODE=1 python3 scripts/check_o002_examples.py

o001-check:
	PYTHONDONTWRITEBYTECODE=1 python3 scripts/check_o001_controls.py
	PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p 'test_o001_*.py'

run:
	cargo run -- $(ARGS)

dev-server:
	$(COMPOSE) $(CODEMIE_COMPOSE_FILES) up --build -d $(CODEMIE_DEV_DEPENDENCIES)
	bash scripts/wait-for-dev-dependencies.sh
	$(COMPOSE) $(CODEMIE_COMPOSE_FILES) up --build --no-deps codemie
