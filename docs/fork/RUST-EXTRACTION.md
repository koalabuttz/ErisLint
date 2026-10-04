<!-- Fork-specific addition. SPDX-License-Identifier: AGPL-3.0-only -->

# Rust extraction checkpoint

This is the historical pre-publication extraction report. The reviewed
first-stage refactors were subsequently published and live-checked; see
[the publication and live-check update](PUBLICATION.md). C, Python and
assembly support have not begun. Architecture/dialect selection remains pending.
The original [baseline](BASELINE.md) describes the earlier unmodified source;
this report records the subsequent compatibility and extraction work.

## Focused changes

1. `7a3c3b4092d0488cffde89d6596f1cd9052d5815`: compatibility fixtures captured
   from the upstream binary before production changes; eight new Rust tests and
   one new editor protocol test.
2. `4f48c1957d59f3a7847fbebf3842edbc1fabdb91`: shared `source` carriers and
   context assembly. Preserve `rust::{Span, Target, TargetKind}` re-exports and
   private state/enclosing fields; construction is crate-private. Rust retains
   AST classification/traversal, descriptions, fallback names, body detection,
   source-range conversion and edition discovery.
3. The commit containing this report introduces a private Rust-only dispatch
   module for extension selection, parser preparation and extraction. The runner
   keeps ignore walking, containment/filtering and sorted deduplication. A map
   carries the adapter selected before entry canonicalization with each disk
   path. The existing per-directory cache now holds prepared Rust parsers.
   Editor input still canonicalizes first, requires an existing file, applies
   its distinct filters and resolves edition only after those filters.

No config defaults, selector values, serialized fields, rules, lockfile or
provider behavior changed. Before adding another language, revisit cache keys
and parser-option typing explicitly; this cache deliberately preserves the
single-language behavior of upstream.

## Validation at each stage

All three stages passed formatting, 41 Rust tests, all-targets Clippy with
warnings denied, TypeScript compilation and six protocol tests. Each validated
one self-lint rule and completed an offline whole-project dry-run:

| Stage | Source files | Evaluations/questions |
| --- | ---: | ---: |
| Compatibility fixtures | 14 | 120 |
| Shared carriers | 15 | 121 |
| Rust dispatch | 16 | 124 |

Counts grow because tests and extracted functions are themselves included in
self-lint. They are request counts, not live findings.

The tests compare full deterministic stdout bytes for disk input, unsaved editor
input and exact target selection against pre-refactor snapshots. Both generated
schemas match their unchanged upstream files byte-for-byte. Compile coverage
uses the old public imports. Coverage also freezes name span versus node range,
CRLF, non-BMP prefixes, Unicode name byte boundaries, UTF-16 conversion, all
existing target kinds/context modes, batching, editions and overrides, parser
failures, input/error precedence, and disk/editor filtering distinctions.

Two initial fixture assumptions were corrected against upstream: leading
comments belong to the full function node range, and the fixture's async
function is rejected in Rust 2015. No production behavior was changed to make
these tests pass. An initial Clippy failure in test code was corrected before
the fixture commit.

For each production refactor, the preserved upstream binary and new binary also
ran whole-project dry-run against identical current sources, and `cmp` passed.
This comparison is separate from comparing different self-linted source trees.
Result SHA-256 values:

- Shared-carrier whole-project output:
  `8eb03a64bfdeb78a5498929f60474374fdc7c7cb271a1fd9debe376ee5ce5054`
- Rust-dispatch whole-project output:
  `528d9051506e1145cca43f9a683dcfa2a30ca4ed99c914a011889c15571f2b81`
- Frozen disk/editor request fixture:
  `c60ab081a1ac4c70a68031711a884277a3ceb2de4873acf7254df923f4e56f46`
- Frozen selected-function request fixture:
  `636891c2e6448b49d00de64d19e8d2406db5c20f0433bee4faa5549077f09126`

## Limits recorded before publication

The VS Code extension-host check remains unrun because `code` is absent; protocol
checks do not replace it. There were no live Jev calls or semantic lint findings.
Managed provider access, pricing and cumulative budget enforcement remain
unverified. Fork creation remains blocked by the previously recorded GitHub 403;
creation was not retried, and no alternate publication or upstream PR occurred.
Work stops at this Rust-only checkpoint for review before C features.
