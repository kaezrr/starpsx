use std::ops::{Add, Mul, Shl, Shr, Sub};

use super::*;
use util::checked_saturated;

#[derive(Default, Debug, Clone, Copy)]
pub struct Vector3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

impl Vector3<i16> {
    pub fn write_xy(&mut self, v: u32) {
        self.y = (v >> 16) as i16;
        self.x = (v & 0xFFFF) as i16;
    }

    pub fn xy(&self) -> u32 {
        (self.y as u32) << 16 | self.x as u32 & 0xFFFF
    }

    /// Sign extended z value
    pub fn zs(&self) -> u32 {
        self.z as u32
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

impl Sub for Vector3<i64> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.y - rhs.y,
        }
    }
}

impl Add for Vector3<i64> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.y + rhs.y,
        }
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

impl Shl<u8> for Vector3<i64> {
    type Output = Self;

    fn shl(self, rhs: u8) -> Self::Output {
        Self {
            x: self.x << rhs,
            y: self.y << rhs,
            z: self.z << rhs,
        }
    }
}

impl Mul<i64> for Vector3<i64> {
    type Output = Self;

    fn mul(self, rhs: i64) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}
