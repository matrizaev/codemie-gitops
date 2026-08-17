COMPOSE ?= podman-compose
CODEMIE_COMPOSE_DIR := ../codemie
CODEMIE_COMPOSE_FILE := $(CODEMIE_COMPOSE_DIR)/docker-compose.yml
CODEMIE_COMPOSE_OVERRIDE := ops/dev/podman-compose.yml
CODEMIE_COMPOSE_FILES := -f $(CODEMIE_COMPOSE_FILE) -f $(CODEMIE_COMPOSE_OVERRIDE)
CODEMIE_DEV_DEPENDENCIES := postgres elasticsearch
# Dev-server API origin used by the e2e target (override with make CODEMIE_URL=...).
CODEMIE_URL ?= http://127.0.0.1:8080

.PHONY: format lint test run dev-server e2e bump-patch

# End-to-end smoke test against a running dev server (make dev-server).
#
# Uses cargo run for the CLI. Auth mode is resolved by `login` from the
# environment: local-auth (CODEMIE_EMAIL/CODEMIE_PASSWORD, no CODEMIE_AUTH_URL)
# or Keycloak (CODEMIE_AUTH_URL + CODEMIE_CLIENT_SECRET). The target applies
# one declaration of each entity kind, saves each back, and re-lints the
# saved outputs. Saved files are left in a fresh temp directory printed at
# the end.
#
#   make dev-server           # in one terminal
#   export CODEMIE_EMAIL=... CODEMIE_PASSWORD=...
#   make e2e
#
# The examples are applied in dependency order (Skill before the Assistant
# that references it). The project named in the examples (portable-example)
# is created if it does not exist, or the logged-in user is assigned to it if
# it exists without membership, so a fresh dev server works out of the box.
# The Workflow kind is applied from the committed example (now a valid
# single-state graph that offline validation and live apply both accept). The
# Datasource kind is exercised with a generated git datasource because the
# committed google example references a placeholder integration that cannot
# exist on a fresh server; it uses a real public repository because current
# servers validate accessibility, and a unique repo name per run so the create
# path is exercised every time (current servers 404 the git datasource update
# when the background indexing task did not persist its repository record).
e2e:
	@set -eu; \
	cmd() { cargo run --quiet -- "$$@"; }; \
	E2E_OUT="$$(mktemp -d)"; \
	E2E_REPO="example-git-repository-$$$$"; \
	export CODEMIE_URL="$(CODEMIE_URL)"; \
	echo "== login =="; \
	CODEMIE_TOKEN="$$(cmd login --url "$$CODEMIE_URL")"; \
	export CODEMIE_TOKEN; \
	echo "== ensure project =="; \
	USER_JSON="$$(curl -sS -H "Authorization: Bearer $$CODEMIE_TOKEN" "$$CODEMIE_URL/v1/user")"; \
	PROJECTS="$$(printf '%s' "$$USER_JSON" | sed -n 's/.*"projects":\(\[[^]]*\]\).*/\1/p')"; \
	if printf '%s' "$$PROJECTS" | grep -q '"portable-example"'; then \
	  echo "project portable-example already available"; \
	else \
	  echo "-- creating project portable-example"; \
	  CREATE_STATUS="$$(curl -sS -o /dev/null -w '%{http_code}' -X POST \
	    -H "Authorization: Bearer $$CODEMIE_TOKEN" -H "Content-Type: application/json" \
	    -d '{"name":"portable-example","description":"E2E test project"}' \
	    "$$CODEMIE_URL/v1/projects")"; \
	  if [ "$$CREATE_STATUS" != "201" ]; then \
	    echo "-- project exists without membership; assigning user"; \
	    USER_ID="$$(printf '%s' "$$USER_JSON" | sed -n 's/.*"user_id":"\([^"]*\)".*/\1/p')"; \
	    ASSIGN_STATUS="$$(curl -sS -o /dev/null -w '%{http_code}' -X POST \
	      -H "Authorization: Bearer $$CODEMIE_TOKEN" -H "Content-Type: application/json" \
	      -d "{\"user_id\":\"$$USER_ID\",\"is_project_admin\":true}" \
	      "$$CODEMIE_URL/v1/projects/portable-example/assignment")"; \
	    if [ "$$ASSIGN_STATUS" != "200" ]; then \
	      echo "project ensure failed: create=$$CREATE_STATUS assign=$$ASSIGN_STATUS" >&2; \
	      exit 1; \
	    fi; \
	  fi; \
	fi; \
	echo "== apply all entity kinds =="; \
	for f in \
	  examples/repository/skills/example-skill.yaml \
	  examples/repository/assistants/example-assistant.yaml \
	  examples/repository/workflows/example-workflow.yaml; do \
	  echo "-- apply $$f"; \
	  cmd apply --file "$$f"; \
	done; \
	echo "-- apply generated git datasource"; \
	printf '%s\n' \
	  'apiVersion: codemie.epam.com/v1alpha1' \
	  'kind: Datasource' \
	  'metadata:' \
	  '  project: portable-example' \
	  '  repo_name: '"$$E2E_REPO" \
	  'spec:' \
	  '  index_type: git' \
	  '  description: E2E git datasource.' \
	  '  link: https://github.com/octocat/Hello-World.git' \
	  '  branch: master' \
	  '  indexType: code' \
	  '  docsGeneration: false' \
	  '  projectSpaceVisible: true' > "$$E2E_OUT/git-datasource.yaml"; \
	cmd apply --file "$$E2E_OUT/git-datasource.yaml"; \
	echo "== save each entity back =="; \
	cmd save --kind Assistant --project portable-example --slug example-assistant --file "$$E2E_OUT/saved-assistant.yaml"; \
	cmd save --kind Workflow  --project portable-example --slug example-workflow  --file "$$E2E_OUT/saved-workflow.yaml"; \
	cmd save --kind Skill     --project portable-example --name example-skill     --file "$$E2E_OUT/saved-skill.yaml"; \
	cmd save --kind Datasource --project portable-example --repo-name "$$E2E_REPO" --file "$$E2E_OUT/saved-datasource.yaml"; \
	echo "== re-lint saved declarations =="; \
	for f in "$$E2E_OUT"/*.yaml; do cmd lint --file "$$f"; done; \
	echo "== e2e OK — saved outputs in $$E2E_OUT =="

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
