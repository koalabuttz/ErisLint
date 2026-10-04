<!-- Fork-specific addition. SPDX-License-Identifier: AGPL-3.0-only -->

# Standalone-language milestone complete

The planned Rust-preserving extraction, C adapter, Python adapter and two bounded
standalone assembly profiles are complete at
[`f9dfb56c3f355c7c4db4e1974594fb19e4e47a8a`](https://github.com/koalabuttz/ErisLint/commit/f9dfb56c3f355c7c4db4e1974594fb19e4e47a8a).
This completion change is documentation only. No merge, upstream pull request,
new feature or change to `main` is part of this checkpoint.

Eriskii's upstream history, branding, copyright and AGPL-3.0-only license remain
intact. The public fork retains upstream/main baseline
`f04d016461b66b38d46647fb20762fb67bda350a`. Fork additions and limits are labeled.
Use the [CLI getting-started guide](GETTING-STARTED.md) to try the offline examples.

## Checkpoints and review trail

| Stage | Evidence and review |
| --- | --- |
| Baseline and Rust extraction | [Original baseline](BASELINE.md), [carrier/dispatch extraction](RUST-EXTRACTION.md), [publication and Rust live review](PUBLICATION.md) |
| C | [C checkpoint](C-CHECKPOINT.md), [independent review fixes](C-REVIEW-FIXES.md), [source contract](C-DESIGN.md) |
| Python | [Python checkpoint](PYTHON-CHECKPOINT.md), [review fixes and final docstring review](PYTHON-REVIEW-FIXES.md), [grammar contract](PYTHON-DESIGN.md) |
| x86 GAS AT&T32 | [Implementation and two reproduced review fixes](ASSEMBLY-X86-CHECKPOINT.md); independently accepted at [`dd56a2c`](https://github.com/koalabuttz/ErisLint/commit/dd56a2cb34927d6e9391f221e02b98daa5ee9a28) |
| LLVM-MOS generic/C64 intent | [MOS checkpoint, references and fixture gates](ASSEMBLY-MOS-CHECKPOINT.md); independent source review accepted the exact implementation above with no blocking defect |

Historical reports describe their stage's scope and counts; they do not imply
that later adapters or changes were included in earlier live reviews.

## Final verification and limits

The tested implementation passes 122 Rust tests, six extension protocol tests,
formatting and all-targets Clippy with warnings denied. Frozen v1/v2/v3 schemas,
representative legacy errors, Rust/C/Python request bytes and twelve assembly
file/region context snapshots pass. Original self-lint configuration and policy
are unchanged. Offline whole-project self-lint constructs 277 questions across
28 Rust files, byte-identical to the pristine upstream binary on those sources.
Offline validation and dry-run are not live semantic lint.

The C and Python adapters preserve source-level metadata and explicit incomplete
context; they do not execute analyzed files. Assembly is a bounded tokenizer and
region reviewer, not an assembler or CPU validator. Macro/CPP expansion, symbols,
relocations, instruction legality, 6510/undocumented features, timing, linking and
runtime effects remain unresolved. No embedded Rust assembly extraction or new
editor-language activation is included. See each source contract for deliberate
unsupported syntax and operational errors.

No hosted CI is configured. The six protocol tests passed locally, but extension
host testing remains unavailable without the `code` executable. Independent
review and model judgments are evidence, not proofs of semantic correctness.

## Final two-function live self-review

The pristine upstream binary from `f04d016461b66b38d46647fb20762fb67bda350a`
reviewed exactly two functions in `src/assembly/lexer.rs` at the tested
implementation commit: `Scanner::next` (profile-aware lexical dispatch) and
`Scanner::flush` (raw statement/structural handling). Function-name byte selection
and unchanged source snapshots were verified offline before the requests. The
original configuration and function-simplicity rule were used without changes,
at one concurrent request, with a separately authorized bounded reservation.

| Function | Model | Selected answer | Confidence | Diagnostics |
| --- | --- | --- | ---: | --- |
| `Scanner::next` | `jev-1.13.0` | `simple` | 0.85 | None |
| `Scanner::flush` | `jev-1.13.0` | `simple` | 0.69 | None |

The batch completed two evaluations in two attempts, with zero retries, warnings
or errors. Both results were reviewed; no code change was warranted. No rule or
threshold was weakened, and no diagnostic false positive was observed. No further
live calls followed this batch.

This is two-function Rust implementation simplicity coverage, **not whole-assembly
review**, live evaluation of assembly-language rules, or validation of assembler
semantics. Earlier live coverage is recorded in the linked stage reports. The
unchanged upstream CLI omits billed token usage; private credentials, reservations
and accounting remain outside the public repository.

SHA-256 evidence:

| Artifact | SHA-256 |
| --- | --- |
| Pristine upstream binary | `c916ce1cc8e63c2cc5ef95cf1f9ddf22a41af1d3974d4fea6e0934c428a8bcef` |
| Original `erislint.json` | `e808ea50c173c67159d25919a505128ecda0314bb2a3fe88a999180ea6f956a1` |
| Original function-simplicity rule | `44db44f3f095623098dd3958c69c19a4178c1c2e602239dba14a30128398c6ef` |
| Reviewed `src/assembly/lexer.rs` | `aae5866bdaf6348c7c3cda7d855cb26c9bedb7c12eb00f64926fb7dfaab69f8d` |
