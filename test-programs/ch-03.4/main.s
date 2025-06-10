.text
.globl main
main:
    addiu $t0, $zero, 0        # accumulator ← 0

# --- Test SLL ($t2 ← $t1 << 5; expect 3<<5 = 0x60) ---
    addiu $t1, $zero, 3
    sll   $t2, $t1, 5
    addiu $t3, $zero, 0x60
    xor   $t4, $t2, $t3
    or    $t0, $t0, $t4

# --- Test SRL ($t2 ← $t1 >> 1 logical; expect 0x80000000>>1 = 0x40000000) ---
    lui   $t1, 0x8000
    addiu $t1, $t1, 0         # $t1 = 0x80000000
    srl   $t2, $t1, 1
    lui   $t3, 0x4000
    addiu $t3, $t3, 0         # $t3 = 0x40000000
    xor   $t4, $t2, $t3
    or    $t0, $t0, $t4

# --- Test SRA ($t2 ← $t1 >> 1 arith; expect 0x80000001>>1 = 0xc0000000) ---
    lui   $t1, 0x8000
    addiu $t1, $t1, 1         # $t1 = 0x80000001
    sra   $t2, $t1, 1
    lui   $t3, 0xc000
    addiu $t3, $t3, 0         # $t3 = 0xc0000000
    xor   $t4, $t2, $t3
    or    $t0, $t0, $t4

# --- Test SLLV ($t2 ← $t1 << $t2; expect 2<<4 = 0x20) ---
    addiu $t1, $zero, 2
    addiu $t2, $zero, 4
    sllv  $t3, $t1, $t2
    addiu $t4, $zero, 0x20
    xor   $t5, $t3, $t4
    or    $t0, $t0, $t5

# --- Test SRLV ($t2 ← $t1 >> $t2 logical; expect 0x10>>2 = 0x4) ---
    addiu $t1, $zero, 0x10
    addiu $t2, $zero, 2
    srlv  $t3, $t1, $t2
    addiu $t4, $zero, 4
    xor   $t5, $t3, $t4
    or    $t0, $t0, $t5

# --- Test SRAV ($t2 ← $t1 >> $t2 arith; expect –1>>4 = –1) ---
    addiu $t1, $zero, -1
    addiu $t2, $zero, 4
    srav  $t3, $t1, $t2
    addiu $t4, $zero, -1
    xor   $t5, $t3, $t4
    or    $t0, $t0, $t5

# --- Test ADDU & SUBU ---
    addiu $t1, $zero, 123
    addiu $t2, $zero,  23
    addu  $t3, $t1, $t2       # expect 146
    addiu $t4, $zero, 146
    xor   $t5, $t3, $t4
    or    $t0, $t0, $t5

    subu  $t3, $t1, $t2       # expect 100
    addiu $t4, $zero, 100
    xor   $t5, $t3, $t4
    or    $t0, $t0, $t5

# --- Test AND, OR, XOR, NOR ---
    lui   $t1, 0xf0f0
    ori   $t1, $t1, 0xf0f0
    lui   $t2, 0x0ff0
    ori   $t2, $t2, 0x0ff0    # $t2 = 0x0ff00ff0

    and   $t3, $t1, $t2       # expect 0x00f000f0
    lui   $t4, 0x00f0
    ori   $t4, $t4, 0x00f0
    xor   $t5, $t3, $t4
    or    $t0, $t0, $t5

    or    $t3, $t1, $t2       # expect 0xfff0fff0
    lui   $t4, 0xfff0
    ori   $t4, $t4, 0xfff0
    xor   $t5, $t3, $t4
    or    $t0, $t0, $t5

    xor   $t3, $t1, $t2       # expect 0xff00ff <…>=0xff00ff<…>
    xor   $t4, $t1, $t2       # reuse: compute same, so diff=0
    xor   $t5, $t3, $t4
    or    $t0, $t0, $t5

    # NOR: ~(t1|t2):
    or    $t3, $t1, $t2       # t3 = t1|t2
    lui   $t4, 0xffff         # t4 = 0xffff0000
    ori   $t4, $t4, 0xffff    # t4 = 0xffffffff
    xor   $t3, $t3, $t4       # t3 = ~(t1|t2)
    # now expected is in t3; real: nor $t5,$t1,$t2
    nor   $t5, $t1, $t2
    xor   $t6, $t3, $t5
    or    $t0, $t0, $t6

# --- Test SLT & SLTU ---
    addiu $t1, $zero, 5
    addiu $t2, $zero, 10
    slt   $t3, $t1, $t2       # expect 1
    addiu $t4, $zero, 1
    xor   $t5, $t3, $t4
    or    $t0, $t0, $t5

    sltu  $t3, $t1, $t2       # expect 1
    addiu $t4, $zero, 1
    xor   $t5, $t3, $t4
    or    $t0, $t0, $t5
