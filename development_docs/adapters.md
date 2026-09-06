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

The extension declares that it uses the Python Global Interpreter Lock. Core-heavy work detaches after Python inputs have become Rust-owned values. Live raw handles use `Rc<RefCell<RawDocument>>`, remain unsendable, and clone the raw state before a write or tidy operation detaches.

`Style.id` is an adapter source label. It is a bundled name, path string, or `xml`. A plain locale string passed to `Document` bypasses `Locale.load` validation and is forwarded to the renderer.

Update native registration, `__init__.py`, `_native.pyi`, `__init__.pyi`, typed dictionaries, `__all__`, runtime signature tests, public reference, and installed-wheel tests together.

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

Value expressions map row-local input, parse, missing-key, and render failures to null. Query-shape failures abort execution. Report structs preserve parser or formatter details. Maintain exact outer and inner validity across scalar, list, and struct outputs.

Bundled styles are cached process-wide by lowercase name. Locale strings are forwarded without archive validation. Keep the user reference precise until locale validation ownership changes.

Add an expression by updating the builder, namespace, exports, stub, static option converter, serde record, Rust registration, dtype constructor, default alias, eager and lazy tests, null tests, broadcasting tests, installed-wheel tests, and public docs.

## Pyodide Boundary

PyEmscripten wheels reuse the Python composition and Polars adapter surfaces. Package-local mock tests establish import parity. The runtime tests under `.github/pyodide` establish WebAssembly imports, parsing, rendering, raw editing, formatting, Polars callbacks, and row failures against the pinned xbuild environment.

Keep the Python Polars package and Rust plugin application binary interface family aligned through `.github/pyodide/runtime.json` and `make pyodide-lock-check`.
