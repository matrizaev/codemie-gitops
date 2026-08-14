COMPOSE ?= podman-compose
CODEMIE_COMPOSE_DIR := codemie
CODEMIE_COMPOSE_FILE := $(CODEMIE_COMPOSE_DIR)/docker-compose.yml
CODEMIE_COMPOSE_OVERRIDE := ops/dev/podman-compose.yml
CODEMIE_COMPOSE_FILES := -f $(CODEMIE_COMPOSE_FILE) -f $(CODEMIE_COMPOSE_OVERRIDE)
CODEMIE_DEV_DEPENDENCIES := postgres elasticsearch

.PHONY: format lint test run dev-server bump-patch

bump-patch:
	@OLD=$$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1); \
	MAJOR=$$(echo "$$OLD" | cut -d. -f1); \
	MINOR=$$(echo "$$OLD" | cut -d. -f2); \
	PATCH=$$(echo "$$OLD" | cut -d. -f3); \
	NEW="$$MAJOR.$$MINOR.$$((PATCH + 1))"; \
	sed -i "s/^version = \"$$OLD\"/version = \"$$NEW\"/" Cargo.toml; \
	cargo update --package codemie-gitops; \
	echo "Bumped $$OLD → $$NEW"

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
