use std::ops::{AddAssign, Index, IndexMut, Mul, Shr};

#[derive(Debug, Clone, Copy)]
pub struct Accumulator {
    /// A 32 bit value that can, during the execution of some GTE instructions,
    /// grow to a signed 43 bit value.
    pub value: i64,

    /// Whether the value has ever overflowed positively. This is not reset
    /// unless explicitly cleared.
    pub positive_overflow: bool,

    /// Whether the value has ever overflowed negatively. This is not reset
    /// unless explicitly cleared.
    pub negative_overflow: bool,
}

impl Default for Accumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Accumulator {
    // Constants for 44-bit signed integer limits
    const MAX: i64 = (1 << 43) - 1;
    const MIN: i64 = -(1 << 43);

    pub fn new() -> Self {
        Self {
            value: 0,
            positive_overflow: false,
            negative_overflow: false,
        }
    }

    /// Takes a new value and initializes the accumulator.
    /// The value is clamped to fit within the 44-bit signed integer range.
    pub fn set(&mut self, value: i64) {
        if value > Self::MAX {
            self.positive_overflow = true;
        } else if value < Self::MIN {
            self.negative_overflow = true;
        }

        // Ignore bits over 44, then sign extend as an i44
        self.value = (value << 20) >> 20;
    }

    pub fn get(&self) -> u32 {
        // Return the lower 32 bits of the accumulator value
        self.value as u32
    }

    /// Direct write to the accumulator does not check for overflow.
    pub fn write(&mut self, value: i32) {
        self.value = value as u32 as i64;
    }

    /// Consumes the accumulator, performing the final shift and returning a 32-bit integer.
    pub fn commit(self, shift_fractional: u32) -> i32 {
        (self.value >> shift_fractional) as i32
    }
}

impl AddAssign<i64> for Accumulator {
    fn add_assign(&mut self, rhs: i64) {
        self.set(self.value + rhs);
    }
}

impl AddAssign<i32> for Accumulator {
    fn add_assign(&mut self, rhs: i32) {
        self.set(self.value + (rhs as i64));
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Matrix(pub [Vector<i16>; 3]);

impl Matrix {
    /// Returns the row at the given index.
    pub fn row(&self, index: usize) -> Vector<i16> {
        self.0[index]
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Vector<T>(pub [T; 3]);

impl<T> Index<usize> for Vector<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<T> IndexMut<usize> for Vector<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl Mul<Vector<i16>> for Vector<i16> {
    type Output = Vector<i32>;

    fn mul(self, rhs: Vector<i16>) -> Self::Output {
        Vector([
            self.0[0] as i32 * rhs.0[0] as i32,
            self.0[1] as i32 * rhs.0[1] as i32,
            self.0[2] as i32 * rhs.0[2] as i32,
        ])
    }
}

impl Mul<i16> for Vector<i16> {
    type Output = Vector<i32>;

    fn mul(self, rhs: i16) -> Self::Output {
        Vector([
            self.0[0] as i32 * rhs as i32,
            self.0[1] as i32 * rhs as i32,
            self.0[2] as i32 * rhs as i32,
        ])
    }
}

impl Shr<usize> for Vector<i32> {
    type Output = Vector<i32>;

    fn shr(self, rhs: usize) -> Self::Output {
        Vector([
            self.0[0] >> rhs,
            self.0[1] >> rhs,
            self.0[2] >> rhs,
        ])
    }
}