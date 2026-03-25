use derive_more::Add;
use derive_more::AddAssign;
use derive_more::Sub;

#[derive(Default, Debug, Clone, Copy, Add, Sub, AddAssign)]
pub struct Vec2 {
    pub x: i32,
    pub y: i32,
}

impl Vec2 {
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Evaluates the edge function E(x, y) = ax + by + c for point `p` relative to line `p0` -> `p1`.
///
/// Returns `(e, a, b)` where:
/// * `e`: Resulting value (0 on edge, >0 inside/normal side, <0 outside).
/// * `a`: Slope component (p0.y - p1.y).
/// * `b`: Slope component (p1.x - p0.x).
#[must_use]
pub const fn edge_function(p: Vec2, p0: Vec2, p1: Vec2) -> (i32, i32, i32) {
    let a = p0.y - p1.y;
    let b = p1.x - p0.x;
    let e = b * (p.y - p0.y) + a * (p.x - p0.x);
    (e, a, b)
}

// Test that vertices v0, v1, v2 are in clockwise order
#[must_use]
pub const fn needs_vertex_reordering(t: &[Vec2; 3]) -> bool {
    ((t[2].x - t[0].x) * (t[1].y - t[0].y) - (t[1].x - t[0].x) * (t[2].y - t[0].y)) > 0
}

// Test if edge AB is a top or left edge
#[must_use]
pub const fn is_top_left(a: Vec2, b: Vec2) -> bool {
    if a.y == b.y { a.x < b.x } else { a.y > b.y }
}
