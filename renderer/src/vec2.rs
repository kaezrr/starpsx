use std::ops::{Add, Sub};

#[derive(Default, Debug, Clone, Copy)]
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

// Test that vertices v0, v1, v2 are in clockwise order
pub fn needs_vertex_reordering(t: &[Vec2; 3]) -> bool {
    signed_area(t[0], t[1], t[2]) < 0
}

// Signed area of the triangle a b p in clockwise order
fn signed_area(a: Vec2, b: Vec2, p: Vec2) -> i32 {
    let ap = p - a;
    let ab = b - a;
    ap.x * ab.y - ab.x * ap.y
}

fn is_top_left(a: Vec2, b: Vec2) -> bool {
    if a.y == b.y { a.x < b.x } else { a.y < b.y }
}

// Test if a point is inside triangle ABC
pub fn point_in_triangle(t: [Vec2; 3], p: Vec2) -> bool {
    let edges = [
        (signed_area(t[0], t[1], p), is_top_left(t[0], t[1])), // AB
        (signed_area(t[1], t[2], p), is_top_left(t[1], t[2])), // BC
        (signed_area(t[2], t[0], p), is_top_left(t[2], t[0])), // CA
    ];
    edges
        .into_iter()
        .all(|(area, top_left)| area > 0 || (area == 0 && top_left))
}

pub fn compute_barycentric_coords(t: [Vec2; 3], p: Vec2) -> [f64; 3] {
    let edges = [
        signed_area(t[0], t[1], p), // AB
        signed_area(t[1], t[2], p), // BC
        signed_area(t[2], t[0], p), // CA
    ];

    let denominator = signed_area(t[0], t[1], t[2]);
    if denominator == 0 {
        return [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0];
    }
    let denominator = f64::from(denominator);
    let weight0 = f64::from(edges[1]) / denominator;
    let weight1 = f64::from(edges[2]) / denominator;
    let weight2 = f64::from(edges[0]) / denominator;

    [weight0, weight1, weight2]
}
