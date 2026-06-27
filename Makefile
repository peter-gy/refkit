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
	cargo clippy --workspace --all-targets --all-features -- -D warnings

.PHONY: typecheck
typecheck:
	uv run ty check
	uv run pyrefly check

.PHONY: test
test:
	uv run pytest

.PHONY: benchmark-test
benchmark-test:
	uv run --package refkit-bench pytest benchmark/tests

.PHONY: rust
rust:
	cargo check --workspace
	cargo test --workspace

.PHONY: rust-floor
rust-floor:
	@if ! rustup toolchain list | grep -Eq '^1\.85(\.|-|$$)'; then \
		echo "Network access: installing Rust 1.85 with rustup."; \
		rustup toolchain install 1.85 --profile minimal; \
	fi
	rustup run 1.85 cargo check --locked --workspace

.PHONY: clean-build
clean-build:
	rm -rf dist target/wheels

.PHONY: clean
clean:
	rm -rf \
		dist \
		wheels \
		build \
		target \
		htmlcov \
		.coverage \
		.pytest_cache \
		.ruff_cache \
		.mypy_cache \
		.pyrefly \
		.ty \
		.tox \
		.nox \
		__pycache__ \
		*.egg-info
	find benchmark crates docs packages \
		\( -name __pycache__ \
		-o -name '*.egg-info' \
		-o -name .pytest_cache \
		-o -name .ruff_cache \
		-o -name .mypy_cache \
		-o -name .pyrefly \
		-o -name .ty \
		-o -name .tox \
		-o -name .nox \) \
		-type d -prune -exec rm -rf {} +
	find benchmark crates docs packages \
		\( -name '*.pyc' -o -name '*.pyo' -o -name '*.so' \) \
		-type f -delete
	@if [ -d benchmark/results ]; then \
		find benchmark/results -mindepth 1 ! -name .gitkeep -exec rm -rf {} +; \
	fi

.PHONY: build
build: clean-build
	uv build --all-packages --no-create-gitignore

.PHONY: all
all: lint typecheck test benchmark-test rust rust-floor build
