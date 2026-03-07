use std::ops::{Add, AddAssign, Sub};

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

impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
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
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Edge function = ax + by + c
/// (= 0) if P is on edge
/// (> 0) if P is on the same side of the edge normal
/// (< 0) if P is on the opposite side of the edge normal
/// Return edge value, coefficient a and coefficient b
pub fn edge_function(p: Vec2, p0: Vec2, p1: Vec2) -> (i32, i32, i32) {
    let a = p0.y - p1.y;
    let b = p1.x - p0.x;
    let e = b * (p.y - p0.y) + a * (p.x - p0.x);
    (e, a, b)
}

// Test that vertices v0, v1, v2 are in clockwise order
pub fn needs_vertex_reordering(t: &[Vec2; 3]) -> bool {
    signed_area(t[0], t[1], t[2]) > 0
}

fn signed_area(a: Vec2, b: Vec2, p: Vec2) -> i32 {
    (p.x - a.x) * (b.y - a.y) - (b.x - a.x) * (p.y - a.y)
}

// Test if edge AB is a top or left edge
pub fn is_top_left(a: Vec2, b: Vec2) -> bool {
    if a.y == b.y { a.x > b.x } else { a.y < b.y }
}
