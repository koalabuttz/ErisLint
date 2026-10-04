<!-- Fork-specific addition. SPDX-License-Identifier: AGPL-3.0-only -->

# CLI getting started

This public fork extends [Eriskii's ErisLint](https://github.com/Eriskii/ErisLint)
with opt-in C, Python and bounded standalone assembly source review. It retains
the upstream Rust defaults, credit and AGPL-3.0-only license.

Use Rust 1.95 or newer to build the fork:

```sh
git clone --branch fork/milestone-1 https://github.com/koalabuttz/ErisLint.git
cd ErisLint
cargo build --locked
```

Start offline. These commands validate the C example and show the exact source
and questions that would be sent, without credentials or provider calls:

```sh
./target/debug/erislint --config examples/c/erislint.json --check-config
./target/debug/erislint --config examples/c/erislint.json --dry-run
```

Choose an existing example, keeping its configuration and referenced rule files
together. File patterns are relative to the configuration directory; adapt both
global `include` patterns and language-specific selection when using your source.

| Language/profile | Example configuration | Scope and limitations |
| --- | --- | --- |
| Rust | [Repository self-lint config](../../erislint.json), [unchanged rule](../../.erislint/rules/function-simplicity.json) | Version 1 defaults; [Rust extraction](RUST-EXTRACTION.md); no macro/type resolution |
| C | [C config](../../examples/c/erislint.json) | Opt-in version 2; `.c` and explicitly selected C headers; [C guide](../../examples/c/README.md) |
| Python | [Python config](../../examples/python/erislint.json) | Opt-in version 2; UTF-8 `.py`; [Python guide and grammar limits](../../examples/python/README.md) |
| x86 GAS AT&T32 | [x86 config](../../examples/assembly/erislint.json) | Version 3; explicit profile, preprocessing and GAS slash mode; [x86 guide](../../examples/assembly/README.md) |
| LLVM-MOS generic/C64 intent | [MOS config](../../examples/assembly-mos/erislint.json) | Version 3; explicit profile/preprocessing, no GAS slash mode; [MOS guide](../../examples/assembly-mos/README.md) |

Assembly supports standalone `.s`/`.S` file targets and explicitly marked review
regions. It preserves raw source and unresolved context; it does not establish
assembler acceptance, instruction legality, CPU features or runtime correctness.
Embedded Rust `asm!` and `global_asm!` are not extracted as assembly. C/Python
analysis likewise does not execute source, resolve all dependencies or prove
semantic correctness. Editor activation remains Rust-only.

For a live run, first review the offline requests, current provider pricing and
your spending allowance. Configure `jev_key` securely outside the repository;
the CLI does not load `.env` automatically or enforce a spending cap. Then use
an explicitly selected configuration, for example:

```sh
./target/debug/erislint --config examples/c/erislint.json --jobs 1 --format compact
```

Live results are review leads. Exit codes are `0` for no lint errors, `1` for
lint errors, and `2` for operational failure. `--deny-warnings` also makes
warnings exit `1`; `--errors-only` filters display without changing the exit
decision. Assembly `insufficient_context` produces a fixed inconclusive warning;
it is not a defect verdict. Explicit rule `off` skips evaluation.

See the [milestone completion record](MILESTONE-COMPLETE.md) for exact tested
implementation, compatibility evidence and the limited live self-review scope.
