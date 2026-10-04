# Fork-specific assembly source review

This synthetic example exercises the bounded `x86-gas-att32` profile. It is not
an assembler conformance test or runnable program. ErisLint retains Eriskii's
upstream branding and AGPL-3.0-only license.

Run these offline from the repository root:

```sh
cargo run --locked -- --config examples/assembly/erislint.json --check-config
cargo run --locked -- --config examples/assembly/erislint.json --dry-run
```

The example yields six evaluations: `file` and `assembly_region`, each in
`target`, `enclosing` and `file` context. No credentials or provider calls are
needed. These commands validate configuration and construct requests; they do
not perform semantic lint. Live provider evaluation requires separate spending
authorization.

Version 3 requires `assembly_sources` with explicit `files`, `profile`,
`preprocessing` (`none` or `cpp-unexpanded`) and `slash_mode` (`gas-default` or
`divide`). Global `include`/`exclude` still apply. Rule selectors must specify
`language: "assembly"` and `profile: "x86-gas-att32"`. Only standalone UTF-8
`.s`/`.S` files are supported. Overlapping selections fail instead of guessing.

Regions use standalone hash comments, with whitespace after the hash:

```asm
# erislint-region-begin sample
    movl $1, %eax
# erislint-region-end sample
```

Names must be unique ASCII identifiers matching
`[A-Za-z_][A-Za-z0-9_.-]{0,63}`. Regions cannot nest or have empty/whitespace-only
bodies. Marker-like text in strings, block comments, macros, repetitions or
opaque CPP directives is inert. File rules do not require regions; an active
region rule on a file without valid markers fails.

The scanner preserves byte ranges, comments, Unicode scalar columns and raw
records. It recognizes bounded GAS quoting, labels, directives, semicolon
statement boundaries, balanced macro/repetition/conditional structure and
explicit `.code32`/`.att_syntax`. CPP directives remain unexpanded and logical
continuations remain opaque. Macro bodies, includes, conditionals, symbols,
operands and CPU effects remain unresolved. Essential dependencies survive every
context mode. Unsupported mode switches, malformed recognized structure and
unsupported lexical forms fail conservatively. This is not full GAS grammar
validation. See the [exact contract](../../docs/fork/ASSEMBLY-CONTRACT-DETAILS.md).

Every assembly question must include `insufficient_context` with the exact
description shown in the JSON example. Policies must select substantive answers;
they cannot use uncertainty as a policy condition. An uncertainty answer emits
the fixed warning `Assembly review inconclusive: insufficient context.` even
under severity overrides. An explicit `off` setting skips evaluation. Existing
warning exit/display flags retain their normal behavior.

Version 3 can inherit older configs and use explicit rule imports such as
`{"path":"rules.json","version":2}`. String imports use the declaring config's
version; `$schema` does not override validation. Version 1/2 documents cannot
inherit version 3. Existing language defaults and schemas remain unchanged.

LLVM-MOS, embedded Rust assembly, instruction validation, symbol resolution,
linking, execution and VS Code assembly activation are outside this milestone.
