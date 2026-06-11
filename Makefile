.PHONY: sync
sync:
	uv sync --group benchmark

.PHONY: format
format:
	uv run ruff check --fix .
	uv run ruff format .
	cargo fmt

.PHONY: lint
lint:
	uv run ruff check .
	uv run ruff format --check .
	cargo fmt --check
	cargo clippy --all-targets --all-features -- -D warnings

.PHONY: typecheck
typecheck:
	uv run ty check
	uv run pyrefly check

.PHONY: test
test:
	uv run pytest

.PHONY: benchmark-test
benchmark-test:
	uv run pytest benchmark

.PHONY: rust
rust:
	cargo check
	cargo test

.PHONY: build
build:
	uv build

.PHONY: all
all: lint typecheck test benchmark-test rust build
