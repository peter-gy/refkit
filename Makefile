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

.PHONY: rust-floor
rust-floor:
	@if ! rustup toolchain list | grep -Eq '^1\.85(\.|-|$$)'; then \
		echo "Network access: installing Rust 1.85 with rustup."; \
		rustup toolchain install 1.85 --profile minimal; \
	fi
	rustup run 1.85 cargo check --locked

.PHONY: build
build:
	uv build

.PHONY: package-check
package-check:
	@tmp=$$(mktemp -d); \
	trap 'rm -rf "$$tmp"' EXIT; \
	set -e; \
	uv build --out-dir "$$tmp"; \
	uv run python scripts/inspect_package.py "$$tmp"

.PHONY: release-smoke
release-smoke:
	@tmp=$$(mktemp -d); \
	trap 'rm -rf "$$tmp"' EXIT; \
	set -e; \
	uv build --out-dir "$$tmp"; \
	uv run python scripts/inspect_package.py "$$tmp"; \
	uv run python scripts/release_smoke.py --dist-dir "$$tmp" --pythons 3.11 3.12 3.13 3.14

.PHONY: dependency-provenance
dependency-provenance:
	uv run python scripts/check_dependency_provenance.py
	cargo tree --locked -i hayagriva
	cargo tree --locked -i biblatex
	cargo tree --locked -i citationberg
	cargo tree --locked -i serde_yaml
	cargo tree --locked -i unsafe-libyaml

.PHONY: advisory
advisory:
	@echo "Network access: this target may download advisory databases and audit tools."
	@if ! cargo audit --version >/dev/null 2>&1; then cargo install cargo-audit --locked; fi
	uv run python scripts/check_dependency_provenance.py
	cargo audit --deny warnings --ignore RUSTSEC-2024-0436
	@tmp=$$(mktemp); \
	trap 'rm -f "$$tmp"' EXIT; \
	set -e; \
	uv export --locked --all-groups --format requirements.txt --no-hashes --no-emit-project --no-emit-workspace > $$tmp; \
	uvx pip-audit --strict --requirement $$tmp; \
	rc=$$?; rm -f $$tmp; exit $$rc

.PHONY: all
all: lint typecheck test benchmark-test rust rust-floor package-check
