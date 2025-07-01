use std::ops::{AddAssign, Index, IndexMut, Mul, Not, Shl, Shr};

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

    pub fn new_with_value(value: i64) -> Self {
        let mut acc = Self::new();
        acc.set(value);
        acc
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
    pub fn commit(&mut self, shift_fractional: usize) {
        self.value >>= shift_fractional;
    }
}

pub trait Settable<T> {
    fn set(&mut self, value: T);
}

impl Settable<i64> for Accumulator {
    /// Takes a new value and initializes the accumulator.
    /// The value is clamped to fit within the 44-bit signed integer range.
    fn set(&mut self, value: i64) {
        if value > Self::MAX {
            self.positive_overflow = true;
        } else if value < Self::MIN {
            self.negative_overflow = true;
        }

        // Ignore bits over 44, then sign extend as an i44
        self.value = (value << 20) >> 20;
    }
}

impl Settable<i32> for Accumulator {
    fn set(&mut self, value: i32) {
        self.value = value as u32 as i64;
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
        Vector([self.0[0] >> rhs, self.0[1] >> rhs, self.0[2] >> rhs])
    }
}

impl Shl<usize> for Vector<i16> {
    type Output = Vector<i16>;

    fn shl(self, rhs: usize) -> Self::Output {
        Vector([self.0[0] << rhs, self.0[1] << rhs, self.0[2] << rhs])
    }
}

impl Vector<i16> {
    pub fn as_fixed(&self, shift: usize) -> Vector<Accumulator> {
        Vector([
            Accumulator::new_with_value((self[0] as i64) << shift),
            Accumulator::new_with_value((self[1] as i64) << shift),
            Accumulator::new_with_value((self[2] as i64) << shift),
        ])
    }
}

impl Vector<i32> {
    pub fn as_fixed(&self, shift: usize) -> Vector<Accumulator> {
        Vector([
            Accumulator::new_with_value((self[0] as i64) << shift),
            Accumulator::new_with_value((self[1] as i64) << shift),
            Accumulator::new_with_value((self[2] as i64) << shift),
        ])
    }
}

impl<T> Settable<Vector<i64>> for Vector<T>
where
    T: Settable<i64>,
{
    fn set(&mut self, value: Vector<i64>) {
        self[0].set(value[0]);
        self[1].set(value[1]);
        self[2].set(value[2]);
    }
}

impl<T> Settable<Vector<i32>> for Vector<T>
where
    T: Settable<i32>,
{
    fn set(&mut self, value: Vector<i32>) {
        self[0].set(value[0]);
        self[1].set(value[1]);
        self[2].set(value[2]);
    }
}

impl Vector<Accumulator> {
    pub fn flags(&self) -> u32 {
        let mut val = 0;

        for (i, acc) in self.0.iter().enumerate() {
            if acc.positive_overflow {
                val |= 1 << (30 - i);
            } else if acc.negative_overflow {
                val |= 1 << (27 - i);
            }
        }

        val
    }

    fn reset_flags(&mut self) {
        for acc in &mut self.0 {
            acc.positive_overflow = false;
            acc.negative_overflow = false;
        }
    }

    pub fn commit(&mut self, shift_fractional: usize) {
        for acc in &mut self.0 {
            acc.commit(shift_fractional);
        }
    }
}

impl<T> AddAssign<Vector<i64>> for Vector<T>
where
    T: AddAssign<i64>,
{
    fn add_assign(&mut self, rhs: Vector<i64>) {
        self[0].add_assign(rhs[0]);
        self[1].add_assign(rhs[1]);
        self[2].add_assign(rhs[2]);
    }
}

impl<T> AddAssign<Vector<i32>> for Vector<T>
where
    T: AddAssign<i32>,
{
    fn add_assign(&mut self, rhs: Vector<i32>) {
        self[0].add_assign(rhs[0]);
        self[1].add_assign(rhs[1]);
        self[2].add_assign(rhs[2]);
    }
}
