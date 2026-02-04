use std::ops::{Add, Mul, Shr, Sub};

use super::*;

// NOTE: Every operation promotes types to i64 to avoid overflows

impl Sub for Vector2<i16> {
    type Output = Vector2<i64>;

    fn sub(self, rhs: Self) -> Self::Output {
        Vector2 {
            x: self.x as i64 - rhs.x as i64,
            y: self.y as i64 - rhs.y as i64,
        }
    }
}

impl Vector2<i64> {
    pub fn cross(self, rhs: Self) -> i64 {
        // Formula: (x1 * y2) - (y1 * x2)
        self.x * rhs.y - self.y * rhs.x
    }
}

impl Shr<u8> for Vector3<i64> {
    type Output = Self;

    fn shr(self, rhs: u8) -> Self::Output {
        Self {
            x: self.x >> rhs,
            y: self.y >> rhs,
            z: self.z >> rhs,
        }
    }
}

impl Vector3<i64> {
    pub fn cross(self, rhs: Self) -> Vector3<i64> {
        Vector3 {
            x: (self.y * rhs.z) - (self.z * rhs.y),
            y: (self.z * rhs.x) - (self.x * rhs.z),
            z: (self.x * rhs.y) - (self.y * rhs.x),
        }
    }

    /// Saturates according to lm bit, also returns saturation status of each field
    pub fn saturated(self, lm: SaturationRange) -> (Vector3<i16>, [bool; 3]) {
        let min = match lm {
            SaturationRange::Signed16 => -0x8000,
            SaturationRange::Unsigned15 => 0,
        };

        let (x, flag_x) = checked_saturated(self.x as i32, min, 0x7FFF);
        let (y, flag_y) = checked_saturated(self.y as i32, min, 0x7FFF);
        let (z, flag_z) = checked_saturated(self.z as i32, min, 0x7FFF);

        (
            Vector3 {
                x: x as i16,
                y: y as i16,
                z: z as i16,
            },
            [flag_x, flag_y, flag_z],
        )
    }
}

/// Saturate within [min..=max] and also return whether it overflowed
fn checked_saturated<T>(curr: T, min: T, max: T) -> (T, bool)
where
    T: PartialOrd<T>,
{
    if curr < min {
        return (min, true);
    }

    if curr > max {
        return (max, true);
    }

    (curr, false)
}

/// 44-bit math helper
fn m44(t: i64) -> i64 {
    (t << 20) >> 20
}
