# Fork-specific Python example

From the repository root, validate and inspect requests locally:

```sh
cargo run --locked -- --config examples/python/erislint.json --check-config
cargo run --locked -- --config examples/python/erislint.json --dry-run
```

The example yields one class and two function targets (an async method and its
nested function). Its missing import and decorator are never loaded or executed.
These are offline configuration/request checks, not live semantic lint. Live
runs send the selected source/context to Jev and require provider access.

Configuration version 2 requires both `python_files` and Python-scoped rules.
Global `include`/`exclude` filters apply first; the default include remains Rust
only. Only UTF-8 `.py` files are accepted, not notebooks, `.pyi` stubs, bytecode,
or alternate source encodings. Overlapping C/Python file selections fail.
Methods and async functions use `kind: "function"`; classes use `kind: "class"`.
Use `kind: "file"` for modules without selected definitions. Active rules with
no matching targets fail; explicit rule filters and disabled rules can skip files.

The exact supported grammar is `tree-sitter-python` 0.25.0, with explicit rejection
of Python 2 print/exec/backtick syntax and empty suites. This is not a CPython
version/conformance checker. Lambda expressions are preserved in source but are
not separate named-function targets. Parse errors and unsupported grammar fail.
Comments are retained as file-wide records; decorators, docstrings, annotations,
bases and nesting retain raw source and spans. Docstrings are literal source,
not decoded runtime values. Bytes/interpolated strings are not labeled docstrings.
No imports, types, symbols, decorators, metaclasses or runtime bindings resolve.

CLI `--stdin-file` snapshots work with an existing selected `.py` file, and
`--target-start` selects a function by its name's UTF-8 byte offset. VS Code
activation remains Rust-only. See [the design](../../docs/fork/PYTHON-DESIGN.md).

SPDX-License-Identifier: AGPL-3.0-only.
