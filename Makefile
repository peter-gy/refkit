POLARS_REFKIT_RUST := packages/polars-refkit/rust/Cargo.toml

.PHONY: format
format:
	uv run ruff check --fix .
	uv run ruff format .
	cargo fmt --all
	cargo fmt --manifest-path $(POLARS_REFKIT_RUST) --all

.PHONY: lint
lint:
	uv run ruff check .
	uv run ruff format --check .
	cargo fmt --all --check
	cargo fmt --manifest-path $(POLARS_REFKIT_RUST) --all --check
	cargo clippy --workspace --all-targets --all-features -- -D warnings
	cargo clippy --manifest-path $(POLARS_REFKIT_RUST) --all-targets --all-features -- -D warnings

.PHONY: typecheck
typecheck:
	uv run ty check
	uv run pyrefly check

.PHONY: test
test:
	uv run pytest

.PHONY: benchmark-test
benchmark-test:
	uv run --package refkit-bench pytest packages/refkit-bench/tests

.PHONY: rust
rust:
	cargo check --workspace
	cargo check --manifest-path $(POLARS_REFKIT_RUST) --all-targets --all-features
	cargo test --workspace
	cargo test --manifest-path $(POLARS_REFKIT_RUST)

.PHONY: rust-floor
rust-floor:
	@if ! rustup toolchain list | grep -Eq '^1\.85(\.|-|$$)'; then \
		echo "Network access: installing Rust 1.85 with rustup."; \
		rustup toolchain install 1.85 --profile minimal; \
	fi
	rustup run 1.85 cargo check --locked --workspace
	rustup run 1.85 cargo check --locked --manifest-path $(POLARS_REFKIT_RUST) --all-targets --all-features

.PHONY: clean-build
clean-build:
	rm -rf dist target/wheels target packages/refkit-core/dist packages/polars-refkit/dist

.PHONY: clean
clean:
	rm -rf \
		dist \
		dist-pyodide \
		wheels \
		build \
		target \
		target \
		packages/refkit-core/dist \
		packages/polars-refkit/dist \
		htmlcov \
		.coverage \
		*.profraw \
		.pytest_cache \
		.ruff_cache \
		.mypy_cache \
		.pyrefly \
		.ty \
		.tox \
		.nox \
		__pycache__ \
		*.egg-info
	find docs packages \
		\( -name __pycache__ \
		-o -name '*.egg-info' \
		-o -name .pytest_cache \
		-o -name .ruff_cache \
		-o -name .mypy_cache \
		-o -name .pyrefly \
		-o -name .ty \
		-o -name .tox \
		-o -name .nox \
		-o -name target \
		-o -name .pyodide_build \) \
		-type d -prune -exec rm -rf {} +
	find docs packages \
		\( -name '*.pyc' -o -name '*.pyo' -o -name '*.so' -o -name '*.profraw' \) \
		-type f -delete
	@if [ -d packages/refkit-bench/results ]; then \
		find packages/refkit-bench/results -mindepth 1 ! -name .gitkeep -exec rm -rf {} +; \
	fi

.PHONY: build
build: clean-build
	uv build --all-packages --no-create-gitignore

.PHONY: all
all: lint typecheck test benchmark-test rust rust-floor build
