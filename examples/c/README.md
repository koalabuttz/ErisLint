# Fork-specific C example

This configuration explicitly treats both `.c` and `.h` as C. From the repository
root, run the local checks (no provider key needed):

```sh
cargo run --locked -- --config examples/c/erislint.json --check-config
cargo run --locked -- --config examples/c/erislint.json --dry-run
```

The dry-run contains the definition and header prototype. Includes are not loaded
by the parser: both files are selected independently, and preprocessor conditions
remain unevaluated. The prototype has no body; a live evaluator may need to choose
`unknown`. These commands do not perform live semantic lint. Live runs send the
selected source/context to Jev and require separately authorized provider access.

See [the C contract](../../docs/fork/C-DESIGN.md) for configuration, limitations,
and supported target kinds. SPDX-License-Identifier: AGPL-3.0-only.
