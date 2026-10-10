# RefKit Developer Documentation

These documents describe how to change, test, document, package, and release RefKit. The root README, `docs/`, and published package READMEs own installation and public API guidance.

## Set Up The Workspace

Install [Pixi](https://pixi.sh/latest/installation/) 0.81.0 or newer. The committed `pixi.lock` supplies the system tools on Linux x86_64 (glibc 2.28+) and macOS 13+ Intel/Apple Silicon. On Windows, clone and run the repository inside WSL2. macOS requires Apple's Command Line Tools for the SDK (`xcode-select --install`).

From the repository root:

```bash
pixi install --locked
pixi run --locked setup
pixi run --locked test
```

The setup task installs `wasm-bindgen-cli` 0.2.126 with its upstream Cargo lock under `.pixi/tools` (that version is unavailable from conda-forge on macOS). It then runs `make sync js-build` and installs the frozen documentation dependencies. uv builds both editable native adapters and provides Maturin. Python packages stay in `.venv`, and editors should use `.venv/bin/python`. Pixi forces uv to use its locked Python interpreter without downloading another runtime. uv builds one package at a time, and Cargo uses two jobs and limited development debug information to reduce Polars build costs.

To request full Rust debug information, run `pixi run --locked env CARGO_PROFILE_DEV_DEBUG=2 make refkit-develop`, substituting the affected Cargo or Make command. Release builds retain their existing profiles.

On Linux, the Rust linker wrapper uses the locked GCC with its built-in specs, avoiding Conda's automatic local-prefix RPATH in wheels. Activation supplies the library search path for native Rust test executables instead. The existing distribution contract rejects builder paths in archives.

On macOS, the Rust linker wrapper uses the locked Clang with its default Conda configuration disabled. It selects Apple's SDK from `SDKROOT`, or from `xcrun` when unset, so system libraries resolve from the SDK and wheels contain no automatic Pixi-prefix RPATH. Cargo runs native executables through `pixi-macos-runner.sh`, which supplies Python library lookup at runtime. The runner sets dyld's fallback path after its shell starts because macOS runtime protections strip inherited dyld variables from system shells. Pixi adds the repository's `scripts` directory to `PATH` so Cargo finds this runner even when the checkout path contains spaces.

Use `pixi run --locked <command>` for individual commands, or `pixi shell --locked` for an interactive session with the same tools. For example:

```bash
pixi run --locked make refkit-develop
pixi run --locked rebuild
pixi run --locked docs
```

## Browser Prerequisites And Complete Gate

Download the Chromium, Firefox and WebKit revisions recorded by the locked Playwright package after setup:

```bash
pixi run --locked browsers
```

Browser executables live under `.pixi/browsers`. On Linux, Playwright also needs host GUI/media libraries. On a supported Debian/Ubuntu host, install them explicitly with `pixi run --locked npm --prefix packages/refkit-js exec -- playwright install-deps chromium firefox webkit` (uses sudo/apt). These host libraries, the OS kernel, and Apple's SDK are outside the Pixi lock; the feature-development environment does not need browser libraries. Pyodide cross-builds use the separate pinned `.github/actions/setup-pyodide` workflow.

Run `pixi run --locked check` before handing off a repository change. It invokes the existing `make check` gate, validating locks, release metadata, architecture, documentation, Pyodide inputs, Python and Rust code, tests, dependency policy, the Rust 1.95 floor, and built distributions. CI also exercises the locked Pixi setup on Linux and macOS; the existing artifact lanes continue to test other Python, Node and platform combinations.

## Update Environment Dependencies

`pixi.toml` declares system-tool requirements and `pixi.lock` records exact packages and hashes for all supported platforms. Change the manifest and run `pixi lock` for intentional updates, then commit both files. Keep Rust at the shared floor, pnpm aligned with `docs/package.json`, audit tools aligned with the Makefile, and the binding generator aligned with the Wasm Cargo lock. Preserve `uv.lock`, all three Cargo locks, and the npm/pnpm locks as the owners of their respective package graphs. Do not duplicate Python dependencies in Pixi.

All Make targets remain usable with manually installed prerequisites: uv, Rust/Cargo 1.95 or newer, Node 24 or newer, pnpm 11.10.0, the Wasm target and matching binding generator, and the pinned audit tools. Pixi supplies these tools consistently for daily development.

## Find The Right Document

| Task | Read |
| --- | --- |
| Place behavior in the correct crate or package | [Architecture](architecture.md) |
| Trace a capability across owners, interfaces, and tests | [Capability map](capabilities.md) |
| Preserve parser, raw edit, tidy, and render invariants | [Core semantics](core-semantics.md) |
| Apply Rust lint, suppression, dependency, and compiler policy | [Rust quality](rust-quality.md) |
| Change Python, Polars, or Pyodide host boundaries | [Adapter contracts](adapters.md) |
| Set up an edit and rebuild loop | [Development workflow](development.md) |
| Choose the test boundary, installed `refkit-tests` probes, and completion gate | [Testing](testing.md) |
| Build and validate the VitePress site | [Documentation site](documentation.md) |
| Update locks, stubs, generated state, or repository policy | [Repository contracts](repository-contracts.md) |
| Build wheels, validate Pyodide, or prepare a release | [Packaging and release](packaging-and-release.md) |
| Measure a capability or interpret benchmark output | [Benchmarks](benchmarks.md) |
| Choose an equivalent reference-package workflow | [Bibliography package boundaries](feature-matrix.md) |
| Diagnose local build, plugin, runtime, or docs failures | [Development troubleshooting](troubleshooting.md) |

The nearest `AGENTS.md` owns short, local invariants. Put explanations and cross-package workflows in this directory. When an invariant can be checked from source or an artifact, extend the corresponding script under `scripts/` and keep it in `make check`.

## Documentation Boundary

End-user documentation covers installation, supported inputs, public APIs, return values, errors, and runtime use. Developer documentation covers source ownership, dependency direction, build tools, test topology, benchmarks, CI, packaging, and release procedures.

`make docs-check` validates Markdown boundaries, installs the locked VitePress dependencies, typechecks and builds the site, then verifies routes, public assets, metadata, and local links. `scripts/distribution_contract.py` verifies that built distributions exclude this directory.
