<!-- Fork-specific addition. SPDX-License-Identifier: AGPL-3.0-only -->

# Upstream baseline evidence

Recorded 2026-10-04 against unmodified upstream source at
`f04d016461b66b38d46647fb20762fb67bda350a` from
[Eriskii/ErisLint](https://github.com/Eriskii/ErisLint).
These results establish a regression baseline, not support for new languages.
See [the phased plan](PLAN.md) before implementing an adapter.

## Review and isolation

Read the upstream README, package manifests, self-lint config and referenced
rule, parser/runner interfaces, CLI offline-mode routing, tests, and
[`skills/erislint/SKILL.md`](../../skills/erislint/SKILL.md). No `AGENTS.md` or
additional `.agents` skills were present in the cloned repository. Preserve
Eriskii's original notices, manifests, branding and license unchanged.

Used a fresh full-history clone and isolated Rust 1.95.0 installation. No source,
Git history, configuration or evidence from another project was imported.
Dependencies came from the committed lockfiles. No provider credentials were
supplied to these checks (`jev_key` was removed from their environment).

## Checks actually run

Environment: Linux x86-64; rustc 1.95.0 (59807616e); Node v24.19.0;
npm 11.9.0. Commands below are relative to the repository unless stated otherwise.

| Check | Result |
| --- | --- |
| `cargo fmt --check` | Passed |
| `cargo test --locked` | Passed: 33 tests (7 library, 6 CLI, 15 contracts, 5 output); no failures or ignored tests; binary/doc test suites contain 0 tests |
| `cargo clippy --locked --all-targets -- -D warnings` | Passed |
| `npm ci --ignore-scripts --no-audit --no-fund` in `vscode/` | Installed 287 packages from lockfile; lifecycle scripts disabled; dependency deprecation notices only |
| `npm test` in `vscode/` | TypeScript compilation passed; 5 protocol tests passed |
| `target/debug/erislint --config erislint.json --check-config` | Passed: 1 rule |
| `target/debug/erislint --config erislint.json --dry-run` | Passed: 13 files, 108 evaluations, 108 questions |
| `git diff --check` | Passed for fork documentation |

The Rust API tests use local mock servers; the protocol tests use fixture
processes. No live Jev evaluations were made. Offline self-lint validates
configuration, parsing and request construction only; it reports neither live
findings nor semantic lint success. The original simplicity rule and thresholds
were not changed.

Full dry-run JSON SHA-256 (stdout, including its trailing newline):
`54317df632ec23b2c4c0a8b38fa0b04335662bbddf16827767d3fcb172ddc012`.
Reproduce with the pinned source and lockfile, the explicit config above and
redirected stdout. Keep the full request snapshot local for comparison during
the first refactors rather than committing duplicated source into documentation.

## Limits and milestone status

- VS Code extension-host tests were not run: the `code` executable is absent.
  Protocol tests passed; desktop integration and packaging were not validated.
- Live self-lint was not run. The tool catalog exposes no dedicated Jev tool;
  this does not establish that a managed network-secret route is unavailable.
  Network access is enabled, but the declared capabilities provide no Jev-specific
  secret-injection route or usage contract. That route remains unverified;
  credentials were not probed. Current pricing and an enforceable cumulative
  reservation mechanism were not established. No live findings or false
  positives can be reported.
- Public fork creation was attempted through the authenticated GitHub CLI and
  returned HTTP 403, `Resource not accessible by integration`. No fork URL or
  remote documentation commit is verified. This local baseline remains useful,
  but publication requires supported fork access. No upstream PR was opened.
- Only fork-specific documentation has changed. No language refactor, parser
  dependency change, C/Python/assembly implementation or upstream history rewrite
  has started. The first source refactor follows the plan-review checkpoint.
