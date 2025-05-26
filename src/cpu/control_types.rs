use bitfield::bitfield;

/// Represents one of the possible reasons for an exception in the MIPS
/// architecture.
#[derive(Debug, Clone)]
pub enum ExceptionCause {
    /// A syscall instruction was executed.
    Syscall = 8,
    /// A breakpoint instruction was executed, or a hardware breakpoint was hit.
    Breakpoint = 9,
}

impl Into<u32> for ExceptionCause {
    /// Defines how to convert an `ExceptionCause` into a raw `u32` value.
    fn into(self) -> u32 {
        self as u32
    }
}

impl From<u32> for ExceptionCause {
    /// Defines how to convert a raw `u32` value into an `ExceptionCause`.
    fn from(value: u32) -> Self {
        match value {
            8 => ExceptionCause::Syscall,
            9 => ExceptionCause::Breakpoint,
            _ => panic!("Invalid exception cause value: {}", value),
        }
    }
}

bitfield! {
    /// Defines the COP0 Cause register and its fields.
    #[derive(Copy, Clone)]
    pub struct Cause(u32);
    impl Debug;

    /// The exception code bits, which indicate why an exception or interrupt
    /// has occurred.
    pub u32, from into ExceptionCause, exception_code, set_exception_code: 6, 2;
}

impl Default for Cause {
    /// Initializes the Cause register with a default value.
    fn default() -> Self {
        Cause(0)
    }
}

bitfield! {
    /// Defines the COP0 Status register and its fields.
    #[derive(Copy, Clone)]
    pub struct Status(u32);
    impl Debug;

    /// Causes an exception to be raised when the CPU is in kernel mode.
    pub interrupt_enable, _: 0;
    /// Is the CPU in kernel mode?
    pub kernel_mode, _: 1;
    /// Previous interrupt enable state.
    pub interrupt_enable_previous, _: 2;
    /// Previous kernel mode state.
    pub kernel_mode_previous, _: 3;
    /// Nested exception - previous interrupt enable state.
    pub interrupt_enable_old, _: 4;
    /// Nested exception - previous kernel mode state.
    pub kernel_mode_old, _: 5;

    /// Helper to work on the low 6 bits of the Status register.
    pub low_fields, set_low_fields: 5, 0;

    /// Whether the I-Cache is mounted to the memory address space.
    pub isolate_cache, _: 16;

    // Set to 1 to use the boot exception vectors in kseg1, instead of the
    // normal exception vectors in kseg0.
    pub boot_exception_vectors, _: 22;
}

impl Default for Status {
    /// Initializes the Status register with a default value.
    fn default() -> Self {
        Status(1 << 22)
    }
}
