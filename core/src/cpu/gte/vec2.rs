use std::ops::Sub;

#[derive(Default, Debug, Clone, Copy)]
pub struct Vector2<T> {
    pub x: T,
    pub y: T,
}

impl Vector2<i16> {
    pub fn from_u32(v: u32) -> Self {
        Self {
            y: (v >> 16) as i16,
            x: (v & 0xFFFF) as i16,
        }
    }

    pub fn write_u32(&mut self, v: u32) {
        *self = Self::from_u32(v);
    }

    pub fn as_u32(&self) -> u32 {
        (self.y as u32) << 16 | (self.x as u32) & 0xFFFF
    }
}

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
