.PHONY: format lint test o001-check run dev-server

format:
	cargo fmt --all

lint:
	cargo clippy --all-targets -- -D warnings

test:
	cargo test --locked --all-targets
	PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p 'test_o001_*.py'

o001-check:
	PYTHONDONTWRITEBYTECODE=1 python3 scripts/check_o001_controls.py
	PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p 'test_o001_*.py'

run:
	cargo run -- $(ARGS)

dev-server:
	$(MAKE) -C codemie run
