JS_REFKIT_RUST := packages/refkit-js/rust/Cargo.toml
POLARS_REFKIT_RUST := packages/polars-refkit/rust/Cargo.toml
UV_EXCLUDE_NEWER := 3 days
UV_EXCLUDE_NEWER_PACKAGE := agent-plugins=2026-09-07T00:00:00Z
UV_RESOLVER_FLAGS := --no-config --exclude-newer "$(UV_EXCLUDE_NEWER)" --exclude-newer-package "$(UV_EXCLUDE_NEWER_PACKAGE)"
UV_RUN := uv run $(UV_RESOLVER_FLAGS) --locked --all-packages --group dev
PYTHON := uv run $(UV_RESOLVER_FLAGS) --isolated --locked --only-group build python
UV_LINT := uv run $(UV_RESOLVER_FLAGS) --isolated --locked --only-group lint
PNPM_DOCS := pnpm --dir docs
DOCS_PAGES_BASE_PATH := /refkit
RUST_FLOOR := 1.88
RUST_SYSROOT := $(shell rustc --print sysroot)
RUST_REMAP_FLAGS := --remap-path-prefix=$(HOME)=home --remap-path-prefix=$(HOME)/.cargo/registry/src=cargo-registry --remap-path-prefix=$(HOME)/.cargo/git/checkouts=cargo-git --remap-path-prefix=$(HOME)/.rustup=rustup --remap-path-prefix=$(RUST_SYSROOT)=rust-toolchain --remap-path-prefix=$(CURDIR)=refkit

.PHONY: sync
sync:
	uv sync $(UV_RESOLVER_FLAGS) --locked --all-packages --group dev

.PHONY: format
format:
	$(UV_LINT) ruff check --fix .
	$(UV_LINT) ruff format .
	cargo fmt --all
	cargo fmt --manifest-path $(POLARS_REFKIT_RUST) --all
	cargo fmt --manifest-path $(JS_REFKIT_RUST) --all

.PHONY: python-lint
python-lint:
	$(UV_LINT) ruff check .
	$(UV_LINT) ruff format --check .

.PHONY: rust-lint
rust-lint:
	cargo fmt --all --check
	cargo fmt --manifest-path $(POLARS_REFKIT_RUST) --all --check
	cargo fmt --manifest-path $(JS_REFKIT_RUST) --all --check
	cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
	cargo clippy --locked --manifest-path $(JS_REFKIT_RUST) --all-targets --all-features -- -D warnings
	cargo clippy --locked --manifest-path $(POLARS_REFKIT_RUST) --all-targets --all-features -- -D warnings

.PHONY: lint
lint: python-lint rust-lint

.PHONY: typecheck
typecheck:
	$(UV_RUN) ty check
	$(UV_RUN) pyrefly check

.PHONY: test
test:
	$(UV_RUN) python -m pytest

.PHONY: refkit-develop
refkit-develop:
	uv pip install $(UV_RESOLVER_FLAGS) --reinstall --no-deps --editable packages/refkit

.PHONY: refkit-develop-release
refkit-develop-release:
	MATURIN_PEP517_ARGS="--profile release --locked" uv pip install $(UV_RESOLVER_FLAGS) --reinstall --no-deps --editable packages/refkit

.PHONY: polars-refkit-develop
polars-refkit-develop:
	uv pip install $(UV_RESOLVER_FLAGS) --reinstall --no-deps --editable packages/polars-refkit

.PHONY: polars-refkit-develop-release
polars-refkit-develop-release:
	MATURIN_PEP517_ARGS="--profile release --locked" uv pip install $(UV_RESOLVER_FLAGS) --reinstall --no-deps --editable packages/polars-refkit

.PHONY: benchmark-test
benchmark-test:
	npm ci --ignore-scripts --no-audit --no-fund --prefix packages/refkit-bench/node
	$(UV_RUN) python -m pytest -c packages/refkit-bench/pyproject.toml --cov-config=packages/refkit-bench/pyproject.toml packages/refkit-bench/tests

.PHONY: rust
rust:
	cargo check --locked --workspace --all-targets --all-features
	cargo check --locked --manifest-path $(JS_REFKIT_RUST) --all-targets --all-features
	cargo check --locked --manifest-path $(POLARS_REFKIT_RUST) --all-targets --all-features
	cargo test --locked --workspace
	cargo test --locked --manifest-path $(JS_REFKIT_RUST)
	cargo test --locked --manifest-path $(POLARS_REFKIT_RUST)

.PHONY: rust-floor
rust-floor:
	@if ! rustup toolchain list | grep -Eq '^$(RUST_FLOOR)(\.|-|$$)'; then \
		echo "Network access: installing Rust $(RUST_FLOOR) with rustup."; \
		rustup toolchain install $(RUST_FLOOR) --profile minimal; \
	fi
	RUSTC="$$(rustup which --toolchain $(RUST_FLOOR) rustc)" "$$(rustup which --toolchain $(RUST_FLOOR) cargo)" check --locked --workspace --all-targets --all-features
	RUSTC="$$(rustup which --toolchain $(RUST_FLOOR) rustc)" "$$(rustup which --toolchain $(RUST_FLOOR) cargo)" check --locked --manifest-path $(JS_REFKIT_RUST) --all-targets --all-features
	RUSTC="$$(rustup which --toolchain $(RUST_FLOOR) rustc)" "$$(rustup which --toolchain $(RUST_FLOOR) cargo)" check --locked --manifest-path $(POLARS_REFKIT_RUST) --all-targets --all-features

.PHONY: pyodide-lock pyodide-lock-check
pyodide-lock:
	$(PYTHON) scripts/pyodide_lock.py

pyodide-lock-check:
	$(PYTHON) scripts/pyodide_lock.py --check

.PHONY: clean-dist
clean-dist:
	rm -rf dist packages/refkit/dist packages/polars-refkit/dist

.PHONY: clean
clean:
	rm -rf \
		dist \
		dist-pyodide \
		wheels \
		build \
		target \
		packages/polars-refkit/rust/target \
		packages/refkit/dist \
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
build: clean-dist
	uv build $(UV_RESOLVER_FLAGS) --package refkit --sdist --no-create-gitignore
	RUSTFLAGS="$(RUST_REMAP_FLAGS)" uv build $(UV_RESOLVER_FLAGS) --package refkit --wheel --no-create-gitignore
	uv build $(UV_RESOLVER_FLAGS) --package polars-refkit --sdist --no-create-gitignore
	RUSTFLAGS="$(RUST_REMAP_FLAGS)" uv build $(UV_RESOLVER_FLAGS) --package polars-refkit --wheel --no-create-gitignore
	$(PYTHON) scripts/distribution_contract.py dist/*

.PHONY: lock
lock:
	uv lock $(UV_RESOLVER_FLAGS) --check

.PHONY: lock-upgrade
lock-upgrade:
	uv lock $(UV_RESOLVER_FLAGS) --upgrade

.PHONY: release-check
release-check:
	$(PYTHON) scripts/release_contract.py

.PHONY: architecture-check
architecture-check:
	$(PYTHON) scripts/architecture_contract.py

.PHONY: docs-source-check
docs-source-check:
	$(PYTHON) scripts/docs_contract.py

.PHONY: docs-examples-check
docs-examples-check:
	$(UV_RUN) python -m refkit_tests.check_examples --root .

.PHONY: docs-dev
docs-dev:
	$(PNPM_DOCS) dev

.PHONY: docs-site-check
docs-site-check:
	$(PNPM_DOCS) install --frozen-lockfile
	$(PNPM_DOCS) build
	BASE_PATH=$(DOCS_PAGES_BASE_PATH) $(PNPM_DOCS) build

.PHONY: docs-build
docs-build:
	$(PNPM_DOCS) install --frozen-lockfile
	$(PNPM_DOCS) build

.PHONY: docs-check
docs-check: docs-source-check docs-site-check

.PHONY: check
check: js-check lock release-check architecture-check docs-check pyodide-lock-check lint typecheck test benchmark-test docs-examples-check rust rust-floor build

.PHONY: js-build js-check
js-build:
	npm --prefix packages/refkit-js ci --ignore-scripts --no-audit --no-fund
	npm --prefix packages/refkit-js run build

js-check: js-build
	npm --prefix packages/refkit-js run lint
	npm --prefix packages/refkit-js run typecheck
	npm --prefix packages/refkit-js test
	$(UV_RUN) python packages/refkit-js/tests/parity.py
	npm --prefix packages/refkit-js run test:docs
	npm --prefix packages/refkit-js run test:package
	npm --prefix packages/refkit-js run test:browser
