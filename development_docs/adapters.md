# Adapter Contracts

Adapters translate host values and lifecycle into the portable core API. They depend inward and keep host-specific policy outside bibliography semantics.

## Python Adapter

`packages/refkit/rust` maps core types into PyO3 classes and exceptions. `packages/refkit/src/refkit/__init__.py` is the composition root that verifies the native version and adds path-based helpers.

| Module | Boundary |
| --- | --- |
| `filesystem.rs` | Reads bytes, selects file formats, attaches decode context, and writes files. |
| `library.rs` and `entry.rs` | Maps normalized records, selectors, projection, diagnostics, and key errors. |
| `style.rs` | Loads bundled or explicit styles and validates `Locale` objects. |
| `citation.rs` and `document.rs` | Validates Python citation shapes and maps ordered rendering. |
| `rendered.rs` | Converts text, HTML, and typed nodes into Python values. |
| `raw.rs` | Owns GIL-bound live raw views and field mutation. |
| `tidy.rs` | Parses option forms and maps warnings and structured syntax errors. |
| `module.rs` | Registers `refkit._native` and runtime metadata. |

The extension declares that it uses the Python Global Interpreter Lock. Core-heavy work detaches after Python inputs have become Rust-owned values. Live raw handles use `Rc<RefCell<RawDocument>>`, remain bound to their creating Python thread, and clone the raw state before a write or tidy operation detaches.

`Style.id` is an adapter source label. It is a bundled name, path string, or `xml`. A plain locale string passed to `Document` bypasses `Locale.load` validation and is forwarded to the renderer.

Update native registration, `__init__.py`, `_native.pyi`, `__init__.pyi`, runtime dictionaries in `types.py`, `__all__`, runtime signature tests, public reference, and installed-wheel tests together.

### Code-mode capability

`refkit.agent` is a lazy instruction and resource adapter over the public Python API. The `marimo.agent.capability` entry point maps `refkit` to that module. Marimo reads the entry-point name and module during discovery, then a code-mode agent imports the module and calls `help` when the capability is relevant.

The agent module adds no bibliography operations. Its dynamic documentation routes agents to `Library`, `Document`, `BibDocument`, tidy functions, the public documentation map, and the version-matched Agent Skill installed with the distribution. `agent_plugin()` and `agent_skill()` expose those resources through `agent-plugins`.

Each package owns its installed skill and capability entry point. Keep `refkit.agent` out of the package root so importing `refkit` does not import agent tooling. Test discovery metadata, module loading, dynamic help, resource lookup, and root-import laziness together.

## Polars Adapter

The Polars call path is:

```text
Python builder or pl.Expr.refkit method
  -> static argument conversion
  -> register_plugin_function
  -> serde keyword record
  -> Rust expression
  -> refkit-core capability
  -> declared Polars dtype
```

Importing `polars_refkit` registers the expression namespace. The native library path resolves inside the installed package when Polars executes an expression.

Static recovery and tidy option validation happens while Python builds `pl.Expr`. Input dtype checks, projection fields, style loading, broadcasting, parsing, rendering, formatting, and output construction happen when an eager query runs or a lazy plan collects.

Two-input render expressions accept equal lengths or a singleton on either side. A singleton source is parsed once inside that expression. Separate expressions do not share parsed libraries.

Value expressions map row-local input, parse, missing-key, and render failures to null. Query-shape failures abort execution. Report structs preserve structured parser diagnostics, renderer error codes, formatter warnings, and rename records. Maintain exact outer and inner validity across scalar, list, and struct outputs.

Bundled styles are cached process-wide by lowercase name. Locale strings are forwarded without archive validation. Keep the user reference precise until locale validation ownership changes.

Rendering builders choose a known plugin and dtype from `output`. Formatting builders accept one typed options dictionary. Add a capability by updating its builder, namespace, exports, stub, transport, Rust registration, declared dtype, eager/lazy tests, installed probe, and public reference together.

## Pyodide Boundary

PyEmscripten wheels reuse the Python composition and Polars adapter surfaces. Shared runtime tests and probes in `packages/refkit-tests/src/refkit_tests` establish installed imports, parsing, rendering, raw editing, formatting, Polars callbacks, and row failures on CPython and the pinned Pyodide xbuild environment. CI installs the support wheel alongside each candidate package and runs it outside the checkout.

Keep the Python Polars package and Rust plugin application binary interface family aligned through `.github/pyodide/runtime.json` and `make pyodide-lock-check`.
