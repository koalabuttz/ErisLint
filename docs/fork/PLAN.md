<!-- Fork-specific addition. SPDX-License-Identifier: AGPL-3.0-only -->

# Fork development plan

This is a fork-specific roadmap for ErisLint, created by
[Eriskii](https://github.com/Eriskii/ErisLint). Upstream baseline:
`f04d016461b66b38d46647fb20762fb67bda350a`. Preserve upstream history,
branding, copyright notices, authorship, and AGPL-3.0-only licensing. New
language support described here is planned, not implemented or endorsed upstream.

## Phases and acceptance gates

1. **Baseline and common interfaces.** Record existing checks and offline
   self-lint, then capture Rust request/diagnostic compatibility fixtures. Extract
   common source types in a refactor-only commit; follow with a Rust adapter seam
   in another refactor-only commit. Existing Rust configs, discovery, edition
   selection, request JSON, ordering, spans, CLI exit codes, and editor snapshots
   must behave identically. Stop for review of this plan before either refactor.
2. **C source adapter.** Initially support explicitly selected C files (`.c` and
   C-mode `.h`), functions and file targets; reject C++ and ambiguous language
   inference. Evaluate a lossless parser before selecting a dependency. Preserve
   comments, declarations, original source spans, preprocessor directives and
   conditional context. Do not run a compiler or silently expand macros. Expose
   unresolved includes, macro-dependent syntax and conditional branches as
   incomplete context. Add opt-in config/selector support and C-specific rules
   separately; keep Rust defaults and rules unchanged. Gate on syntax fixtures,
   Unicode/CRLF spans, headers, preprocessing limitations, malformed input,
   mixed-language selection and offline request snapshots.
3. **Python adapter.** Support `.py`, functions (including async), classes and
   file targets with an explicit supported grammar version. Preserve decorators,
   docstrings, comments, indentation, annotations and nesting. Never execute or
   import analyzed code. Dynamic imports, runtime types and metaprogramming stay
   unresolved. Gate on grammar/error fixtures, source offsets, nested/decorated
   definitions, language-specific metadata and mixed-language config tests.
4. **One assembly scope.** Proposed initial scope: x86-64 GNU assembler, AT&T
   syntax, unpreprocessed `.s` files, file and label/block targets. Document this
   choice before implementation. Reject or flag Intel syntax, `.S` preprocessing,
   macro expansion and other architectures as unsupported/incomplete. Do not
   infer function boundaries solely from labels, or claim ABI, control-flow or
   instruction correctness. Retain directives, labels, comments and raw operands.
   Expand only after focused fixtures and explicit review.

Each stage must pass the unchanged Rust suite plus its own contract tests before
adding another language. Keep editor activation Rust-only until an explicit,
separately tested editor feature change. Source-level model judgments are review
leads, not semantic correctness proofs.

## Minimal extraction proposal

The current coupling is concentrated in `src/rust.rs` and `src/runner.rs`:
`Span`, `TargetKind`, `Target::input`, parser extraction, and edition discovery.
`src/config.rs` also imports `rust::TargetKind`.

- Move `Span`, the existing `TargetKind` values, and the parser-independent target
  carrier/context assembly to a small `source` module. Retain public re-exports
  from `rust` to preserve existing callers. Keep Rust AST classification,
  `ra_ap_syntax::TextRange` conversion, descriptions and Cargo edition handling
  in `rust`. Do not normalize Rust metadata into a lossy universal schema.
- Add a small internal adapter dispatch boundary for path acceptance, per-file
  parse options and target extraction, initially with Rust as its sole variant.
  Both disk and editor input paths use it. Preserve edition caching and existing
  errors. Prefer a concrete enum over a plugin registry or trait hierarchy until
  a second implementation demonstrates the need.
- The first refactors change no serialized fields. Language selection, new target
  kinds, schema changes and explicit completeness metadata belong in feature
  commits. For new adapters, distinguish absent, unknown and unsupported facts;
  retain original source and language-specific metadata. A parse failure must
  never silently become an empty successful lint result.

Compatibility evidence should compare full Rust dry-run JSON before/after, plus
focused fixtures covering all target kinds, context modes, editions, comments,
Unicode/CRLF, parser failures, batching and unsaved editor input. Preserve the
locked rust-analyzer/Unicode dependency pairing.

## Iterative commits and self-lint

Use focused commits with problem/behavior summaries and actual check outcomes in
commit bodies. Suggested sequence: plan; baseline evidence; compatibility
fixtures; common types refactor; Rust dispatch refactor; one language feature at
a time. Keep refactors separate from features and dependency changes. Review
staged diffs for unrelated files and generated output before committing. Never
rewrite upstream history or open an upstream PR as part of this work.

For every source change, run `cargo fmt --check`, `cargo test --locked`, and
`cargo clippy --locked --all-targets -- -D warnings`, plus affected extension
checks. Use the existing repository rules with explicit `--config erislint.json`:
first `--check-config`, then `--dry-run` on affected source (whole-project at
stage gates). These are offline validation and request construction, not live
semantic lint or passing model findings. Preserve rule thresholds; review
findings before editing code and record useful findings and false positives.

Live self-lint additionally requires an authorized managed connection, verified
current official pricing, and a project-specific cumulative budget reservation
that includes every attempt and retry. Reserve worst-case cost before sending;
stop when cost cannot be bounded or authorization is unavailable. Concurrency
limits alone do not enforce spending. Keep secrets, private account data and
spending records outside public artifacts. Offline work can continue when live
checks are unavailable.
