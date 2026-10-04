<!-- Fork-specific addition. SPDX-License-Identifier: AGPL-3.0-only -->

# Response consistency

Checked against the official [API reference](https://docs.typesafe.ai/api) and
[confidence documentation](https://docs.typesafe.ai/confidence) on 2026-10-04.
The API defines the selected choice as a highest-probability option and the
probability map as normalized. Neither page specifies a wire rounding precision
or tie-breaking rule. Examples use short decimals; that is not a precision
contract. The confidence page supplies a formula, but does not specify whether
its inputs precede rounding of the returned probabilities.

The fork validates before applying diagnostic policies:

- Required fields, known question/choice IDs and exact probability keys retain
  their existing checks. Probability and confidence values must be finite and
  within [0, 1], enforced by the existing `Probability` type.
- The probability sum may differ from 1 by at most `min(0.005 * n, 0.02)`, plus
  `1e-12` for floating-point arithmetic. Half a percentage point per option
  accommodates common two-decimal rounding (such as three 0.33 values). The
  two-percentage-point aggregate cap prevents large choice sets making this
  check meaningless. This is an explicit client compatibility allowance, not
  a provider precision guarantee. A distribution outside this allowance fails
  even if some hypothetical coarser rounding could explain it.
- The selected option must equal a reported maximum. Any tied maximum is valid;
  even a small strictly lower value is invalid. Sum tolerance never excuses a
  ranking reversal. No options are relabeled and no probabilities are normalized.
- Returned confidence remains range-checked but is not required to exactly match
  a derived formula, nor recalculated. Existing policies use the returned value.
  Cross-field confidence consistency remains an explicit limitation until a
  sufficient precision contract is available.

A complete but invalid provider response is an operational failure, not the
semantic `insufficient_context` option. It is rejected before diagnostic
selection and never retried. The CLI exits 2 without a successful report;
`--errors-only` cannot hide that failure. Existing transient transport retries
remain bounded at four attempts total per request. Other concurrent requests may
already be in flight when a failure occurs; use `--jobs 1` for tightly bounded
verification. Valid semantic abstentions continue to follow configured policies.

Tests use generic synthetic inputs only. No private project code, identifiers,
configuration, or raw provider records belong in these public fixtures.

## Local verification checkpoint

`cargo test --locked` passed all 126 tests (10 library, 2 CLI-unit, 114
integration), with zero failures or ignored tests. `cargo fmt --check`,
`cargo clippy --locked --all-targets -- -D warnings`, and `git diff --check`
passed. The extension's `npm test` compiled TypeScript and passed all six
protocol tests. Desktop extension-host tests remain unrun (`code` unavailable).

The unchanged self-lint policy passed offline `--check-config`; whole-project
`--dry-run` constructed 284 requests/questions across 28 files. This validates
configuration, extraction and request construction, not semantic model findings.
The CLI tests preserve warning thresholds and distinguish operational exit 2
from semantic warnings (exit 0 normally, exit 1 with `--deny-warnings`), including
`--errors-only` display filtering. Complete invalid responses are checked for
exactly one transport attempt; transient retries retain their four-attempt test.
