// src/shape.rs
use crate::components::Transform;

#[derive(Clone, Copy, Debug)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0)
    }

    pub fn dot(&self, other: &Vec2) -> f32 {
        self.x * other.x + self.y * other.y
    }

    pub fn length_sq(&self) -> f32 {
        self.dot(self)
    }

    pub fn length(&self) -> f32 {
        self.length_sq().sqrt()
    }

    pub fn normalized(&self) -> Vec2 {
        let len = self.length();
        if len > 1e-6 {
            Vec2::new(self.x / len, self.y / len)
        } else {
            Vec2::new(0.0, 0.0)
        }
    }

    pub fn perp(&self) -> Vec2 {
        Vec2::new(-self.y, self.x)
    }

    pub fn cross(&self, other: &Vec2) -> f32 {
        self.x * other.y - self.y * other.x
    }
}

impl std::ops::Add for Vec2 {
    type Output = Vec2;
    fn add(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x + other.x, self.y + other.y)
    }
}

impl std::ops::Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x - other.x, self.y - other.y)
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, scalar: f32) -> Vec2 {
        Vec2::new(self.x * scalar, self.y * scalar)
    }
}

impl std::ops::Neg for Vec2 {
    type Output = Vec2;
    fn neg(self) -> Vec2 {
        Vec2::new(-self.x, -self.y)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AABB {
    pub min: Vec2,
    pub max: Vec2,
}

impl AABB {
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    pub fn overlaps(&self, other: &AABB) -> bool {
        self.max.x >= other.min.x
            && self.min.x <= other.max.x
            && self.max.y >= other.min.y
            && self.min.y <= other.max.y
    }

    pub fn expand(&self, radius: f32) -> AABB {
        AABB::new(
            Vec2::new(self.min.x - radius, self.min.y - radius),
            Vec2::new(self.max.x + radius, self.max.y + radius),
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MassProperties {
    pub mass: f32,
    pub inertia: f32,
    pub center_of_mass: Vec2,
}

/// Core shape trait - all collision shapes must implement this
pub trait Shape: std::fmt::Debug {
    /// Support function: returns the point on the shape furthest in the given direction
    fn support(&self, transform: &Transform, direction: Vec2) -> Vec2;

    /// Compute axis-aligned bounding box in world space
    fn compute_aabb(&self, transform: &Transform) -> AABB;

    /// Compute mass properties for a given density
    fn compute_mass_properties(&self, density: f32) -> MassProperties;

    /// Get vertices for rendering (if applicable)
    fn vertices(&self) -> Option<Vec<Vec2>> {
        None
    }

    /// Get radius for circle rendering
    fn radius(&self) -> Option<f32> {
        None
    }
}

/// Circle collision shape
#[derive(Debug, Clone, Copy)]
pub struct Circle {
    pub radius: f32,
}

impl Circle {
    pub fn new(radius: f32) -> Self {
        Self { radius }
    }
}

impl Shape for Circle {
    fn support(&self, transform: &Transform, direction: Vec2) -> Vec2 {
        let dir_normalized = direction.normalized();
        let center = Vec2::new(transform.position[0], transform.position[1]);
        center + dir_normalized * self.radius
    }

    fn compute_aabb(&self, transform: &Transform) -> AABB {
        let center = Vec2::new(transform.position[0], transform.position[1]);
        AABB::new(
            Vec2::new(center.x - self.radius, center.y - self.radius),
            Vec2::new(center.x + self.radius, center.y + self.radius),
        )
    }

    fn compute_mass_properties(&self, density: f32) -> MassProperties {
        let area = std::f32::consts::PI * self.radius * self.radius;
        let mass = area * density;
        let inertia = 0.5 * mass * self.radius * self.radius;
        MassProperties {
            mass,
            inertia,
            center_of_mass: Vec2::zero(),
        }
    }

    fn radius(&self) -> Option<f32> {
        Some(self.radius)
    }
}

/// Convex polygon collision shape
#[derive(Debug, Clone)]
pub struct ConvexPolygon {
    pub vertices: Vec<Vec2>, // Local space, counter-clockwise
}

impl ConvexPolygon {
    pub fn new(vertices: Vec<Vec2>) -> Self {
        assert!(vertices.len() >= 3, "Polygon must have at least 3 vertices");
        Self { vertices }
    }

    /// Create a box centered at origin
    pub fn box_shape(half_width: f32, half_height: f32) -> Self {
        Self::new(vec![
            Vec2::new(-half_width, -half_height),
            Vec2::new(half_width, -half_height),
            Vec2::new(half_width, half_height),
            Vec2::new(-half_width, half_height),
        ])
    }

    /// Create a regular polygon
    pub fn regular(radius: f32, sides: usize) -> Self {
        assert!(sides >= 3, "Polygon must have at least 3 sides");
        let mut vertices = Vec::new();
        for i in 0..sides {
            let angle = (i as f32) * 2.0 * std::f32::consts::PI / (sides as f32);
            vertices.push(Vec2::new(radius * angle.cos(), radius * angle.sin()));
        }
        Self::new(vertices)
    }

    fn transform_point(&self, transform: &Transform, local_point: Vec2) -> Vec2 {
        let cos = transform.rotation.cos();
        let sin = transform.rotation.sin();
        let rotated = Vec2::new(
            local_point.x * cos - local_point.y * sin,
            local_point.x * sin + local_point.y * cos,
        );
        Vec2::new(
            transform.position[0] + rotated.x * transform.scale[0],
            transform.position[1] + rotated.y * transform.scale[1],
        )
    }
}

impl Shape for ConvexPolygon {
    fn support(&self, transform: &Transform, direction: Vec2) -> Vec2 {
        // Transform direction to local space
        let cos = transform.rotation.cos();
        let sin = transform.rotation.sin();
        let local_dir = Vec2::new(
            direction.x * cos + direction.y * sin,
            -direction.x * sin + direction.y * cos,
        );

        // Find vertex with maximum dot product in local space
        let mut max_dot = f32::NEG_INFINITY;
        let mut best_vertex = self.vertices[0];

        for vertex in &self.vertices {
            let dot = vertex.dot(&local_dir);
            if dot > max_dot {
                max_dot = dot;
                best_vertex = *vertex;
            }
        }

        self.transform_point(transform, best_vertex)
    }

    fn compute_aabb(&self, transform: &Transform) -> AABB {
        let first = self.transform_point(transform, self.vertices[0]);
        let mut min = first;
        let mut max = first;

        for vertex in &self.vertices[1..] {
            let world_vertex = self.transform_point(transform, *vertex);
            min.x = min.x.min(world_vertex.x);
            min.y = min.y.min(world_vertex.y);
            max.x = max.x.max(world_vertex.x);
            max.y = max.y.max(world_vertex.y);
        }

        AABB::new(min, max)
    }

    fn compute_mass_properties(&self, density: f32) -> MassProperties {
        // Use triangulation from origin for area and inertia
        let mut area = 0.0;
        let mut centroid = Vec2::zero();
        let mut inertia = 0.0;

        for i in 0..self.vertices.len() {
            let v1 = self.vertices[i];
            let v2 = self.vertices[(i + 1) % self.vertices.len()];

            let tri_area = 0.5 * v1.cross(&v2).abs();
            let tri_centroid = (v1 + v2) * (1.0 / 3.0);

            area += tri_area;
            centroid = centroid + tri_centroid * tri_area;

            // Inertia of triangle about origin
            let d1_sq = v1.length_sq();
            let d2_sq = v2.length_sq();
            let d12 = v1.dot(&v2);
            inertia += tri_area * (d1_sq + d2_sq + d12) / 6.0;
        }

        if area > 1e-6 {
            centroid = centroid * (1.0 / area);
        }

        let mass = area * density;
        let inertia = inertia * density;

        MassProperties {
            mass,
            inertia,
            center_of_mass: centroid,
        }
    }

    fn vertices(&self) -> Option<Vec<Vec2>> {
        Some(self.vertices.clone())
    }
}

/// Compound shape: multiple convex shapes with local transforms
#[derive(Debug, Clone)]
pub struct CompoundShape {
    pub children: Vec<(Box<dyn ShapeClone>, Transform)>,
}

impl CompoundShape {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    pub fn add_child(&mut self, shape: Box<dyn ShapeClone>, local_transform: Transform) {
        self.children.push((shape, local_transform));
    }

    pub fn compute_combined_mass_properties(&self, density: f32) -> MassProperties {
        let mut total_mass = 0.0;
        let mut weighted_com = Vec2::zero();
        let mut total_inertia = 0.0;

        for (shape, local_tf) in &self.children {
            let props = shape.compute_mass_properties(density);
            total_mass += props.mass;

            let world_com = Vec2::new(
                local_tf.position[0] + props.center_of_mass.x,
                local_tf.position[1] + props.center_of_mass.y,
            );
            weighted_com = weighted_com + world_com * props.mass;

            // Parallel axis theorem: I = I_local + m * d²
            let d_sq = world_com.length_sq();
            total_inertia += props.inertia + props.mass * d_sq;
        }

        let center_of_mass = if total_mass > 1e-6 {
            weighted_com * (1.0 / total_mass)
        } else {
            Vec2::zero()
        };

        MassProperties {
            mass: total_mass,
            inertia: total_inertia,
            center_of_mass,
        }
    }

    /// Combine parent and child transform
    pub fn combine_transform(parent: &Transform, child: &Transform) -> Transform {
        let cos = parent.rotation.cos();
        let sin = parent.rotation.sin();

        let local_pos = Vec2::new(child.position[0], child.position[1]);
        let rotated = Vec2::new(
            local_pos.x * cos - local_pos.y * sin,
            local_pos.x * sin + local_pos.y * cos,
        );

        Transform {
            position: [
                parent.position[0] + rotated.x * parent.scale[0],
                parent.position[1] + rotated.y * parent.scale[1],
            ],
            rotation: parent.rotation + child.rotation,
            scale: [
                parent.scale[0] * child.scale[0],
                parent.scale[1] * child.scale[1],
            ],
        }
    }
}

// Trait for cloning boxed shapes
pub trait ShapeClone: Shape {
    fn clone_box(&self) -> Box<dyn ShapeClone>;
}

impl<T> ShapeClone for T
where
    T: 'static + Shape + Clone,
{
    fn clone_box(&self) -> Box<dyn ShapeClone> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn ShapeClone> {
    fn clone(&self) -> Box<dyn ShapeClone> {
        self.clone_box()
    }
}
