<!-- Fork-specific addition. SPDX-License-Identifier: AGPL-3.0-only -->

# Proposed assembly fixture matrix

Execution status: the historical gates below are mapped to implemented tests in
the [x86 checkpoint](ASSEMBLY-X86-CHECKPOINT.md) and
[MOS checkpoint](ASSEMBLY-MOS-CHECKPOINT.md). The latter covers both-profile
request snapshots and offline mock CLI warning/display checks.

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

## Exact acceptance cases from contract review

The [contract details](ASSEMBLY-CONTRACT-DETAILS.md) supersede the matrix's former
open questions. All cases here are planned, not reported test results. `P` means
preserved/incomplete source records, not assembler or CPU acceptance; `E` means
operational rejection before any provider call. In code-fenced examples, physical
newlines are literal. Where bytes are given as JSON strings, decode JSON escapes
once to obtain the exact input; do not execute the assembly.

### Literal and comment cases

| Profile / case | Input or exact construction | Expected |
| --- | --- | --- |
| GAS character | `.byte 'A` followed by LF | P: single prefix-character token, no closing quote expected |
| GAS escaped newline | JSON `".byte '\\n\n"` | P: escape spelling stays raw, final LF is the statement end |
| GAS literal LF | JSON `".byte '\n"` | E: unsupported literal-newline character, not a statement followed by ordinary code |
| GAS literal CRLF | JSON `".byte '\r\n"` | E, preserving error span over quote/newline boundary |
| GAS rejected literal | `.byte 'A'`, `.byte 'é`, bare quote at EOF, `.byte '\q` | E for paired, non-ASCII, missing and unsupported escaped characters respectively |
| MOS character | `.byte 'A'`, `.byte '\n'`, `.byte '\''` | P: one paired character token per operand, no decoding |
| MOS rejected literal | `.byte 'A`, `.byte ''`, `.byte 'AB'`, `.byte 'é'`, `.byte '\q'` | E, never reclassified as opaque instruction text |
| MOS literal LF | JSON `".byte '\n'\n"` | E: actual LF inside character literal |
| MOS literal CRLF | JSON `".byte '\r\n'\r\n"` | E: actual CRLF inside character literal |
| Both string protection | `.ascii "#;/* erislint-region-begin fake */"` | P: one string record; zero markers/directives inside it |
| Both escaped quote | JSON `".ascii \"a\\\"b\"\n"` | P: escaped quote does not end string early |
| Both unsupported strings | raw LF/CRLF, backslash-newline, `\q`, EOF before closing quote | E with byte span; no recovery as ordinary code |
| MOS adjacent comments | `lda #1;note`, `lda #1//note`, `lda/*note*/#1` | P: immediate/operand hash retained; comment spans exact |
| MOS hash state | `  # note` versus `label: #1` versus `/*x*/#1` | First is a hash comment; latter hashes are raw operand punctuation, no opcode-validity claim |
| MOS block spanning LF | JSON `"/*x\ny*/#1\n"` | Hash is NOT statement-start comment; spanning block comment does not restore that state |
| MOS fake markers | `// ; erislint-region-begin fake` and `# ; erislint-region-begin fake` | P: zero markers; do not recursively scan line-comment bodies |
| Both block fake markers | block comment containing a complete canonical marker line | P: zero markers, including after internal newline |
| GAS slash selection | `nop / note` under gas-default versus divide | First suffix is comment; second slash stays punctuation |
| GAS divide ambiguity | `nop // note` with divide | E: explicit unsupported comment-like form |
| Both token adjacency | `name/*x*/suffix`, escaped literal containing delimiter, comment ending next to a quote | Separate source records; no token concatenation or dropped bytes |

Verify printable punctuation characters too: GAS `'#` and `';`, MOS `'#'` and
`';'`, followed by a real comment. Character scanning must precede comment scanning.
Block comments are non-nesting: first `*/` ends the comment; an unmatched terminator
outside a comment is an explicit unsupported lexical form. Test both profiles.

### CPP and structural precedence cases

In `cpp-unexpanded`, this is exactly ONE opaque definition record, zero assembler
mode controls, zero macro definitions and zero region markers:

```text
#define BODY \
 .macro hidden \
 .intel_syntax noprefix \
 #APP \
 ; erislint-region-begin fake
```

Repeat with `#` canonical x86 marker text on the final continued line, and with
CRLF throughout. A quote-looking replacement token does not open an assembly
string. EOF immediately after a continuation backslash is E. A following,
noncontinued `.intel_syntax` line is E. The same definition in `none` mode is E
(`preprocessing_mode_required`). `#define` inside an already-open block comment
is only comment text. An actual source `#include "missing.inc"` never reads a file.

For each profile, exercise these structural pairs and their failure counterparts:

- `.macro m` / `.endm`; `.rept 2`, `.irp p,a,b`, `.irpc p,ab` / `.endr`;
  nested repetitions are P with unknown execution/count/substitution. Nested
  macro definitions, crossed closers and missing closers are E.
- `.if 0` / `.else` / `.endif` and CPP `#if 0` / `#else` / `#endif` are P with
  unknown activity, not evaluated false. Unmatched branches and `.ifc` are E.
- `.intel_syntax`, `.code64`, `.altmacro`, `.noaltmacro` inside an unexpanded
  `.macro` body or either unknown conditional branch remain E. Their spellings
  inside strings/comments or a continued CPP record are inert and P.
- Exact standalone `#APP` and `#NO_APP`, including indented versions and CRLF,
  are E in both profiles and preprocessing modes. `#APP extra` stays a comment.
  Those exact bytes in strings, ordinary comment bodies or CPP replacement text
  are inert. An ordinary comment containing `#NO_APP` must not end an opaque record.
- Marker-looking canonical comment lines inside macro/repetition bodies produce
  no regions. Macro substitutions are recorded, not evaluated as marker text.

### Region grammar and exact ranges

For both profiles and all context modes, test the 69-byte CRLF/Unicode example in
[the contract](ASSEMBLY-CONTRACT-DETAILS.md), including these exact values:
body `[29,42)`, begin-name `[26,27)`, `é` `[37,39)`, file `[0,69)`. Assert original
byte slices and Unicode columns independently. Repeat with `;` instead of `#`,
LF endings, no final newline, tabs, Unicode comments and preamble/trailer text.

Positive names: `r`, `_start`, `loop.1-a`, exactly 64 ASCII characters. Negative
names: empty, digit-first, 65 characters, Unicode, colon, slash or spaces. Eligible
markers with trailing prose or a second name are E, not silently ignored.
Trailing spaces/tabs and indentation are accepted and retained. No space between
canonical delimiter and the reserved keyword is an ordinary comment, not a marker.
`nop # erislint-region-begin r` is inline comment text, not a GAS marker.

Both-profile negatives: immediate end, whitespace-only body (spaces/tabs/LF/CRLF,
and separately Unicode whitespace), duplicate names, nested/crossed/unmatched
markers, missing begin terminator. Comment-only nonblank bodies are P. A closing
marker at EOF is accepted. Markers inside block comments, strings, alternate line
comments, CPP definitions and macro/repetition bodies are inert, never regions.

Create twelve full JSON request snapshots: file and region targets times target,
enclosing and file context modes times two profiles. Include conditional headers,
macro/alias records and declared-state directives outside the region. Assert that
base `analysis` retains their raw bytes/spans and all essential unknown reasons
in every snapshot; only optional surroundings/full-file context may differ.

### Uncertainty and version compatibility gates

Use mock provider answers only for these tests:

- Reject missing/renamed `insufficient_context`, wrong description, only that
  option, catch-all diagnostic policies, uncertainty top-level policies and nested
  choice/probability references. Accept the exact required option plus substantive
  choices and explicit substantive-choice policies.
- An uncertain assembly answer creates exactly one fixed inconclusive warning
  regardless of confidence or severity override; it does not trigger any user
  defect policy. Default exit 0, `--deny-warnings` exit 1. Rule `off` makes no
  request. `--errors-only` follows existing filtering. Substantive answers retain
  normal policy order. Repeat v1/v2 and v3 nonassembly rules to prove their original
  uncertainty/policy behavior is unchanged.
- Mixed v3 Rust/C/Python/x86/MOS files in one directory, multiple directories,
  reversed/duplicate explicit paths, options differing only in preprocessing or
  slash mode, and disk/editor paths: preserve order, spans and cache separation.
- v3 extends v1, v2 and v3: validate base document and its external rules under
  their own version before merge. v1 or v2 extends v3 (including Rust-only v3): E.
  Freeze current v1-to-v2 and v2-to-v1 success/error cases; do not retroactively
  replace their rules with the new v3-to-older prohibition.
- v3 string external rule path means v3. Explicit `{path, version: 2}` accepts
  existing v2 C/Python/Rust rule files; `{path, version: 1}` uses frozen v1 rules.
  Reject assembly fields in older imports even when null, unsupported declared
  versions, and attempts to use `$schema` to override the declared version.
  A v2 base's external paths resolve relative to that base and stay v2 on import.
- v3 override of an inherited rule revalidates the entire replacement, including
  profile/uncertainty constraints if changed to assembly. Inherited source filters
  cannot bypass final profile/rule pairing or global-scope extension validation.
- Freeze BOTH legacy schema families, full Rust/C/Python requests and representative
  CLI errors before modifying exact `version == 2` guards. Cover null fields,
  missing profiles, wrong target kinds, no targets, ambiguous selections, outside-
  root/missing editor paths and unselected files. New v3 errors are separate.

The documentation-only review revision checks local Markdown links, byte arithmetic,
privacy/scope and unchanged offline self-lint. It does not claim any of these future
assembly fixtures have run, and adds no dependencies or executable source.
