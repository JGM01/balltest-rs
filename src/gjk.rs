// src/gjk.rs
use crate::components::Transform;
use crate::shape::{Shape, Vec2};

const GJK_MAX_ITERATIONS: usize = 32;
const EPA_MAX_ITERATIONS: usize = 32;
const EPA_TOLERANCE: f32 = 0.0001;

/// Minkowski difference support function
fn support(
    shape_a: &dyn Shape,
    tf_a: &Transform,
    shape_b: &dyn Shape,
    tf_b: &Transform,
    direction: Vec2,
) -> Vec2 {
    let pa = shape_a.support(tf_a, direction);
    let pb = shape_b.support(tf_b, -direction);
    pa - pb
}

/// Simplex for GJK
struct Simplex {
    points: Vec<Vec2>,
}

impl Simplex {
    fn new() -> Self {
        Self { points: Vec::new() }
    }

    fn push(&mut self, point: Vec2) {
        self.points.push(point);
    }

    fn len(&self) -> usize {
        self.points.len()
    }

    fn get(&self, i: usize) -> Vec2 {
        self.points[i]
    }

    /// Check if simplex contains origin and update simplex/direction
    fn contains_origin(&mut self, direction: &mut Vec2) -> bool {
        match self.len() {
            2 => self.line_case(direction),
            3 => self.triangle_case(direction),
            _ => false,
        }
    }

    fn line_case(&mut self, direction: &mut Vec2) -> bool {
        let a = self.get(1);
        let b = self.get(0);

        let ab = b - a;
        let ao = -a;

        // Direction perpendicular to AB towards origin
        let ab_perp = Vec2::new(-ab.y, ab.x);
        if ab_perp.dot(&ao) > 0.0 {
            *direction = ab_perp;
        } else {
            *direction = Vec2::new(ab.y, -ab.x);
        }

        false
    }

    fn triangle_case(&mut self, direction: &mut Vec2) -> bool {
        let a = self.get(2);
        let b = self.get(1);
        let c = self.get(0);

        let ab = b - a;
        let ac = c - a;
        let ao = -a;

        let ab_perp = Vec2::new(-ab.y, ab.x);
        let ac_perp = Vec2::new(ac.y, -ac.x);

        // Check if origin is on the AC side
        if ac_perp.dot(&ao) > 0.0 {
            // Remove B
            self.points.remove(1);
            *direction = ac_perp;
            return false;
        }

        // Check if origin is on the AB side
        if ab_perp.dot(&ao) > 0.0 {
            // Remove C
            self.points.remove(0);
            *direction = ab_perp;
            return false;
        }

        // Origin is inside triangle
        true
    }
}

/// GJK collision detection - returns true if shapes intersect
pub fn gjk(shape_a: &dyn Shape, tf_a: &Transform, shape_b: &dyn Shape, tf_b: &Transform) -> bool {
    // Initial direction
    let center_a = Vec2::new(tf_a.position[0], tf_a.position[1]);
    let center_b = Vec2::new(tf_b.position[0], tf_b.position[1]);
    let mut direction = center_b - center_a;

    if direction.length_sq() < 1e-6 {
        direction = Vec2::new(1.0, 0.0);
    }

    let mut simplex = Simplex::new();
    simplex.push(support(shape_a, tf_a, shape_b, tf_b, direction));

    direction = -simplex.get(0);

    for _ in 0..GJK_MAX_ITERATIONS {
        let a = support(shape_a, tf_a, shape_b, tf_b, direction);

        // If new point doesn't pass origin, no collision
        if a.dot(&direction) < 0.0 {
            return false;
        }

        simplex.push(a);

        if simplex.contains_origin(&mut direction) {
            return true;
        }
    }

    false
}

/// Edge in EPA polytope
#[derive(Clone, Copy)]
struct Edge {
    distance: f32,
    normal: Vec2,
    index: usize,
}

/// EPA - Expanding Polytope Algorithm
/// Returns penetration depth and collision normal
pub fn epa(
    shape_a: &dyn Shape,
    tf_a: &Transform,
    shape_b: &dyn Shape,
    tf_b: &Transform,
    initial_simplex: Vec<Vec2>,
) -> Option<(f32, Vec2)> {
    let mut polytope = initial_simplex;

    for _ in 0..EPA_MAX_ITERATIONS {
        let edge = find_closest_edge(&polytope);

        let new_point = support(shape_a, tf_a, shape_b, tf_b, edge.normal);
        let distance = new_point.dot(&edge.normal);

        // If new point is not significantly further, we've found the edge
        if distance - edge.distance < EPA_TOLERANCE {
            return Some((edge.distance, edge.normal));
        }

        // Insert new point into polytope
        polytope.insert(edge.index, new_point);
    }

    // Fallback if we hit iteration limit
    let edge = find_closest_edge(&polytope);
    Some((edge.distance, edge.normal))
}

fn find_closest_edge(polytope: &[Vec2]) -> Edge {
    let mut closest = Edge {
        distance: f32::INFINITY,
        normal: Vec2::zero(),
        index: 0,
    };

    for i in 0..polytope.len() {
        let j = (i + 1) % polytope.len();

        let a = polytope[i];
        let b = polytope[j];

        let e = b - a;
        let n = Vec2::new(e.y, -e.x).normalized(); // Perpendicular, pointing outward

        let distance = n.dot(&a);

        if distance < closest.distance {
            closest.distance = distance;
            closest.normal = n;
            closest.index = j;
        }
    }

    closest
}

/// Run GJK then EPA if collision detected
pub fn gjk_epa(
    shape_a: &dyn Shape,
    tf_a: &Transform,
    shape_b: &dyn Shape,
    tf_b: &Transform,
) -> Option<(f32, Vec2)> {
    // Run GJK with simplex tracking
    let center_a = Vec2::new(tf_a.position[0], tf_a.position[1]);
    let center_b = Vec2::new(tf_b.position[0], tf_b.position[1]);
    let mut direction = center_b - center_a;

    if direction.length_sq() < 1e-6 {
        direction = Vec2::new(1.0, 0.0);
    }

    let mut simplex = Simplex::new();
    simplex.push(support(shape_a, tf_a, shape_b, tf_b, direction));

    direction = -simplex.get(0);

    for _ in 0..GJK_MAX_ITERATIONS {
        let a = support(shape_a, tf_a, shape_b, tf_b, direction);

        if a.dot(&direction) < 0.0 {
            return None; // No collision
        }

        simplex.push(a);

        if simplex.contains_origin(&mut direction) {
            // Collision detected - run EPA
            return epa(shape_a, tf_a, shape_b, tf_b, simplex.points);
        }
    }

    None
}

/// Optimized circle-circle collision
pub fn circle_circle_collision(
    radius_a: f32,
    pos_a: Vec2,
    radius_b: f32,
    pos_b: Vec2,
) -> Option<(f32, Vec2)> {
    let delta = pos_b - pos_a;
    let dist_sq = delta.length_sq();
    let total_radius = radius_a + radius_b;

    if dist_sq < total_radius * total_radius {
        let dist = dist_sq.sqrt();
        let penetration = total_radius - dist;

        let normal = if dist > 1e-6 {
            delta * (1.0 / dist)
        } else {
            Vec2::new(0.0, 1.0)
        };

        Some((penetration, normal))
    } else {
        None
    }
}
