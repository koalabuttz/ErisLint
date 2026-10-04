<!-- Fork-specific addition. SPDX-License-Identifier: AGPL-3.0-only -->

# Python milestone checkpoint

Built on the reviewed C-fix checkpoint
`6e9f39e4ea7e6305a68ff08c2c6d6a9bc40f84d7`, preserving Eriskii's upstream history,
attribution and Rust defaults. Assembly remains unimplemented.

## Delivered

- Explicit version-2 `python_files` selection and Python-scoped rules, with
  function, async function, method, class and complete-file targets. No Python
  code is imported or executed. See the [runnable example](../../examples/python/README.md)
  and [grammar/config contract](PYTHON-DESIGN.md).
- Original comments, raw literal docstrings, decorators, annotations, type
  parameters, bases, bodies, nesting and Unicode-aware spans. Decorated ranges
  include decorators; file ranges include every input byte. Context explicitly
  marks unresolved dynamic behavior and incomplete analysis.
- Shared language-aware preparation caching, batching, policies, diagnostics and
  CLI editor snapshots. Global scope precedes language selection; overlapping
  C/Python globs and selected unsupported extensions fail. Active rules with no
  matching target cannot silently pass. Explicit filters/off overrides may skip.
- Additive v2 schema fields; independently frozen v1 target enum preserves the
  original schema bytes and rejects Python fields/classes in v1. Omitted language
  and default discovery remain Rust. C requests remain unchanged.

## Offline evidence

All 78 Rust tests pass, including 16 Python contracts and 19 C contracts. Six
extension protocol tests, TypeScript compilation, formatting and strict Clippy
pass. Tests cover metadata and spans, malformed/legacy syntax, empty suites,
configuration and inheritance, global scope, overlap errors, mixed-language cache
ordering, unsaved CLI snapshots, diagnostics, invalid UTF-8 and full example
request bytes. The Python example validates offline and yields three targets in
one file. An additional offline probe confirms Python 3 `print(...)`/`exec(...)`
calls parse as source while comment-only suites fail.

Whole-project offline self-lint validates the unchanged original rule and builds
193 questions across 21 Rust files. Its output matches the pristine upstream
binary byte-for-byte on those same current sources. All frozen Rust and C request
fixtures and legacy schema outputs remain passing. Offline request construction
does not constitute semantic lint.

## Bounded live review

The pristine binary from upstream
`f04d016461b66b38d46647fb20762fb67bda350a`, with the unchanged original config and
function-simplicity rule, reviewed exactly four C-fix functions: `c::extract`,
`c::target`, `Plan::from_source`, and `runner::source_files`. It then reviewed
exactly four Python-implementation functions: `python::extract`, `python::target`,
`Adapter::for_path`, and `Config::load`. Each review used an unchanged source
snapshot and exact function-name byte selection, verified offline first.

Each batch completed four evaluations in four attempts, with zero retries,
warnings or errors. Model `jev-1.13.0` selected `simple` for all eight answers.
The results were reviewed; no diagnostic-driven changes were warranted and no
diagnostic false positives were identified. Existing policies were not weakened.

Live coverage is deliberately limited to those eight Rust functions, not all
changed code or the whole project. Custom Python/C rules were not live-evaluated.
Model judgments do not prove correctness. Worst-case attempt reservations were
made before requests within the separately authorized project limit; all prior
reservations were retained. The original CLI does not expose billed token usage.
Private credentials and accounting stay outside this public repository.

## Limits and next step

The exact parser contract is `tree-sitter-python` 0.25.0 with additional rejection
of empty suites and Python 2 print/exec/backtick syntax. This is not full CPython
conformance or runtime validation. UTF-8 `.py` sources only; no notebooks, stubs,
bytecode, import/type resolution, dynamic execution or separate lambda targets.
VS Code activation remains Rust-only. Extension-host testing remains unavailable
without `code`; no hosted CI is configured. Independent Python review is next;
assembly needs its own explicit dialect/architecture scope.
