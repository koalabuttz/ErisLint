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

No live provider calls were made for these corrections. Previous live review
results describe the prior implementation only. No thresholds were weakened.
VS Code activation remains Rust-only; extension-host testing remains unavailable
without `code`, and no hosted CI is configured. No assembly work was started.
