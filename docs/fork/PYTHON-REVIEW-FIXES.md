<!-- Fork-specific addition. SPDX-License-Identifier: AGPL-3.0-only -->

# Python review corrections

Independent review of `eb4be763d8abc9949cb7d14b53be5115afdbe353` identified a
reproducible docstring classification defect and an inaccurate grammar description.
These are corrected in separate focused commits.

## Docstring classification

The old implementation inspected only the first named child of an expression
statement. As a result, `'not docs', 1` and `'not docs',` became docstrings, while
`('real docs')` did not. Both failures were reproduced before correction.

The classifier now requires one standalone expression with no tuple comma, then
unwraps parentheses to classify string literals and literal concatenation. The
record retains the original outer expression's source bytes and span, including
parentheses, whitespace and comments; it does not decode or evaluate string values.
Tuples, bytes literals, f-strings and other nonconstant expressions remain excluded.
Tests cover file/class/function/method scopes, nested parentheses, multiline
concatenation and comments, singleton/multiple tuples and nonconstant expressions.
All target source remains preserved independently of docstring classification.

## Precise grammar contract

The supported contract is the permissive pinned `tree-sitter-python` 0.25.0 grammar
with explicit exclusions for Python 2 print/exec/backtick syntax and empty suites.
It is not a Python 3 grammar subset or a Python version/conformance validator.
Other legacy syntax remains accepted: `1L`, `a <> b`, and tuple-unpacking function
parameters. A checked-in source fixture verifies these forms without execution.
Grammar errors and missing nodes remain operational failures. Request metadata
and documentation now state the same boundary; only the syntax-contract text
changes in the frozen Python example requests.

## Verification and limits

All 81 Rust tests pass, including 19 Python and 19 C contracts. Six extension
protocol tests, TypeScript compilation, formatting, strict Clippy and offline
configuration validation pass. Offline whole-project self-lint builds 197
questions across 21 Rust files; output matches the pristine upstream binary
byte-for-byte on the same sources. Frozen Rust/C request fixtures and legacy
schemas remain unchanged and passing.

No live provider calls were made while implementing these corrections. The final
bounded review below subsequently covered the two classification functions.
Earlier live results describe the prior implementation only. No thresholds were weakened.
VS Code activation remains Rust-only; extension-host testing remains unavailable
without `code`, and no hosted CI is configured. No assembly work was started.

## Final bounded live review

After independent approval, the pristine binary from upstream
`f04d016461b66b38d46647fb20762fb67bda350a` reviewed `python::docstring` and
`python::single_expression` at exact fork commit
`28045dbc227b791ae0dde1fbdf78f6c3c9fcd9da`. The original configuration and
function-simplicity rule were unchanged. Exact function-name byte selection and
one question per evaluation were verified offline before requests.

The batch reserved up to four attempts per evaluation before sending. It completed
two evaluations in two API calls with zero retries, warnings or errors. Model
`jev-1.13.0` selected `simple` for both: confidence 0.92 for `docstring` and 0.51
for `single_expression`. Both results were reviewed; the helper's tuple and trivia
checks are required by the regression cases, and no code changes were warranted.
No diagnostic false positives were identified.

Coverage is limited to these two Rust classification functions. Test functions,
the metadata wording correction and the whole tree were not live-reviewed in
this batch. These judgments do not establish Python semantic correctness.
All prior spending reservations remain retained; actual billed token usage is
not exposed by the unchanged CLI. Private accounting remains outside this repo.
This closeout changes documentation only, so the 81-test offline checkpoint above
continues to describe the implementation. No further live calls were made.
