<!-- Fork-specific addition. SPDX-License-Identifier: AGPL-3.0-only -->

# C milestone checkpoint

This records the initial C milestone. See [review fixes](C-REVIEW-FIXES.md)
for the subsequent offline regression checkpoint.

The opt-in C milestone is implemented after independent review of the published
Rust checkpoint `c765486f4a7c437dc91d5fd8d0070a58c573bdf0`. This work stops before
Python, assembly, new VS Code language activation, or an upstream PR.

## Delivered behavior

- Version 1 remains Rust-only, including its original generated schema bytes.
  Version 2 requires explicit `c_files` and C-scoped rules. Headers are selected
  deliberately as C. Unscoped rules remain Rust rules; v1 cannot inherit C
  enablement from v2. Separate v2 schemas and a [runnable C example](../../examples/c/README.md)
  document the opt-in contract.
- Tree-sitter C extracts function definitions, unambiguous prototypes and file
  targets. Function-pointer variables are not mistaken for functions. Source,
  comments, raw declarators/parameters, UTF-8 name and node spans, preprocessor
  records and conditional ancestry are retained.
- C requests explicitly mark incomplete analysis: no macro expansion, include
  loading, conditional evaluation, symbol resolution or type inference. Syntax
  errors, missing nodes and unsupported declaration shapes fail operationally.
  Active C rules without a matching target do not silently pass; file rules can
  inspect files without functions. Explicit filters/off overrides may skip input.
- Prepared-parser caching includes directory, language and parser options.
  Mixed Rust/C directories retain the correct parser regardless of file/input
  order. C-only files do not read Cargo metadata. Shared batching, policies,
  diagnostics and CLI editor snapshots are reused.

## Verification

All final checks passed: `cargo fmt --check`, `cargo test --locked` (56 tests),
`cargo clippy --locked --all-targets -- -D warnings`, TypeScript compilation and
six protocol tests. Thirteen C contracts cover metadata/spans, prototypes and
pointer variables, unsupported/malformed syntax, headers, language-scoped rules,
version/inheritance validation, mixed-directory/order behavior, shared diagnostics,
unsaved editor input and full example request bytes. Added Rust regression tests
cover multiple directories/editions, reversed/duplicate explicit paths and Unix
symlink canonicalization/containment.

Both legacy schema outputs and all frozen Rust requests remain byte-for-byte
unchanged. Separate v2 schema files and complete C example request bytes are
also checked. Whole-project offline self-lint validates one unchanged rule and
builds 156 questions across 19 files. The modified binary's output matches the
unchanged upstream binary on those same current Rust sources. The C example's
offline config and dry-run pass with two files and two targets.

## Actual unchanged-upstream live self-lint

The original binary built from upstream
`f04d016461b66b38d46647fb20762fb67bda350a` reviewed the C implementation's Rust
code using the unchanged original config/rule and one concurrent request.
Coverage: `src/adapter.rs`, `src/c.rs`, `src/config.rs`, `src/config/legacy.rs`,
`src/main.rs`, and `src/runner.rs`.

- 45 evaluations/questions, 45 attempts, zero retries.
- Model `jev-1.13.0`: 43 `simple`, two `insufficient_context`.
- Zero warnings/errors; uncertainty concerned the two config version-default
  helpers. No diagnostic-driven changes or diagnostic false positives were
  identified, and no thresholds were weakened.

This is live review of the Rust implementation, not live validation of custom C
rules or a correctness proof. Actual billed tokens are not exposed by the original
CLI. Credentials and private accounting remain outside public artifacts.

## Limits for independent review

This is source inspection for the pinned C grammar, not ISO conformance checking,
C++ support, macro-expanded analysis or a compiler substitute. Some real C code
requires preprocessing before it can be parsed; this milestone reports failures
rather than guessing. C-specific struct/enum targets and inferred return types
are not implemented. Native dependency compilation needs a build toolchain;
analysis never runs a compiler or the analyzed project.

Version-2 schemas describe the added fields; `--check-config` also enforces the
cross-field/version constraints. VS Code activation remains Rust-only, and its
extension-host test remains unavailable without `code`; protocol tests passed.
No hosted CI is configured in this checkout. The next step is independent C
implementation review before Python work.
