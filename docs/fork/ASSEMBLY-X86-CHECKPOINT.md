# Fork-specific x86 assembly implementation checkpoint

This milestone extends the public ErisLint fork while preserving Eriskii's
upstream attribution and AGPL-3.0-only license. Its design baseline is
`fe38735f310558c04f3e8b033068c8b6641425b8`; upstream/main remain at
`f04d016461b66b38d46647fb20762fb67bda350a`.

## Implemented scope

Version 3 adds explicit assembly source options and versioned rule imports with
declaring-file provenance. Version-2 schema carriers were frozen in a separate
refactor before adding fields. Version 1/2 schema commands and Rust/C/Python
defaults remain unchanged. Older documents reject new fields even when null and
cannot inherit a version-3 document.

A shared lossless text-record layer supplies UTF-8 byte ranges and Unicode
scalar positions. The first profile, `x86-gas-att32`, supports standalone `.s`
and `.S` source with explicit preprocessing/slash modes. Bounded tokenization
retains comments, literals, labels, directives, assignments and opaque statements;
balanced macros, repetitions and conditionals remain unresolved. CPP logical
directives remain opaque. File targets and explicitly marked nonempty regions
retain essential unresolved dependencies in every context mode.

Assembly questions require the fixed `insufficient_context` answer description.
Selecting it emits a fixed warning independently of substantive policies and
severity overrides. Explicitly disabled rules are skipped. Source matching,
cache keys and unsaved-buffer handling distinguish profile options and languages.

See the [example](../../examples/assembly/README.md) and
[normative contract](ASSEMBLY-CONTRACT-DETAILS.md) for accepted lexical forms,
region grammar and conservative rejections.

## Verification actually performed

- `cargo fmt --check`, `cargo test --locked` (104 tests) and
  `cargo clippy --locked --all-targets -- -D warnings`: pass.
- Extension TypeScript compilation and `npm test`: six protocol tests pass.
- The assembly example passes offline configuration validation. Complete request
  bytes are frozen for both target kinds in all three context modes (six
  evaluations). These are x86 snapshots; no MOS snapshot coverage is claimed.
- Twenty-three assembly integration tests cover version/inheritance/import
  provenance, uncertainty policies, lexical rejection, exact region ranges,
  Unicode/CRLF, inert markers, balanced and unsupported states, global scope,
  mixed-language/option selection, unsaved input and schemas.
- Nine legacy CLI error fixtures match the exact pre-v3 binary output, including
  line/column details. Existing Rust, C and Python request/schema fixtures pass.
- Original self-lint configuration validates offline. The dry-run constructs
  254 questions across 27 Rust files and matches the pristine upstream binary
  byte-for-byte on those same current sources. Original config/rules are unchanged.
- `git diff --check`: pass. All assembly source fixtures are synthetic public
  examples; no analyzed assembler or preprocessor was executed.

Offline validation and request construction are not semantic lint. No live Jev
calls were made for this milestone, and it consumed no additional project
spending reservation. No hosted CI is configured; extension-host testing remains
unavailable without `code`.

## Limits and next gate

### Independent review fixes

Two offline reproductions exposed gaps after the initial checkpoint. An invalid
base document's assembly options could disappear when a child replaced them;
source entries now compile under each declaring document before replacement,
with its path included in errors. Final source/rule pairing still occurs after
merge, so valid split declarations and replacements remain supported. Tests
cover omitted/null slash mode, empty patterns, invalid globs and unsupported
profiles in replaced bases, including later sibling replacement. Frozen legacy
error fixtures remain unchanged.

Block comments could hide a bare carriage return. Their bodies now use the
existing CRLF validation before record emission. Regressions cover bare CR in
comments and at neighboring boundaries, accepted CRLF, adjacent tokens and
opaque continued CPP records. Both regressions failed before their fixes.

After these fixes, all 107 Rust tests (26 assembly tests), six protocol tests,
formatting and strict Clippy pass. Offline self-lint validates the original
configuration and constructs 257 questions across 27 files, matching pristine
upstream request bytes on the same sources. No live calls were made.

### Remaining scope

This is a bounded source-review scanner, not a GAS parser or instruction
validator. Assembler acceptance, symbol/operand resolution, macro/CPP expansion,
CPU behavior, linking and runtime correctness remain unknown. Labels are not
functions. Unsupported lexical forms and recognized structural/state conflicts
fail conservatively; opaque operands and arbitrary directive arguments are not
fully syntax-validated. Assembly editor activation and embedded Rust assembly
are deferred.

`mos-llvm-c64` is recognized only to report that it is unimplemented. Stop here
for independent x86 review before adding that profile. Further live self-lint
requires separately authorized spending and valid pricing.
