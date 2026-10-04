; Fork-specific synthetic source-review example. SPDX-License-Identifier: AGPL-3.0-only
#define FLAG unresolved_flag
#include "missing.inc"
.section .text
.set COUNT, unresolved_count
.macro step amount
    lda #\amount
.endm
#if FLAG
; erislint-region-begin sample
1:  lda #$01 ; opaque operands
    step COUNT
    .ascii "é ; #"
    .byte 'A', '\n'
    bne 1b
; erislint-region-end sample
#endif
