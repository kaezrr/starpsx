use std::ops::{Add, Sub};

#[derive(Debug, Clone, Copy)]
pub struct Vec2 {
    pub x: i32,
    pub y: i32,
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Vec2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Vec2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Vec2 {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    // ZERO vector
    pub fn zero() -> Self {
        Self { x: 0, y: 0 }
    }

    // Dot product with another vector
    pub fn dot(self, other: Vec2) -> i32 {
        self.x * other.x + self.y * other.y
    }

    // Return a 90 degrees clockwise rotated vector
    pub fn perpendicular(self) -> Self {
        Self {
            x: self.y,
            y: -self.x,
        }
    }
}

// Ensure that vertices v0, v1, v2 are in clockwise order
fn ensure_vertex_ordering(t: &mut [Vec2; 3]) {
    if !point_on_right_side(t[0], t[1], t[2]) {
        t.swap(0, 1);
    }
}

// Test whether point p lies on the right side of a -> b vector
fn point_on_right_side(a: Vec2, b: Vec2, p: Vec2) -> bool {
    let ap = p - a;
    let ab_perp = (b - a).perpendicular();
    ap.dot(ab_perp) >= 0
}

// Test if a point is inside triangle ABC
pub fn point_in_triangle(mut t: [Vec2; 3], p: Vec2) -> bool {
    ensure_vertex_ordering(&mut t);
    let side_ab = point_on_right_side(t[0], t[1], p);
    let side_bc = point_on_right_side(t[1], t[2], p);
    let side_ca = point_on_right_side(t[2], t[0], p);
    side_ab && side_bc && side_ca
}
