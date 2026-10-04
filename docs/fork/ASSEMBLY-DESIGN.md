<!-- Fork-specific addition. SPDX-License-Identifier: AGPL-3.0-only -->

# Assembly source-review proposal

Status: design for review, not implemented. This extends Eriskii's ErisLint through
one shared assembly layer and two explicit profiles. Preserve upstream credit,
license, history and Rust/C/Python behavior. No private project sources, artifacts,
build configuration or repository identifiers belong in fixtures or public docs.

## Profiles and verified basis

| Proposed profile | Declared CPU/platform intent | Source dialect |
| --- | --- | --- |
| `x86-gas-att32` | i386-compatible, 32-bit mode; no ABI inferred | GNU assembler AT&T syntax, `.code32`, GNU-style macro/directive records |
| `mos-llvm-c64` | C64/6510 intent; no hardware behavior model | LLVM-MOS default/generic assembler syntax; MOS operands and addressing notation |

CPU intent, assembler dialect, input preprocessing and platform are separate
metadata. A C64 profile is not a claim that LLVM accepts a `6510` CPU option, nor
that all 65xx extensions are compatible. The inspected LLVM-MOS device table has
separate `mos6502` and `mos6502x` families; undocumented instructions are not enabled
or validated by this proposal. [Device definitions](https://github.com/llvm-mos/llvm-mos/blob/06bc967d2668c7c11c4d6eb43a6aed1f99ad258b/llvm/lib/Target/MOS/MOSDevices.td).

The rust-mos project explicitly depends on llvm-mos and llvm-mos-sdk. Its use of
LLVM does not make its assembly x86 GAS. [rust-mos target notes](https://github.com/mrk-its/rust-mos#mos-target-notes).
LLVM-MOS defines generic, ca65 and xa65 parser variants; this proposal selects only
generic, not ca65, ACME or Kick Assembler compatibility. [Parser variants](https://github.com/llvm-mos/llvm-mos/blob/06bc967d2668c7c11c4d6eb43a6aed1f99ad258b/llvm/lib/Target/MOS/MOS.td).
The SDK has a dedicated C64 platform and both `.s` and `.S` inputs.
[Platform definition](https://github.com/llvm-mos/llvm-mos-sdk/blob/3f6968bbc156ff9a63102a8e158db868819bd61c/mos-platform/c64/CMakeLists.txt).

The key lexical differences must be modeled before operand interpretation:

- x86 GAS uses `#` for line comments and `;` to separate statements. Slash comment
  behavior depends on `--divide`; make that option explicit rather than guessing.
  [GNU special characters](https://sourceware.org/binutils/docs/as/i386_002dChars.html).
- LLVM-MOS sets `;` as its comment delimiter, newline as its statement separator,
  and enables Motorola-style integers. Its target parser handles MOS operands and
  directive aliases such as `.word` to `.2byte`. Preserve these spellings without
  calculating encodings or widths. [Assembler properties](https://github.com/llvm-mos/llvm-mos/blob/06bc967d2668c7c11c4d6eb43a6aed1f99ad258b/llvm/lib/Target/MOS/MCTargetDesc/MOSMCAsmInfo.cpp),
  [target parser](https://github.com/llvm-mos/llvm-mos/blob/06bc967d2668c7c11c4d6eb43a6aed1f99ad258b/llvm/lib/Target/MOS/AsmParser/MOSAsmParser.cpp).
- Numeric labels and `1f`/`1b` references are source records, not function names or
  resolved branch destinations. GNU permits repeated numeric labels.
  [GNU symbol names](https://sourceware.org/binutils/docs/as/Symbol-Names.html).

These public-source observations were checked on 2026-10-04; they do not identify
an installed toolchain version. Pin the implementation's fixture reference revision
and lexical contract independently of a user's actual assembler version.

## Input and selection contract

Start with standalone UTF-8 `.s` and `.S` only. Require explicit profile/file
selection and explicit preprocessing intent: `none` or `cpp-unexpanded`. The suffix
is recorded, not used to silently execute a preprocessor. Preserve raw `#include`,
`#define`, conditionals, continuations and line markers in the latter mode. Do not
load includes, expand macros, evaluate branches or remap physical source offsets
using line markers. Keep preprocessor recognition distinct from x86 comments and
MOS immediate `#` operands. Never infer a CPU from a filename, mnemonic or host.

Propose a version-3 config/schema boundary, freezing both current v1 and v2 schema
outputs and validation contracts. Draft configuration shape, not usable today:

```json
{
  "version": 3,
  "assembly_sources": [
    {"files": ["asm/x86/**/*.S"], "profile": "x86-gas-att32",
     "preprocessing": "cpp-unexpanded", "slash_mode": "gas-default"},
    {"files": ["asm/c64/**/*.s"], "profile": "mos-llvm-c64",
     "preprocessing": "none"}
  ]
}
```

This fragment omits ordinary include/rule configuration. Require global include
patterns covering selected sources and matching assembly-scoped rules. Proposed
selectors use `language: assembly`, an explicit `profile`, and `kind: file` or
`assembly_region`. No implicit cross-profile rules in the first release. Reject
missing/unknown profiles, invalid option combinations and overlapping selections
across profiles or Rust/C/Python adapters. Apply global filters before extension
validation on disk and in editor snapshots. Selected unsupported extensions fail.
No selected files, or active region rules without regions, fail operationally;
explicit rule filters/off overrides may skip. Preserve existing ordering and
canonicalization/containment checks. Cache keys must include directory, language,
profile, preprocessing and lexical options, not just directory or architecture.

Rust `asm!` and `global_asm!` extraction is a later, separately reviewed feature.
Ordinary Rust parsing does not supply expanded templates, operand substitutions,
conditional compilation, target options or correct assembly source mappings.
No new VS Code activation is part of this milestone.

## Parser choice and incomplete output

Recommend a small bounded source tokenizer, not an assembler emulator. Reuse the
existing span/target carriers and runner; add profile-specific lexical tables and
state transitions behind one dispatch boundary. Preserve every original byte;
records refer to source spans rather than reprinted or normalized instructions.

A concrete existing candidate, MIT-licensed `tree-sitter-asm`, is generic and was
last committed on 2025-11-08 at the inspected revision. Its grammar combines `#`
and `;` line comments and has limited quoted-string handling. It cannot be adopted
unchanged for these conflicting profiles. This is a source-level inspection,
not a tested performance or completeness comparison, nor a claim that no suitable
parser exists. [Inspected grammar](https://github.com/RubixDev/tree-sitter-asm/blob/839741fef4dab5128952334624905c82b40c7133/grammar.js).
Embedding LLVM's MC machinery would add a substantial native dependency and still
require a separate lossless source representation; no such dependency is selected.
Reconsider a maintained parser only if it passes the profile fixtures without
silently broadening the contract. No dependency installation is part of this plan.

Tokenize strings/escapes, comments, line continuations, statement boundaries,
labels, directive names and raw operand text. Do not classify arbitrary statement
heads as proven instructions: they may be macro invocations or unknown syntax.
Retain `.macro`/`.endm`, substitution text, repetitions and conditional records,
with macro/assembler state explicitly unknown. Capture known section and mode
directives as declarations, not verified effective state. Conflicting mode changes
such as `.code64` or `.intel_syntax` are unsupported for the x86 profile; MOS
variant/CPU switches likewise require a separate contract. Unknown directives
with safely delimited operands remain opaque records with span-bearing issues.

Every request includes profile and source-mode metadata, `completeness: incomplete`,
and structured unknown/unsupported reasons. Unresolved macros, symbols, relocations,
conditions and preprocessing remain unknown. Unsafe token boundaries (for example
unterminated quotes/comments), unbalanced recognized structural delimiters or
unsupported lexical mode changes fail operationally before any provider request.
Never silently discard input, downgrade a requested region to a file, or turn a
failure into an empty successful lint. A safely preserved opaque statement is
incomplete analysis, not proof that an assembler accepts it.

## Meaningful targets without invented functions

A file target spans `0..source.len()` including whitespace-only input. For the first
region target, use explicit paired comment annotations authored by the user:
`erislint-region-begin NAME` and `erislint-region-end NAME`, after the selected
profile's comment delimiter. The region covers bytes between those marker comments;
retain marker spans and the raw region source. Its name means a review region,
not a callable routine, basic block or control-flow boundary.

Require nonempty regions, unique names, matching pairs and no nesting. Recognize
markers only as standalone comment lines outside strings and captured macro bodies.
Keep preprocessor/assembler conditional ancestry in context without deciding branch
activity. Fixtures must settle exact marker/newline ownership before implementation.
Labels, repeated numeric labels, `.type`/`.size`, section records and macro headers
are metadata inside regions; none automatically creates a function target. Later
explicit region schemes can be added after review, without changing this meaning.

No claims about instruction legality, CPU correctness, undocumented opcodes, cycle
timing, stack effects, ABI, branch displacement, macro expansion, object layout,
linker placement or C64 memory-mapped behavior are supported. Any future claim
needs an explicit model and independently tested evidence. Source-level model
judgments remain review leads with an uncertainty answer available.

## Implementation stages after design approval

1. Freeze Rust/C/Python requests and v1/v2 schemas; add the v3/profile selection
   contract and tests. Separate any common-carrier refactor from feature commits.
2. Implement lossless file records and bounded tokenization for x86 GAS/AT&T32,
   including explicit slash mode, raw preprocessing and uncertainty outputs.
3. Add LLVM-MOS generic/C64 lexical tables and operands as raw records. Gate on
   cross-profile punctuation tests and the pinned public references.
4. Add explicit regions, structural error handling, mixed-language dispatch/cache
   tests, disk/editor snapshots, diagnostics and complete request fixtures.
5. Publish examples and limitations; run full regression, formatting, strict Clippy,
   protocol checks, offline config validation and whole-tree dry-run. Compare Rust
   requests with pristine upstream. Independent review precedes broader syntax.

Use focused, reviewable commits with actual test evidence. The fixture matrix in
[ASSEMBLY-FIXTURES.md](ASSEMBLY-FIXTURES.md) is the acceptance gate. No assembler,
linker, preprocessor, analyzed program or provider is run by this design task.
Future live self-review requires a separate bounded reservation within the existing
project allowance; offline checks are not live semantic lint. Stop here for review
before parser implementation.
