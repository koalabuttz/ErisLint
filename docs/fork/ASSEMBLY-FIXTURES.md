<!-- Fork-specific addition. SPDX-License-Identifier: AGPL-3.0-only -->

# Proposed assembly fixture matrix

Status: planned tests, not executed fixtures or support claims. Author tiny original
samples from the public dialect references in [the design](ASSEMBLY-DESIGN.md).
Do not import private sources, generated assembly, symbols, addresses or build
artifacts. Pin reference revisions and record expected tokens, spans, raw source,
profile/preprocessing metadata, unknown reasons, targets and operational failures.

| Area | x86 GAS/AT&T32 cases | LLVM-MOS generic/C64 cases | Required assertion |
| --- | --- | --- | --- |
| Comment and separator conflict | `nop; nop # note`; explicit slash modes | `lda #$01 ; note` | Two x86 statements; MOS immediate retained and suffix comment separated; no common comment regex |
| Strings and escapes | `.ascii "#;\\\""`; comment-shaped text | `.ascii ";#\\\""`; punctuation inside literals | Preserve bytes; delimiters inside strings never split statements; unsupported escapes are explicit |
| Block comments/continuations | Original multiline/comment-boundary samples | Verify against pinned LLVM generic lexer before accepting | Exact spans across lines; unsupported forms do not disappear |
| Registers and raw operands | `%eax`, `$1`, `4(%eax,%ecx,4)`, indirect operand spelling | `#$01`, `$d000`, `($20),y`, symbol expressions and modifiers | Lossless operand records, no inferred width, address or validity |
| Labels | Named and `.L` labels, aliases, label plus statement | Named/local spelling and label plus instruction-like text | Labels retained as labels; never function targets |
| Numeric local references | Repeated `1:`, `1f`, `1b` | Confirm generic LLVM-MOS forms against pinned parser fixtures | Preserve occurrence spans/reference spelling; no guessed destination or displacement |
| Directives and sections | `.code32`, `.section`, `.type`, `.size`, `.byte` | `.section`, `.byte`, `.word`, target-specific modifiers | Raw declaration metadata; no object layout or relocation evaluation |
| Macros/repetition | `.macro`/`.endm`, substitutions, `.rept`/`.endr` | LLVM generic equivalents only after reference confirmation | Opaque bodies and invocation uncertainty; no expansion; marker-looking macro text not extracted |
| Preprocessing | `.S` with defines/includes/conditional branches/line markers | `.S` with preprocessor `#` and MOS immediate `#` | Explicit `cpp-unexpanded`; source retained, no include reads or conditional evaluation |
| Mode/profile mismatch | `.code16`, `.code64`, `.intel_syntax`; ambiguous slash option | ca65/xa65 syntax switches, other 65xx profile requests | Unsupported contract reported; no silent mode switch |
| Opaque input | Unknown directives, macro-like statement heads | Unknown mnemonics, undocumented-opcode spellings | Preserved source plus incomplete/unsupported reason, never an acceptance/correctness claim |
| Lexical failure | Unterminated strings/comments, malformed recognized structure | Same, using MOS delimiters | Operational error before provider calls; no recovery that drops bytes |
| File targets | Empty, whitespace-only, Unicode comments, CRLF, no final newline | Same | Complete original file range; byte and Unicode line/column spans agree |
| Region targets | Paired standalone `#` annotation comments | Paired standalone `;` annotation comments | Exact marker/body ranges; no label-derived functions |
| Invalid regions | Empty, duplicate, nested, crossed, unmatched markers | Same; markers hidden in strings/macros | Deterministic errors; no synthetic regions or silent empty success |
| Context | Region crossing raw conditional branches | Region referencing missing symbol/import-like directive | Explicit unknown macro/conditional/assembler state; no branch activity inference |
| Selection/cache | x86 and MOS in same directory, reversed explicit input order | Same alongside Rust/C/Python files | Stable ordering; distinct profile/options cache keys; no Cargo lookup for assembly |
| Filtering/versioning | Broad globs, excluded unsupported extensions, overlapping profile globs | Same in disk and unsaved editor input | Global scope first; ambiguity rejected; v1/v2 cannot inherit v3 assembly enablement |
| Compatibility | Existing Rust v1 and C/Python v2 fixtures | Both profiles disabled | Existing schema/request bytes and error contracts unchanged |
| Embedded Rust assembly | Rust files containing `asm!` and `global_asm!` | Rust-mos-style macro invocation text | Existing Rust processing only; no claimed embedded assembly extraction |

Before implementing a profile, write exact expected records for the delimiter,
string and region cases and settle unsupported boundaries. Conditional/macro state
must be visible even when the tokenizer can recover statement boundaries. Full
requests should snapshot at least one complete file and one region per profile.
Use synthetic addresses strictly as unvalidated tokens, not as private memory maps.

Optional future toolchain comparisons may assemble only these original samples
under explicitly pinned tools, after approval for installing/running those tools.
Their results would be syntax/encoding evidence for that toolchain, not proof of
runtime behavior, and must remain separate from normal ErisLint source review.
No assembly tools or samples were executed for this planning milestone.
