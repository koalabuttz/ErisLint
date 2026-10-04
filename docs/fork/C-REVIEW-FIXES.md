<!-- Fork-specific addition. SPDX-License-Identifier: AGPL-3.0-only -->

# C review fixes

Independent review found three reproducible defects after the initial C
checkpoint. Each correction has its own focused commit and regression tests:

- Old-style function parameter declarations were also emitted as function
  targets. Declarations directly beneath a function definition are now excluded;
  genuine file-scope and block-scope prototypes remain eligible.
- File targets omitted leading whitespace, and whitespace-only input produced
  empty source. File source and spans now cover every original byte, including
  unsaved editor snapshots. Function spans remain unchanged.
- Broad `c_files` globs validated unsupported extensions before global scope
  filters. C-mode disk discovery and editor snapshots now apply global filters
  first. Selected unsupported extensions still fail, path existence and
  containment checks remain, and legacy Rust validation ordering is preserved.

All three failures were reproduced before correction. Final offline verification
passes: 62 Rust tests (including 19 C contracts), six extension protocol tests,
TypeScript compilation, formatting, strict Clippy, and configuration validation.
Whole-project dry-run builds 163 questions across 19 files and matches the
unchanged upstream binary byte-for-byte on the same current Rust sources. Frozen
Rust requests, legacy schemas, and C example request coverage remain passing.

No additional live provider calls were made for these fixes. The earlier live
results describe the initial implementation only. No hosted CI is configured;
extension-host testing remains unavailable without `code`. Python and assembly
remain future milestones.
