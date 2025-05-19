# MIPS Assembly Stress Test Program
# For ADDIU, ANDI, ORI, XORI, LUI, SB, SH, SW, LB, LBU, LH, LHU, LW
#
# Register Usage Plan:
# r1-r4: Initial patterns
# r5: Base address for memory operations (0x00000100)
# r6-r10: Results of logical/arithmetic ops
# r11-r14: Temporary values for storing
# r16-r25: Results of load operations
# r26-r29: Final checks on loaded values

.text
.globl main
.set noat
main:
    # Section 1: Register Initialization
    # r1 = 0xAAAAAAAA
    lui   $1, 0xAAAA          # $1 = 0xAAAA0000
    ori   $1, $1, 0xAAAA      # $1 = 0xAAAAAAAA

    # r2 = 0x55555555
    lui   $2, 0x5555          # $2 = 0x55550000
    ori   $2, $2, 0x5555      # $2 = 0x55555555

    # r3 = -1 (0xFFFFFFFF)
    lui   $3, 0xFFFF          # $3 = 0xFFFF0000
    ori   $3, $3, 0xFFFF      # $3 = 0xFFFFFFFF (or addiu $3, $0, -1)

    # r4 = 0x1234ABCD
    lui   $4, 0x1234          # $4 = 0x12340000
    ori   $4, $4, 0xABCD      # $4 = 0x1234ABCD

    # r5 = Base address for memory operations: 0x00000100
    lui   $5, 0x0000          # $5 = 0x00000000
    ori   $5, $5, 0x0100      # $5 = 0x00000100

    # Section 2: Logical & Arithmetic Operations
    # r6 = r1 ANDI 0x0000FF00 (tests ANDI with zero-extended immediate)
    andi  $6, $1, 0xFF00      # $1=0xAAAAAAAA, $6 = 0x0000AA00

    # r7 = r2 ORI 0x0000F0F0  (tests ORI with zero-extended immediate)
    ori   $7, $2, 0xF0F0      # $2=0x55555555, $7 = 0x5555F5F5

    # r8 = r4 XORI 0x0000FFFF (tests XORI with zero-extended immediate)
    xori  $8, $4, 0xFFFF      # $4=0x1234ABCD, $8 = 0x12345432

    # r9 = r1 + 0x1000 (tests ADDIU with positive immediate)
    addiu $9, $1, 0x1000      # $1=0xAAAAAAAA, $9 = 0xAAAABAAA

    # r10 = r3 + 1 (tests ADDIU with small positive immediate on negative number)
    addiu $10, $3, 1          # $3=0xFFFFFFFF, $10 = 0x00000000

    # Section 3: Storing to Memory (using r5 as base: 0x100)
    # Memory Address Map for Stores:
    # 0x100: Word from r4 (0x1234ABCD)
    # 0x104: Halfword from r1 (0xAAAA)
    # 0x106: Byte from r2 (0x55)
    # 0x108: Word 0xFF775500
    # 0x10C: Word 0x0000BEEF
    # 0x110: Word 0xNEGVAL (0xFFF01234)
    # 0x114: Byte from 0xNEGVAL (0x34)
    # 0x116: Halfword from 0xNEGVAL (0x1234)

    sw    $4, 0($5)           # Store r4 (0x1234ABCD) at 0x100
    sh    $1, 4($5)           # Store low half of r1 (0xAAAA) at 0x104
    sb    $2, 6($5)           # Store low byte of r2 (0x55) at 0x106

    # Prepare some more distinct values for storing
    lui   $11, 0xFF77         # $11 = 0xFF770000
    ori   $11, $11, 0x5500    # $11 = 0xFF775500 (a negative-ish number)
    sw    $11, 8($5)          # Store $11 at 0x108

    addiu $12, $0, 0xBEEF     # $12 = 0xFFFFBEEF
    sw    $12, 12($5)         # Store $12 at 0x10C

    lui   $13, 0xFFF0         # $13 = 0xFFF00000
    ori   $13, $13, 0x1234    # $13 = 0xFFF01234 (NEGVAL)
    sw    $13, 16($5)         # Store $13 at 0x110

    sb    $13, 20($5)         # Store low byte of $13 (0x34) at 0x114
    sh    $13, 22($5)         # Store low half of $13 (0x1234) at 0x116

    # Section 4: Loading from Memory (using r5 as base: 0x100)
    # LW: r16 from 0x100 (should be 0x1234ABCD)
    lw    $16, 0($5)

    # LH/LHU: r17/r18 from 0x104 (should be 0xAAAA)
    lh    $17, 4($5)          # Sign-extended: 0xFFFFAAAA
    lhu   $18, 4($5)          # Zero-extended: 0x0000AAAA

    # LB/LBU: r19/r20 from 0x106 (should be 0x55)
    lb    $19, 6($5)          # Sign-extended: 0x00000055 (since MSB of 0x55 is 0)
    lbu   $20, 6($5)          # Zero-extended: 0x00000055

    # LW: r21 from 0x108 (should be 0xFF775500)
    lw    $21, 8($5)

    # LW: r22 from 0x110 (should be 0xFFF01234 - NEGVAL)
    lw    $22, 16($5)

    # LB/LBU: r23/r24 from 0x114 (should be 0x34, from NEGVAL)
    lb    $23, 20($5)         # Sign-extended: 0x00000034 (MSB of 0x34 is 0)
    lbu   $24, 20($5)         # Zero-extended: 0x00000034

    # LH/LHU: r25/r26 from 0x116 (should be 0x1234, from NEGVAL)
    lh    $25, 22($5)         # Sign-extended: 0x00001234 (MSB of 0x1234 is 0)
    lhu   $26, 22($5)         # Zero-extended: 0x00001234

    # Section 5: Final Operations on Loaded Values
    # Test ADDIU on a sign-extended value (0xFFFFAAAA + 1 = 0xFFFFAAAB)
    addiu $27, $17, 1

    # Test ADDIU on a zero-extended value (0x0000AAAA + 1 = 0x0000AAAB)
    addiu $28, $18, 1
    
    # Test XORI to check if LB and LBU produced same result for positive byte
    # If $19 (LB result) and $20 (LBU result) are same, $29 will be 0
    xori  $29, $19, 0x0055     # $29 = $19 ^ 0x55. If $19 = 0x55, then $29 = 0
                               # This is a bit of a self-check.
                               # If $19 was 0xFFFFFF55, this would be different.
