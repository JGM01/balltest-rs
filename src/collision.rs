use crate::components::Transform;

/// Trait that any collision shape must implement
/// This provides the geometric queries needed for collision detection
pub trait CollisionShape: std::fmt::Debug {
    /// Get the axis-aligned bounding box for broad-phase culling
    /// Returns (min_x, min_y, max_x, max_y) in world space
    fn aabb(&self, transform: &Transform) -> (f32, f32, f32, f32);

    /// Find the closest point on this shape's surface to the given world-space point
    /// This is the key function that enables generalized collision detection
    fn closest_point(&self, transform: &Transform, point: [f32; 2]) -> [f32; 2];

    /// Get the center point of this shape in world space
    fn center(&self, transform: &Transform) -> [f32; 2] {
        transform.position
    }

    /// Check if a world-space point is inside this shape (for clickability)
    fn contains_point(&self, transform: &Transform, point: [f32; 2]) -> bool;
}

/// A circle collision shape
#[derive(Debug, Clone, Copy)]
pub struct CircleShape {
    pub radius: f32,
}

impl CollisionShape for CircleShape {
    fn aabb(&self, transform: &Transform) -> (f32, f32, f32, f32) {
        let pos = transform.position;
        (
            pos[0] - self.radius,
            pos[1] - self.radius,
            pos[0] + self.radius,
            pos[1] + self.radius,
        )
    }

    fn closest_point(&self, transform: &Transform, point: [f32; 2]) -> [f32; 2] {
        let center = transform.position;
        let dx = point[0] - center[0];
        let dy = point[1] - center[1];
        let dist_sq = dx * dx + dy * dy;

        if dist_sq < 0.0001 {
            // Point is at the center, return any point on the circle
            return [center[0] + self.radius, center[1]];
        }

        let dist = dist_sq.sqrt();
        // Return the point on the circle's surface in the direction of the query point
        [
            center[0] + (dx / dist) * self.radius,
            center[1] + (dy / dist) * self.radius,
        ]
    }

    fn contains_point(&self, transform: &Transform, point: [f32; 2]) -> bool {
        let center = transform.position;
        let dx = point[0] - center[0];
        let dy = point[1] - center[1];
        let dist_sq = dx * dx + dy * dy;
        dist_sq <= self.radius * self.radius
    }
}

/// A rectangle collision shape
#[derive(Debug, Clone, Copy)]
pub struct RectangleShape {
    pub half_width: f32,
    pub half_height: f32,
}

impl RectangleShape {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            half_width: width / 2.0,
            half_height: height / 2.0,
        }
    }
}

impl CollisionShape for RectangleShape {
    fn aabb(&self, transform: &Transform) -> (f32, f32, f32, f32) {
        let pos = transform.position;
        (
            pos[0] - self.half_width,
            pos[1] - self.half_height,
            pos[0] + self.half_width,
            pos[1] + self.half_height,
        )
    }

    fn closest_point(&self, transform: &Transform, point: [f32; 2]) -> [f32; 2] {
        let center = transform.position;

        // Find the closest point on or inside the rectangle
        let local_x = point[0] - center[0];
        let local_y = point[1] - center[1];

        let clamped_x = local_x.clamp(-self.half_width, self.half_width);
        let clamped_y = local_y.clamp(-self.half_height, self.half_height);

        [center[0] + clamped_x, center[1] + clamped_y]
    }

    fn contains_point(&self, transform: &Transform, point: [f32; 2]) -> bool {
        let center = transform.position;
        let dx = (point[0] - center[0]).abs();
        let dy = (point[1] - center[1]).abs();
        dx <= self.half_width && dy <= self.half_height
    }
}

/// Represents the result of a collision test
#[derive(Debug, Clone, Copy)]
pub struct CollisionManifold {
    pub normal: [f32; 2],        // Points from object A toward object B
    pub penetration: f32,        // How deep the objects overlap
    pub contact_point: [f32; 2], // Where the collision occurred (world space)
}

/// Detect collision between any two shapes using the closest-point method
/// This works for any combination of shapes that implement CollisionShape
pub fn detect_collision(
    shape_a: &dyn CollisionShape,
    transform_a: &Transform,
    shape_b: &dyn CollisionShape,
    transform_b: &Transform,
) -> Option<CollisionManifold> {
    // Broad phase: check AABB overlap first
    let aabb_a = shape_a.aabb(transform_a);
    let aabb_b = shape_b.aabb(transform_b);

    // If bounding boxes don't overlap, no collision possible
    if aabb_a.2 < aabb_b.0 || aabb_a.0 > aabb_b.2 || aabb_a.3 < aabb_b.1 || aabb_a.1 > aabb_b.3 {
        return None;
    }

    // Narrow phase: use closest-point method
    // Find the closest point on B to A's center
    let center_a = shape_a.center(transform_a);
    let closest_on_b = shape_b.closest_point(transform_b, center_a);

    // Find the closest point on A to B's center
    let center_b = shape_b.center(transform_b);
    let closest_on_a = shape_a.closest_point(transform_a, center_b);

    // The separation vector points from the closest point on A to the closest point on B
    let dx = closest_on_b[0] - closest_on_a[0];
    let dy = closest_on_b[1] - closest_on_a[1];
    let dist_sq = dx * dx + dy * dy;

    // If the distance between closest points is very small, we have a collision
    // We need a small threshold because floating point math isn't exact
    if dist_sq < 0.0001 {
        // Shapes are touching or overlapping
        // Use the vector from center A to center B as the normal
        let normal_dx = center_b[0] - center_a[0];
        let normal_dy = center_b[1] - center_a[1];
        let normal_dist_sq = normal_dx * normal_dx + normal_dy * normal_dy;

        if normal_dist_sq > 0.0001 {
            let normal_dist = normal_dist_sq.sqrt();
            let normal = [normal_dx / normal_dist, normal_dy / normal_dist];

            // Estimate penetration depth
            // This is approximate but works well in practice
            let penetration = 0.02; // Small overlap detected

            Some(CollisionManifold {
                normal,
                penetration,
                contact_point: closest_on_a,
            })
        } else {
            // Centers are at the same position, use arbitrary normal
            Some(CollisionManifold {
                normal: [0.0, 1.0],
                penetration: 0.02,
                contact_point: closest_on_a,
            })
        }
    } else {
        let dist = dist_sq.sqrt();

        // Check if shapes are actually overlapping
        // This happens when the "closest points" are actually inside the other shape
        if shape_a.contains_point(transform_a, closest_on_b)
            || shape_b.contains_point(transform_b, closest_on_a)
        {
            // We have penetration
            let normal = [dx / dist, dy / dist];

            // The penetration depth is how far we need to separate the shapes
            // For overlapping shapes, we estimate based on center distance
            let center_dist =
                ((center_b[0] - center_a[0]).powi(2) + (center_b[1] - center_a[1]).powi(2)).sqrt();

            // Get approximate "radius" of each shape (distance from center to closest point)
            let radius_a = ((closest_on_a[0] - center_a[0]).powi(2)
                + (closest_on_a[1] - center_a[1]).powi(2))
            .sqrt();
            let radius_b = ((closest_on_b[0] - center_b[0]).powi(2)
                + (closest_on_b[1] - center_b[1]).powi(2))
            .sqrt();

            let penetration = (radius_a + radius_b - center_dist).max(0.01);

            Some(CollisionManifold {
                normal,
                penetration,
                contact_point: [
                    (closest_on_a[0] + closest_on_b[0]) / 2.0,
                    (closest_on_a[1] + closest_on_b[1]) / 2.0,
                ],
            })
        } else {
            // Shapes are separated, no collision
            None
        }
    }
}
