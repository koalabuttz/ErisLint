# Fork-specific Rust compatibility fixtures

SPDX-License-Identifier: AGPL-3.0-only

Captured with the unmodified ErisLint binary built from upstream
`f04d016461b66b38d46647fb20762fb67bda350a`, before source refactoring.
`source.txt` intentionally has CRLF and non-BMP text preceding a Unicode name.
Tests copy it to `source.rs` in an isolated project alongside `config.json` as
`erislint.json`. No provider calls or credentials are involved.

`disk.json` is the exact stdout of `--dry-run`, including the final newline.
The identical `--stdin-file source.rs --dry-run` output is compared to the same
fixture. `selected.json` adds `--target-start` with the UTF-8 byte offset of
`café`. All target kinds, context modes, batched questions, declarations without
bodies, comments, nesting, external modules and unexpanded macros are represented.
Schemas are compared byte-for-byte to the existing upstream schema files.

Do not regenerate these fixtures merely to make a refactor pass. Any intended
contract change requires a separate feature review and documented fixture diff.
