# Fork-specific synthetic source-review example. SPDX-License-Identifier: AGPL-3.0-only
#define FLAG unresolved_flag
.code32
.set COUNT, unresolved_count
.macro step amount
    addl $\amount, %eax
.endm
#if FLAG
# erislint-region-begin sample
1:  movl $1, %eax; step COUNT
    .ascii "é # ;"
    .byte 'A, '\n
    jmp 1b
# erislint-region-end sample
#endif
