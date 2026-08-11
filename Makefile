.PHONY: format lint test run dev-server

format:
	cargo fmt --all

lint:
	cargo clippy --all-targets -- -D warnings

test:
	cargo test --all-targets

run:
	cargo run -- $(ARGS)

dev-server:
	$(MAKE) -C codemie run
