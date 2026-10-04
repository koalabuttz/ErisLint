# Fork-specific LLVM-MOS source review

This synthetic example selects `mos-llvm-c64`: a bounded subset of LLVM-MOS
generic assembly syntax with C64/6510 **intent**, not validated CPU features.
It preserves Eriskii's upstream branding and AGPL-3.0-only license. The example
is source-review material, not an executable program or assembler acceptance test.

Run offline from the repository root:

```sh
cargo run --locked -- --config examples/assembly-mos/erislint.json --check-config
cargo run --locked -- --config examples/assembly-mos/erislint.json --dry-run
```

The example constructs six requests: file and marked region, each in target,
enclosing and file context. No credentials, provider calls, include reads,
preprocessing, assembly or analyzed-code execution are involved. Offline request
construction is not semantic lint. Live review needs separate authorization.

Use version 3, matching global `include` patterns, and `assembly_sources` with
explicit `files`, `profile: "mos-llvm-c64"` and `preprocessing: "none"` or
`"cpp-unexpanded"`. Omit `slash_mode`; GAS slash settings are rejected for MOS.
Selectors require `language: "assembly"`, the same explicit profile, and either
`kind: "file"` or `"assembly_region"`. Only UTF-8 standalone `.s`/`.S` files
are supported. A suffix never triggers preprocessing or chooses a CPU.

Canonical MOS regions use standalone semicolon comments:

```asm
; erislint-region-begin sample
    lda #$01 ; raw, unvalidated operands
; erislint-region-end sample
```

The shared [region grammar](../../docs/fork/ASSEMBLY-CONTRACT-DETAILS.md)
requires unique matching names, nonempty bodies and no nesting. Hash, `//` and
block comments cannot create MOS markers. Inline markers and marker-like text
inside literals, macros, repetitions or continued CPP records are inert.

Semicolon and `//` introduce line comments; `/*...*/` is non-nesting. Hash starts
an additional comment only at statement start, apart from explicitly recognized
CPP directives. After a label, operand or block comment, hash stays punctuation.
Characters use paired quotes (`'A'`, `'\n'`); GAS quote-prefix characters are
rejected. Strings preserve UTF-8 and only the reviewed bounded escapes. Bare CR,
raw newlines in literals, unsupported escapes and unclosed tokens fail.

Labels, directional local references, numeric spellings, modifiers, directives
and instruction-like statements remain raw records. Includes, macros, conditionals,
aliases, relocations and assembler state remain unresolved in every context mode.
CPU/variant switches and alternate macro syntax are deliberately rejected.
No instruction legality, opcode support, undocumented 6510 features, address
validity, memory-map effects, timing, linking or execution is established.
`analysis.validated_cpu_features` is empty by design.

Assembly rules require the exact `insufficient_context` criterion in the example.
That answer emits the fixed inconclusive warning; severity overrides do not turn
it into a defect. Explicit `off` skips evaluation. Normal warning exit/display
flags apply. Rust/C/Python and x86 behavior and schemas remain covered separately.

See the [MOS checkpoint](../../docs/fork/ASSEMBLY-MOS-CHECKPOINT.md) for pinned
primary sources, test coverage and remaining limits. ca65, xa65, ACME, Kick
Assembler, embedded Rust assembly extraction and new editor activation are outside
this milestone.
