<!-- Fork-specific addition. SPDX-License-Identifier: AGPL-3.0-only -->

# Assembly contract decisions after review

Implementation status: the contract below now governs both bounded profiles.
See the [x86 checkpoint](ASSEMBLY-X86-CHECKPOINT.md) and
[MOS checkpoint](ASSEMBLY-MOS-CHECKPOINT.md) for executed verification; historical
proposal wording below is retained as the reviewed decision record.

Normative proposal, not implemented. This refines [the design](ASSEMBLY-DESIGN.md)
and takes precedence over its earlier open questions. No parser, assembler or
provider was run. All fixtures below are planned acceptance tests.

## 1. Lexical boundaries and supported literals

The pinned LLVM generic lexer handles `//`, `/*...*/`, and statement-start `#`
comments in addition to target comments. Whitespace preserves statement-start
state; a block comment consumes that state. Operand `#` is a token, not a line
comment. `LexSingleQuote` requires a closing quote after one byte or escaped byte.
`MCAsmInfo` defaults allow additional comments; its character-literal printing
property is not a replacement for lexer behavior.
[Lexer](https://github.com/llvm-mos/llvm-mos/blob/06bc967d2668c7c11c4d6eb43a6aed1f99ad258b/llvm/lib/MC/MCParser/AsmLexer.cpp),
[defaults](https://github.com/llvm-mos/llvm-mos/blob/06bc967d2668c7c11c4d6eb43a6aed1f99ad258b/llvm/include/llvm/MC/MCAsmInfo.h).
GAS instead documents quote-prefix character constants, including a literal newline
following the quote. [GNU characters](https://sourceware.org/binutils/docs/as/Chars.html).

Initial supported subset, intentionally narrower than either assembler:

- Double-quoted strings: preserve UTF-8 content; recognize escaped quote/backslash
  and the escapes `\b`, `\f`, `\n`, `\r`, `\t`. Reject other escape spellings,
  raw CR/LF inside strings, backslash-newline within strings and unterminated
  strings. Do not decode values. A rejected literal is an operational error,
  never ordinary instruction text. Strings remain opaque to comment/marker scans.
- GAS characters: quote followed by exactly one printable ASCII byte, or one of
  the listed escapes plus `\'`. There is no closing quote. Require EOF, horizontal
  whitespace, newline, comma, operator, parenthesis or a profile comment/separator
  boundary after the character token. Reject paired `'A'`, multi-byte characters,
  unknown escapes, a bare quote at EOF, and quote followed by literal LF or CRLF.
  The newline rejection is deliberate even though GAS supports that special case.
- LLVM-MOS characters: paired `'A'` or one supported escaped byte, including `\'`.
  Reject unclosed `'A`, empty `''`, multi-byte/multi-character contents, unknown
  escapes and actual CR/LF between quotes. Escaped `\n` remains accepted as raw
  spelling. This subset does not claim all LLVM character syntax.
- MOS comments: `;` and `//` end at the next physical newline; `/*...*/` is
  non-nesting and may span lines. Recognize adjacent forms such as `lda #1;note`,
  `lda #1//note` and `lda/*note*/#1`. A slash not starting a comment is raw operand
  punctuation. At statement start, optional spaces/tabs followed by `#` form a
  comment unless the CPP precedence rule below applies. After any token, including
  a same-line label or block comment, `#` is operand punctuation. Thus `/*x*/#1`
  must not become a line comment. Newlines outside comments/literals restore
  statement-start state; a spanning block comment does not itself reset it.
- x86 comments: recognize `/*...*/` before slash-mode handling, `#` line comments
  and `;` statement separators. `gas-default` treats remaining `/` as a line
  comment; `divide` retains remaining slash punctuation. In `divide`, `//` is
  explicitly unsupported in the first subset, rather than guessed as a comment.
  A block comment must never merge neighboring source tokens or alter offsets.

Line comments own text up to but excluding LF/CRLF; line-ending records retain
those bytes. Outside opaque CPP records, reject bare CR line endings initially.
Escaped line continuations outside strings are recorded with physical spans;
comments end at physical newlines unless already inside a CPP record. Preserve
literal punctuation such as `'#'` in MOS and `'#` in GAS before comment detection.
Neither comment bodies nor strings may recursively create markers or directives.
An unmatched block-comment terminator outside a comment is operationally unsupported.

## 2. Precedence and structural state

Scan left to right using this precedence, with no preprocessing or macro execution:

1. While inside an already-open string, character or comment, finish that token
   under the selected subset. A `#` inside it cannot begin CPP or assembler state.
2. At a physical line's first non-space/tab byte, outside those tokens, recognize
   a CPP directive only in `cpp-unexpanded`: `define`, `undef`, `include`, `if`,
   `ifdef`, `ifndef`, `elif`, `else`, `endif`, `line`, `error`, `warning`, `pragma`,
   or a numeric line marker. Retain the entire directive and every backslash-LF
   or backslash-CRLF continued physical line as ONE opaque record. Do not rescan
   replacement text for quotes, `.macro`, `.intel_syntax`, `#APP` or markers.
   A final backslash with no continued line is an operational error. Apply only
   the directive header to the raw conditional stack, not its replacement text.
3. Otherwise tokenize profile literals/comments/statements. Recognize structural
   directives and annotation comments only at their defined source boundaries.
   In `none` mode, recognized CPP directive heads/line markers outside opaque
   tokens fail with `preprocessing_mode_required`; do not silently analyze a
   preprocessor-dependent source as assembler-only comments. Other statement-start
   hash comments remain comments in MOS. Ordinary x86 hash comments remain comments.

`#APP` and `#NO_APP` concern assembler scrubbing, distinct from CPP.
[GNU preprocessing](https://sourceware.org/binutils/docs/as/Preprocessing.html).
For both profiles, exact standalone hash records with either name (optional
indentation/trailing spaces, no other text) are unsupported operational errors.
No simulated scrub-state transitions. Inside an opaque continued CPP record,
string or ordinary comment body they are inert bytes. `#APP extra` is an ordinary
hash comment, not a supported state control. Inputs relying on assembler `-f`
are outside this initial source contract.

Recognize `.macro NAME ...` through `.endm` and `.rept`, `.irp`, `.irpc` through
`.endr` as raw structural records. Nested repetitions are balanced; a nested
`.macro` definition is unsupported. Missing/crossed terminators fail. Macro and
repetition bodies are opaque for target discovery, expansion and evaluation, but
still scanned for literal boundaries, structural delimiters and forbidden state
controls. Ordinary backslash substitutions stay raw. Recognize `.if`, `.ifdef`,
`.ifndef`, `.elseif`, `.else`, `.endif` as unevaluated assembler conditional records;
reject other `.if*` forms initially with `unsupported_conditional_form`.

Reject `.altmacro` and `.noaltmacro` wherever the structural scan sees them;
alternate macro quoting/concatenation is not implemented. Also reject `.code16`,
`.code64`, `.intel_syntax`, and unsupported MOS CPU/variant state switches (including `.cpu`, `.setcpu`, `.arch`,
`.machine` and `.syntax`) even
inside unexpanded macro/repetition bodies or unknown conditional branches. No
claim that a macro is invoked or a branch active is needed for this conservative
source restriction. Supported `.code32`/`.att_syntax` are declared-state records,
not proof of effective mode. A continued CPP definition containing any of these
spellings stays opaque and does NOT trigger rejection. Fixtures enforce this
precedence so inert replacement text is never mistaken for executed structure.

## 3. Region grammar, spans and essential dependencies

Only canonical profile line comments can mark regions: `#` for x86 and `;` for
MOS. `//`, `/*...*/`, and MOS hash comments cannot mark regions. At an eligible
physical line, exact grammar is:

```text
[space-or-tab]* DELIMITER [space-or-tab]+ erislint-region-begin [space-or-tab]+ NAME [space-or-tab]* EOL
[space-or-tab]* DELIMITER [space-or-tab]+ erislint-region-end   [space-or-tab]+ NAME [space-or-tab]* EOL-or-EOF
NAME = [A-Za-z_][A-Za-z0-9_.-]{0,63}
```

Names are case-sensitive and unique per file. A begin requires LF/CRLF; an end
may terminate at EOF. Trailing text beyond horizontal whitespace is an error for
an otherwise eligible comment starting with either reserved marker keyword.
Likewise invalid/missing names fail. Unrelated comments are ordinary records.
Markers in strings, block/other line-comment bodies, continued CPP definitions,
macro or repetition bodies are inert, not malformed markers. Inline comments
following a statement are ineligible. Indentation is spaces/tabs only.

Require matching names, no nesting, no crossed pairs and no unmatched markers.
The body starts AFTER the begin line terminator and ends BEFORE the indentation
of the end-marker line. It includes its own final LF/CRLF, if present. Reject
zero-byte bodies and bodies consisting entirely of Unicode whitespace. A comment-
only body is nonempty reviewable source, not an implied instruction block.
`range` is the exact body range; target `span` is the name on the begin line.
Keep separate full marker-line spans (indentation and terminators included) and
comment spans (delimiter through trailing horizontal whitespace, excluding EOL).
Columns count Unicode scalar values from one, tabs count as one; offsets are
zero-based bytes, end-exclusive. Do not normalize line endings.

Exact example, shown with escaped physical line endings:

```text
"  # erislint-region-begin r\r\n\t.byte \"é\"\r\n  # erislint-region-end r\r\n"
```

UTF-8 length 69 bytes. Begin line `[0,29)`, body `[29,42)`, end line `[42,69)`.
Begin name `[26,27)` is line 1 columns 27..28. Body range is line 2 column 1 to
line 3 column 1; `é` occupies bytes `[37,39)` and line 2 columns 9..10. Comment
spans are `[2,27)` and `[44,67)`. Replacing both `#` delimiters with `;` gives
identical MOS offsets. An end at EOF without CRLF changes total length to 67 and
end-line span to `[42,67)`, leaving the body unchanged.

Essential dependencies live in the target's base `analysis` state, NOT only in
optional `context.enclosing`: profile/options, raw preprocessing status, unknown
conditional ancestry (including branches that cross region bounds), macro/alias
and assembler-state dependencies with source spans, and unresolved-state reasons.
Keep relevant raw directive records there, including out-of-region definitions;
if relevance cannot be determined, retain conservative file-wide directive
records and label their scope unresolved. `context: target` must retain these
facts. `enclosing` may add surroundings; `file` additionally adds complete source.
Neither changes essential unknowns or claims state resolution. Snapshot full
file and region requests in all three modes for BOTH profiles (12 snapshots).

## 4. Required assembly uncertainty answer and compatibility

Only assembly-scoped v3 rules require a choice named `insufficient_context` with
this exact description: `Required context is missing, unknown, or unsupported; no substantive judgment can be made.`
Require at least one other substantive choice within the existing 2..255 bound.
Append fixed assembly question guidance to select that option when missing state
prevents judgment, without replacing user-authored substantive criteria. Reject
missing/renamed uncertainty options or conflicting descriptions during config
validation, including inherited and external assembly rules.

For an assembly answer selecting `insufficient_context`, skip user diagnostic
policies and emit one fixed warning: `Assembly review inconclusive: insufficient context.`
Retain the answer/model probabilities in the ordinary unfiltered report. This is
an inconclusive review warning, never a defect or correctness verdict. Severity
overrides cannot promote or suppress it; rule `off` prevents evaluation entirely.
Normal warning exit behavior applies (0 unless `--deny-warnings`, then 1).
`--errors-only` retains its existing explicit display filtering behavior.

Assembly user diagnostic policies must each have a top-level `when.choice` naming
a substantive option; nested conditions may not reference `insufficient_context`
as choice or probability. Reject catch-all policies and such references rather
than allowing uncertainty to fall through into a defect policy. On substantive
answers, existing policy order/threshold logic applies. The uncertainty mechanism
is assembly-only; do not change question validation, thresholds, reports or
uncertainty behavior for v1/v2 or Rust/C/Python rules under v3.

Version decisions:

- v3 may extend v1/v2/v3 configs. Validate each document using its OWN frozen
  version contract before merging; retain rule provenance. v1/v2 may not extend
  any v3 document, even a Rust-only one. Existing v1-v2 inheritance behavior and
  its current rejection cases remain unchanged.
- Existing string `rule_files` paths use the declaring config's version, as today.
  v3 additionally permits `{ "path": "rules.json", "version": 2 }` (versions
  1, 2 or 3) to explicitly import older/newer rule-file shapes under v3. A v2
  import rejects assembly fields even when null. `$schema` is informational,
  not permission to reinterpret a rule file. Inline v3 Rust/C/Python rules retain
  their established language behavior; omitted language still means Rust.
- A v3 child can override a legacy rule with a validated v3 definition; an
  assembly replacement must satisfy all profile and uncertainty requirements.
  Inherited assembly enablement cannot leak into older consumers. Validate the
  final profile/rule pairing after merge, in addition to document validation.
- Audit every exact `version == 2` check in config loading/merge and C/Python
  enablement. Use explicit accepted-version capabilities for v3 rather than a
  blanket numeric comparison. Freeze legacy DTO/schema bytes and error snapshots
  first; v2 external rule files retain their current acceptance/rejection rules.

Acceptance requires mixed v3 Rust/C/Python/both-assembly-profile configurations,
all inheritance directions, explicit versioned external rules, override provenance,
unknown-version/null-field errors, and frozen legacy schemas/requests/errors.
Mock responses test uncertainty warnings, overrides, policy rejection and CLI flags;
no provider access is required for those tests. Stop for review before code.
