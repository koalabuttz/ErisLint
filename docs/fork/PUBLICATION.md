<!-- Fork-specific addition. SPDX-License-Identifier: AGPL-3.0-only -->

# Rust checkpoint publication and live review

The historical [baseline](BASELINE.md) and [extraction report](RUST-EXTRACTION.md)
predate successful publication and live self-lint. Their access blockers describe
that earlier session, not the current project status.

Published branch: [fork/milestone-1](https://github.com/koalabuttz/ErisLint/tree/fork/milestone-1).
Reviewed Rust checkpoint: `c765486f4a7c437dc91d5fd8d0070a58c573bdf0`.
The six focused commits were published through the GitHub connector. It did not
support retaining original author/committer timestamps and metadata; messages,
ordered parentage from the upstream base, exact trees, file bytes and modes were
verified after fetching. Original local history is retained separately. New work
starts from the published checkpoint rather than replaying duplicate history.

| Original local commit | Published commit |
| --- | --- |
| `4dc250c3714411b34207d745266f21210e980f1a` | `c5416998113e161bb5e72fdebacc4ba1ecae92f9` |
| `d4f1a76d4033931337673f7e5f41f306ec137de8` | `72a8e9046e2f6fe0e82449612e50b9d920be5330` |
| `8b93f218225125e92b2dd3befc7885dde04eb84c` | `31194ef8766192016140ece8c3171d6cdad16230` |
| `7a3c3b4092d0488cffde89d6596f1cd9052d5815` | `cc7c6860c4008c0cfd6739fd25ab16633103855b` |
| `4f48c1957d59f3a7847fbebf3842edbc1fabdb91` | `ac7da8e6173f8132b22f8d5da80c4fedaa0e6495` |
| `9b99e98a1054be8561b830992f3d0b1639640124` | `c765486f4a7c437dc91d5fd8d0070a58c573bdf0` |

On 2026-10-04, the unmodified upstream binary from
`f04d016461b66b38d46647fb20762fb67bda350a` evaluated the modified Rust checkpoint
with the unchanged upstream config and simplicity rule, at one concurrent
request. This was an actual live check, separate from the earlier offline
compatibility comparisons:

- 16 source files, 124 evaluations/questions, 124 attempts and zero retries.
- Returned model: `jev-1.13.0`.
- 122 `simple`, two `insufficient_context`, zero warnings and zero errors.
- Uncertain targets: `src/config.rs::version` and `tests/common/mod.rs::rule`.
  Neither triggered a diagnostic. No rule thresholds or source were changed to
  obtain a pass, and no diagnostic false positives were observed.

These are model judgments, not correctness proofs. The unchanged CLI does not
expose billed token usage. Credentials and private accounting are not part of
this public report. VS Code extension-host testing remains unrun because the
`code` executable is unavailable; the six protocol tests passed. No hosted CI
checks were reported at publication.
