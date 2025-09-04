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
pub fn ensure_vertex_ordering(t: &mut [Vec2; 3], cols: Option<&mut [u16; 3]>) {
    if signed_area(t[0], t[1], t[2]) < 0 {
        t.swap(0, 1);
        if let Some(cols) = cols {
            cols.swap(0, 1);
        }
    }
}

// Signed area of the triangle a b p in clockwise order
fn signed_area(a: Vec2, b: Vec2, p: Vec2) -> i32 {
    let ap = p - a;
    let ab = b - a;
    (ap.x * ab.y - ab.x * ap.y) / 2
}

// Test if a point is inside triangle ABC
pub fn point_in_triangle(t: [Vec2; 3], p: Vec2) -> bool {
    let side_ab = signed_area(t[0], t[1], p) >= 0;
    let side_bc = signed_area(t[1], t[2], p) >= 0;
    let side_ca = signed_area(t[2], t[0], p) >= 0;
    side_ab && side_bc && side_ca
}

pub fn compute_barycentric_coords(t: [Vec2; 3], p: Vec2) -> Option<[f64; 3]> {
    let area_ab = signed_area(t[0], t[1], p);
    let area_bc = signed_area(t[1], t[2], p);
    let area_ca = signed_area(t[2], t[0], p);

    if area_ab < 0 || area_bc < 0 || area_ca < 0 {
        return None;
    }

    let denominator = signed_area(t[0], t[1], t[2]);
    if denominator == 0 {
        return Some([1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0]);
    }
    let denominator = f64::from(denominator);
    let weight0 = f64::from(area_bc) / denominator;
    let weight1 = f64::from(area_ca) / denominator;
    let weight2 = f64::from(area_ab) / denominator;

    Some([weight0, weight1, weight2])
}
