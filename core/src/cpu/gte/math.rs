use std::ops::{Add, Mul, Shr};

use super::*;

// NOTE: Every operation promotes types to FixedI64 to avoid overflows

impl Mul<i64> for Vector3<i32> {
    type Output = Vector3<i64>;

    fn mul(self, rhs: i64) -> Self::Output {
        Vector3 {
            x: (self.x as i64) * rhs,
            y: (self.y as i64) * rhs,
            z: (self.z as i64) * rhs,
        }
    }
}

impl Mul<Vector3<i16>> for &Matrix3 {
    type Output = Vector3<i64>;

    fn mul(self, rhs: Vector3<i16>) -> Self::Output {
        let dot = |row_idx: usize, vec: Vector3<i16>| -> i64 {
            let m = &self.elems;
            let i = row_idx * 3;

            (m[i] as i64 * vec.x as i64)
                + (m[i + 1] as i64 * vec.y as i64)
                + (m[i + 2] as i64 * vec.z as i64)
        };

        Vector3 {
            x: dot(0, rhs),
            y: dot(1, rhs),
            z: dot(2, rhs),
        }
    }
}

impl<T> Add<Vector3<T>> for Vector3<T>
where
    T: Add<T, Output = T>,
{
    type Output = Vector3<T>;

    fn add(self, rhs: Vector3<T>) -> Self::Output {
        Vector3 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl<T> Shr<u8> for Vector3<T>
where
    T: Shr<u8, Output = T>,
{
    type Output = Vector3<T>;

    fn shr(self, rhs: u8) -> Self::Output {
        Vector3 {
            x: self.x >> rhs,
            y: self.y >> rhs,
            z: self.z >> rhs,
        }
    }
}

impl From<Vector3<i64>> for Vector3<i32> {
    fn from(value: Vector3<i64>) -> Self {
        Self {
            x: value.x as i32,
            y: value.y as i32,
            z: value.z as i32,
        }
    }
}

impl Vector3<i32> {
    // Saturates according to lm bit but rtpt doesnt follow it
    pub fn saturated(self, lm: SaturationRange, rtpt: bool) -> Self {
        let min = match lm {
            SaturationRange::Unsigned15 => 0,
            SaturationRange::Signed16 => -0x8000,
        };

        // IR3 always follows lm bit (bug)
        Self {
            x: self.x.clamp(if rtpt { 0 } else { min }, 0x7FFF),
            y: self.y.clamp(if rtpt { 0 } else { min }, 0x7FFF),
            z: self.z.clamp(min, 0x7FFF),
        }
    }
}

impl From<Vector3<i32>> for Vector3<i16> {
    fn from(value: Vector3<i32>) -> Self {
        Self {
            x: value.x as i16,
            y: value.y as i16,
            z: value.z as i16,
        }
    }
}
