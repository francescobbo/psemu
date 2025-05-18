use bitfield::bitfield;

bitfield! {
    /// Represents a 32-bit MIPS instruction, providing access to its fields
    #[derive(Copy, Clone)]
    pub struct Instruction(u32);
    impl Debug;

    /// The opcode field (6 bits) identifies the operation to be performed
    pub opcode, _: 31, 26;

    /// The RS field (5 bits) specifies the source register index
    pub u8, into usize, rs, _: 25, 21;

    /// The RT field (5 bits) specifies the target register index
    pub u8, into usize, rt, _: 20, 16;

    /// The immediate field (16 bits) is a signed value used in some
    /// instructions, signed-extended to 32 bits
    pub i16, into i32, simm16, _: 15, 0;

    /// The immediate field (16 bits) is an unsigned value used in some
    /// instructions, zero-extended to 32 bits
    pub imm16, _: 15, 0;
}
