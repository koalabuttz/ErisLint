<!-- Fork-specific addition. SPDX-License-Identifier: AGPL-3.0-only -->

# Bounded LLVM-MOS/C64 implementation checkpoint

This milestone starts from approved x86 checkpoint
`dd56a2cb34927d6e9391f221e02b98daa5ee9a28`. It preserves Eriskii's branding,
copyright, license and upstream history. Upstream/main remain at
`f04d016461b66b38d46647fb20762fb67bda350a`.

## Implemented scope and reference verification

`mos-llvm-c64` enables standalone UTF-8 `.s`/`.S` source review using a bounded
LLVM-MOS generic subset. Preprocessing intent is required; GAS slash options are
rejected. Runtime options keep profile/preprocessing separation in cache keys.
Existing v3 schema bytes already describe this profile and remain unchanged.

Primary sources were re-read at LLVM-MOS revision
`06bc967d2668c7c11c4d6eb43a6aed1f99ad258b`; no assembler/toolchain was installed
or executed. These observations inform the bounded contract, not a claim of full
parser conformance:

- MOS sets semicolon comments, newline separation and Motorola-style integers.
  Its modifiers are preserved without interpretation.
  [MOSMCAsmInfo.cpp](https://github.com/llvm-mos/llvm-mos/blob/06bc967d2668c7c11c4d6eb43a6aed1f99ad258b/llvm/lib/Target/MOS/MCTargetDesc/MOSMCAsmInfo.cpp)
- The generic lexer supports inherited `//` and block comments, statement-start
  hash comments and paired character literals. Block comments consume the
  statement-start state, even across internal newlines; ordinary spaces retain
  it. Our scanner follows those boundaries while deliberately narrowing literals.
  [AsmLexer.cpp](https://github.com/llvm-mos/llvm-mos/blob/06bc967d2668c7c11c4d6eb43a6aed1f99ad258b/llvm/lib/MC/MCParser/AsmLexer.cpp),
  [MCAsmInfo defaults](https://github.com/llvm-mos/llvm-mos/blob/06bc967d2668c7c11c4d6eb43a6aed1f99ad258b/llvm/include/llvm/MC/MCAsmInfo.h)
- The generic parser contains directional local-symbol handling and the macro,
  repetition and conditional directives used by our structural fixtures. We
  preserve their records without applying the parser's expansion or resolution.
  [AsmParser.cpp](https://github.com/llvm-mos/llvm-mos/blob/06bc967d2668c7c11c4d6eb43a6aed1f99ad258b/llvm/lib/MC/MCParser/AsmParser.cpp)
- The target defines distinct generic, ca65 and xa65 variants; only generic is
  selected. Its `.word` alias does not establish layout in ErisLint.
  [MOS.td](https://github.com/llvm-mos/llvm-mos/blob/06bc967d2668c7c11c4d6eb43a6aed1f99ad258b/llvm/lib/Target/MOS/MOS.td),
  [MOSAsmParser.cpp](https://github.com/llvm-mos/llvm-mos/blob/06bc967d2668c7c11c4d6eb43a6aed1f99ad258b/llvm/lib/Target/MOS/AsmParser/MOSAsmParser.cpp)
- The device definitions distinguish `mos6502` and `mos6502x`. C64/6510 remains
  user intent, not an inferred LLVM CPU option or validated feature set.
  [MOSDevices.td](https://github.com/llvm-mos/llvm-mos/blob/06bc967d2668c7c11c4d6eb43a6aed1f99ad258b/llvm/lib/Target/MOS/MOSDevices.td)

## Shared behavior and limits

The lossless record layer, raw structural dependencies, CPP precedence, explicit
region grammar and fixed uncertainty handling are shared with x86. MOS marks
regions only with canonical standalone semicolon comments. Essential dependencies
remain in base analysis for all context modes. A file target includes all bytes;
region spans distinguish the begin name, body, marker lines and comments.

MOS requests identify bounded generic dialect, reference revision and C64/6510
intent, with an explicitly empty `validated_cpu_features` list. Instruction-like
heads, operands, modifiers and unknown directives stay opaque. This does not
validate instructions, 6510/undocumented opcodes, addressing modes, branch ranges,
object layout, memory-map behavior, stack effects, timing, ABI, linking or execution.
Labels do not create function targets. Includes and analyzed source never execute.

Deliberate rejections include ca65/xa65 profile requests, CPU/dialect switches
(`.cpu`, `.setcpu`, `.arch`, `.machine`, `.syntax`), x86 mode directives under MOS,
alternate macro syntax, unsupported conditional forms, unbalanced structures,
unclosed tokens, bare CR, unsupported escapes, and raw newlines or non-ASCII/multiple
characters in character literals. Recognized structural delimiters are checked;
arbitrary directive arguments and operands are not fully syntax-validated.
Unknown CPU semantics are never converted into a correctness claim.

The exact [reviewed contract](ASSEMBLY-CONTRACT-DETAILS.md) remains authoritative.
The [example](../../examples/assembly-mos/README.md) documents configuration and
offline commands. Embedded Rust `asm!`/`global_asm!` stays in ordinary Rust parsing;
there is no embedded assembly extraction or additional VS Code activation.

## Verification actually performed

All **122 Rust tests** and **six extension protocol tests** pass, as do
`cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings` and
`git diff --check`. No dependencies were added. Fourteen MOS integration tests
cover the reviewed fixture gates with synthetic public source:

| Gate | Concrete coverage |
| --- | --- |
| Lexical conflicts | Exact records/spans for semicolon, slash, immediate and statement-start hash; block-comment adjacency and spanning-line state |
| Literals and malformed input | Paired punctuation/escaped characters, protected UTF-8 strings, bounded escapes, unterminated/non-nesting comments, bare CR and CRLF boundaries |
| Source metadata | Named/repeated numeric/local labels, `1f`/`1b`, raw numeric/address/modifier spellings, directives and opaque statements; no feature validation |
| CPP and structure | LF/CRLF opaque continued definitions, inert replacement text, include retention, raw conditionals, nested repetitions, unsupported state inside macros/branches, scrub controls |
| Regions/files | Exact 69-byte Unicode/CRLF case and columns, EOF endings, preamble/trailer, empty/whitespace files, valid/invalid names and pairs, inert markers |
| Request contexts | Six complete MOS JSON requests plus six unchanged x86 requests; essential macro/alias/conditional/directive state retained across all modes |
| Dispatch | Both profiles alongside Rust/C/Python, differing options, nested directories, reversed/duplicate paths, global filters, ambiguity, invalid UTF-8 and missing/outside-root paths |
| Configuration | Explicit options, invalid profile/mode combinations, inherited/external rule provenance, old-version rejection and final source/rule pairing |
| Offline editor | Unsaved CLI region source, unchanged disk content, missing-region failure, explicit off without credentials |
| Uncertainty | Mock answers retain model/probabilities and fixed warnings; invalid policies rejected; both severity overrides tested |

A private CLI helper now holds the unchanged exit/filter ordering so parsed
`--deny-warnings`/`--errors-only` flags can be tested with mock answers for both
profiles. This is an offline unit test of the actual status/filter path, not a
mock network or live CLI provider run. No endpoint override was introduced.

Frozen v1/v2/v3 schema bytes, representative legacy errors, Rust/C/Python requests
and x86 snapshots all pass unchanged. Original self-lint configuration and rule
hashes are unchanged. Whole-project offline validation and dry-run produce
**277 questions across 28 Rust files**, byte-identical to pristine upstream on
the same sources. This is request construction, not live semantic lint.

No live Jev calls or additional spending reservations were made. No hosted CI is
configured; extension-host testing remains unavailable without `code`. Stop here
for independent MOS review before expanding syntax or making semantic claims.
