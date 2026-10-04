<!-- Fork-specific addition. SPDX-License-Identifier: AGPL-3.0-only -->

# Python milestone contract

This fork adds opt-in Python source inspection while retaining Eriskii's Rust
behavior and the existing opt-in C adapter. Assembly is outside this milestone.

Use the maintained [Tree-sitter Python grammar](https://github.com/tree-sitter/tree-sitter-python)
from crates.io, pinned to `tree-sitter-python = 0.25.0` (MIT). The supported parser
contract is that exact grammar's Python 3 syntax subset, not a claim of complete
CPython version conformance. Python 2 print/exec statements and backtick repr
syntax are explicitly rejected. Grammar errors or missing nodes fail analysis.
Newer syntax unsupported by this grammar must fail rather than be guessed.

Version 2 adds explicit `python_files` (`.py` only) and `where.language: python`.
Both are required together. Global include/exclude scope precedes language
selection; overlapping C/Python selections fail rather than choose a parser.
Default discovery and omitted rule language remain Rust. Version 1 schemas and
validation stay frozen; v2 schemas gain additive Python fields and a Python-only
`class` target. Methods and async functions use `function` targets with explicit
metadata. Language and parser options remain part of the preparation cache key.

Extract complete files, classes, functions, async functions and methods, including
nested definitions. Preserve source bytes, Unicode-aware spans, comments, raw
docstrings, decorators, parameters, annotations, bases and enclosing definitions.
Decorated target ranges include their decorators. File ranges include all bytes.
No imports, decorators, annotations, defaults, bodies or analyzed code execute.
Imports, dynamic attributes, metaclasses, symbol/type resolution and runtime
binding remain explicitly unknown. Lambdas are retained as source but are not
separate named-function targets. Active rules without supported targets fail;
explicit filters and disabled rules may intentionally produce no evaluations.

Commit sequence: dependency and contract; source extraction and tests; config and
runner integration; fixtures and user documentation; final verification record.
Run full offline regression checks at each checkpoint. Use the unchanged upstream
binary and unchanged rule for tightly bounded live implementation review, with
reservations made before requests. Report that coverage separately from offline
whole-tree dry-runs. Preserve original attribution and keep VS Code activation
unchanged.
