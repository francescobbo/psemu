.text
.globl main
.set noreorder # prevent llvm from placing `nop`s after jumps

main:
    # Setup canaries
    addiu $10, $0, 0xff
    addu $11, $0, $0

    j continue
    sll $10, $10, 1

continue:
    # we expect $10 to be 0x1fe now

    j not_taken
    j taken
    sll $10, $10, 1

not_taken:
    # mark failure
    addiu $11, $0, 0xff

    j not_taken
    nop

taken:
    # we expect $10 to be 0x3fc now
    j not_taken
    beq $0, $0, taken2
    sll $10, $10, 1

filler:
    # mark failure
    addiu $11, $0, 0xff
    j filler
    nop

taken2:
    # we expect $10 to be 0x7f8 now
    j finish
    beq $10, $0, not_taken
    sll $10, $10, 1 # should not be executed

finish:
    j finish
    nop
