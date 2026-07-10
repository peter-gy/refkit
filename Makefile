POLARS_REFKIT_RUST := packages/polars-refkit/rust/Cargo.toml
TIDY_RUST := crates/bibtex-tidy-rs/Cargo.toml
UV_RUN := uv run --locked --all-packages --group dev
RUST_FLOOR := 1.88
RUST_SYSROOT := $(shell rustc --print sysroot)
RUST_REMAP_FLAGS := --remap-path-prefix=$(HOME)=home --remap-path-prefix=$(HOME)/.cargo/registry/src=cargo-registry --remap-path-prefix=$(HOME)/.cargo/git/checkouts=cargo-git --remap-path-prefix=$(HOME)/.rustup=rustup --remap-path-prefix=$(RUST_SYSROOT)=rust-toolchain --remap-path-prefix=$(CURDIR)=refkit

.PHONY: format
format:
	$(UV_RUN) ruff check --fix .
	$(UV_RUN) ruff format .
	cargo fmt --all
	cargo fmt --manifest-path $(TIDY_RUST) --all
	cargo fmt --manifest-path $(POLARS_REFKIT_RUST) --all

.PHONY: python-lint
python-lint:
	$(UV_RUN) ruff check .
	$(UV_RUN) ruff format --check .

.PHONY: rust-lint
rust-lint:
	cargo fmt --all --check
	cargo fmt --manifest-path $(TIDY_RUST) --all --check
	cargo fmt --manifest-path $(POLARS_REFKIT_RUST) --all --check
	cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
	cargo clippy --locked --manifest-path $(TIDY_RUST) --all-targets --all-features -- -D warnings
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

.PHONY: benchmark-test
benchmark-test:
	$(UV_RUN) python -m pytest packages/refkit-bench/tests

.PHONY: rust
rust:
	cargo check --locked --workspace --all-targets --all-features
	cargo check --locked --manifest-path $(TIDY_RUST) --all-targets --all-features
	cargo check --locked --manifest-path $(POLARS_REFKIT_RUST) --all-targets --all-features
	cargo test --locked --workspace
	cargo test --locked --manifest-path $(TIDY_RUST)
	cargo test --locked --manifest-path $(POLARS_REFKIT_RUST)

.PHONY: rust-floor
rust-floor:
	@if ! rustup toolchain list | grep -Eq '^$(RUST_FLOOR)(\.|-|$$)'; then \
		echo "Network access: installing Rust $(RUST_FLOOR) with rustup."; \
		rustup toolchain install $(RUST_FLOOR) --profile minimal; \
	fi
	RUSTC="$$(rustup which --toolchain $(RUST_FLOOR) rustc)" "$$(rustup which --toolchain $(RUST_FLOOR) cargo)" check --locked --workspace --all-targets --all-features
	RUSTC="$$(rustup which --toolchain $(RUST_FLOOR) rustc)" "$$(rustup which --toolchain $(RUST_FLOOR) cargo)" check --locked --manifest-path $(TIDY_RUST) --all-targets --all-features
	RUSTC="$$(rustup which --toolchain $(RUST_FLOOR) rustc)" "$$(rustup which --toolchain $(RUST_FLOOR) cargo)" check --locked --manifest-path $(POLARS_REFKIT_RUST) --all-targets --all-features

.PHONY: pyodide-lock pyodide-lock-check
pyodide-lock:
	$(UV_RUN) python scripts/pyodide_lock.py

pyodide-lock-check:
	$(UV_RUN) python scripts/pyodide_lock.py --check

.PHONY: clean-dist
clean-dist:
	rm -rf dist packages/refkit-core/dist packages/polars-refkit/dist

.PHONY: clean
clean:
	rm -rf \
		dist \
		dist-pyodide \
		wheels \
		build \
		target \
		crates/bibtex-tidy-rs/target \
		packages/polars-refkit/rust/target \
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
build: clean-dist
	uv build --package refkit-core --sdist --no-create-gitignore
	RUSTFLAGS="$(RUST_REMAP_FLAGS)" uv build --package refkit-core --wheel --no-create-gitignore
	uv build --package refkit --sdist --no-create-gitignore
	uv build --package refkit --wheel --no-create-gitignore
	uv build --package polars-refkit --sdist --no-create-gitignore
	RUSTFLAGS="$(RUST_REMAP_FLAGS)" uv build --package polars-refkit --wheel --no-create-gitignore
	$(UV_RUN) python -m scripts.normalize_wheel 'dist/*.whl'
	$(UV_RUN) python scripts/distribution_contract.py dist/*

.PHONY: lock
lock:
	uv lock --check

.PHONY: release-check
release-check:
	$(UV_RUN) python scripts/release_contract.py

.PHONY: architecture-check
architecture-check:
	$(UV_RUN) python scripts/architecture_contract.py

.PHONY: check
check: lock release-check architecture-check pyodide-lock-check lint typecheck test rust rust-floor build
