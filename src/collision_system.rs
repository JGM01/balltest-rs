// src/collision_system.rs
use crate::components::Transform;
use crate::gjk::{circle_circle_collision, gjk_epa};
use crate::shape::{AABB, Circle, CompoundShape, Shape, ShapeClone, Vec2};

/// Collision manifold - output of narrow phase
#[derive(Debug, Clone)]
pub struct CollisionManifold {
    pub entity_a: usize,
    pub entity_b: usize,
    pub normal: Vec2, // Points from A to B
    pub penetration: f32,
    pub contact_point: Vec2,
}

/// Shape variant for storage
#[derive(Clone)]
pub enum ShapeVariant {
    Simple(Box<dyn ShapeClone>),
    Compound(CompoundShape),
}

impl ShapeVariant {
    pub fn compute_aabb(&self, transform: &Transform) -> AABB {
        match self {
            ShapeVariant::Simple(shape) => shape.compute_aabb(transform),
            ShapeVariant::Compound(compound) => {
                let mut aabb: Option<AABB> = None;

                for (child_shape, local_tf) in &compound.children {
                    let world_tf = CompoundShape::combine_transform(transform, local_tf);
                    let child_aabb = child_shape.compute_aabb(&world_tf);

                    aabb = Some(match aabb {
                        None => child_aabb,
                        Some(existing) => AABB::new(
                            Vec2::new(
                                existing.min.x.min(child_aabb.min.x),
                                existing.min.y.min(child_aabb.min.y),
                            ),
                            Vec2::new(
                                existing.max.x.max(child_aabb.max.x),
                                existing.max.y.max(child_aabb.max.y),
                            ),
                        ),
                    });
                }

                aabb.unwrap_or_else(|| AABB::new(Vec2::zero(), Vec2::zero()))
            }
        }
    }
}

/// Collider component
#[derive(Clone)]
pub struct Collider {
    pub shape: ShapeVariant,
    pub is_trigger: bool, // For sensors/triggers (no physics response)
}

impl Collider {
    pub fn from_shape(shape: Box<dyn ShapeClone>) -> Self {
        Self {
            shape: ShapeVariant::Simple(shape),
            is_trigger: false,
        }
    }

    pub fn from_compound(compound: CompoundShape) -> Self {
        Self {
            shape: ShapeVariant::Compound(compound),
            is_trigger: false,
        }
    }

    pub fn circle(radius: f32) -> Self {
        Self::from_shape(Box::new(Circle::new(radius)))
    }
}

/// Broad phase collision pair
#[derive(Clone, Copy)]
pub struct CollisionPair {
    entity_a: usize,
    entity_b: usize,
}

/// Collision detection system
pub struct CollisionSystem {
    pub manifolds: Vec<CollisionManifold>,
}

impl CollisionSystem {
    pub fn new() -> Self {
        Self {
            manifolds: Vec::new(),
        }
    }

    /// Broad phase: AABB sweep to find potential collision pairs
    pub fn broad_phase(&self, colliders: &[(usize, &Collider, &Transform)]) -> Vec<CollisionPair> {
        let mut pairs = Vec::new();

        // Simple O(n²) broad phase - could be optimized with spatial grid
        for i in 0..colliders.len() {
            let (idx_a, collider_a, tf_a) = &colliders[i];
            let aabb_a = collider_a.shape.compute_aabb(tf_a);

            for j in (i + 1)..colliders.len() {
                let (idx_b, collider_b, tf_b) = &colliders[j];
                let aabb_b = collider_b.shape.compute_aabb(tf_b);

                if aabb_a.overlaps(&aabb_b) {
                    pairs.push(CollisionPair {
                        entity_a: *idx_a,
                        entity_b: *idx_b,
                    });
                }
            }
        }

        pairs
    }

    /// Narrow phase: precise collision detection using GJK/EPA
    pub fn narrow_phase(
        &mut self,
        pairs: Vec<CollisionPair>,
        colliders: &[(usize, &Collider, &Transform)],
    ) {
        self.manifolds.clear();

        for pair in pairs {
            // Find collider data
            let (collider_a, tf_a) = colliders
                .iter()
                .find(|(idx, _, _)| *idx == pair.entity_a)
                .map(|(_, c, t)| (*c, *t))
                .unwrap();

            let (collider_b, tf_b) = colliders
                .iter()
                .find(|(idx, _, _)| *idx == pair.entity_b)
                .map(|(_, c, t)| (*c, *t))
                .unwrap();

            // Handle different shape combinations
            match (&collider_a.shape, &collider_b.shape) {
                (ShapeVariant::Simple(shape_a), ShapeVariant::Simple(shape_b)) => {
                    self.detect_simple_collision(
                        pair,
                        shape_a.as_ref(),
                        tf_a,
                        shape_b.as_ref(),
                        tf_b,
                    );
                }
                (ShapeVariant::Compound(compound), ShapeVariant::Simple(shape)) => {
                    self.detect_compound_simple(pair, compound, tf_a, shape.as_ref(), tf_b, false);
                }
                (ShapeVariant::Simple(shape), ShapeVariant::Compound(compound)) => {
                    self.detect_compound_simple(pair, compound, tf_b, shape.as_ref(), tf_a, true);
                }
                (ShapeVariant::Compound(compound_a), ShapeVariant::Compound(compound_b)) => {
                    self.detect_compound_compound(pair, compound_a, tf_a, compound_b, tf_b);
                }
            }
        }
    }

    fn detect_simple_collision(
        &mut self,
        pair: CollisionPair,
        shape_a: &dyn Shape,
        tf_a: &Transform,
        shape_b: &dyn Shape,
        tf_b: &Transform,
    ) {
        // Optimized path for circle-circle
        if let (Some(r_a), Some(r_b)) = (shape_a.radius(), shape_b.radius()) {
            let pos_a = Vec2::new(tf_a.position[0], tf_a.position[1]);
            let pos_b = Vec2::new(tf_b.position[0], tf_b.position[1]);

            if let Some((penetration, normal)) = circle_circle_collision(r_a, pos_a, r_b, pos_b) {
                self.manifolds.push(CollisionManifold {
                    entity_a: pair.entity_a,
                    entity_b: pair.entity_b,
                    normal,
                    penetration,
                    contact_point: pos_a + normal * (r_a - penetration * 0.5),
                });
            }
            return;
        }

        // Generic GJK/EPA path
        if let Some((penetration, normal)) = gjk_epa(shape_a, tf_a, shape_b, tf_b) {
            let pos_a = Vec2::new(tf_a.position[0], tf_a.position[1]);
            let contact = pos_a + normal * (penetration * 0.5);

            self.manifolds.push(CollisionManifold {
                entity_a: pair.entity_a,
                entity_b: pair.entity_b,
                normal,
                penetration,
                contact_point: contact,
            });
        }
    }

    fn detect_compound_simple(
        &mut self,
        pair: CollisionPair,
        compound: &CompoundShape,
        tf_compound: &Transform,
        simple_shape: &dyn Shape,
        tf_simple: &Transform,
        flip: bool,
    ) {
        for (child_shape, local_tf) in &compound.children {
            let world_tf = CompoundShape::combine_transform(tf_compound, local_tf);

            if flip {
                self.detect_simple_collision(
                    CollisionPair {
                        entity_a: pair.entity_b,
                        entity_b: pair.entity_a,
                    },
                    simple_shape,
                    tf_simple,
                    child_shape.as_ref(),
                    &world_tf,
                );
            } else {
                self.detect_simple_collision(
                    pair,
                    child_shape.as_ref(),
                    &world_tf,
                    simple_shape,
                    tf_simple,
                );
            }
        }
    }

    fn detect_compound_compound(
        &mut self,
        pair: CollisionPair,
        compound_a: &CompoundShape,
        tf_a: &Transform,
        compound_b: &CompoundShape,
        tf_b: &Transform,
    ) {
        for (child_a, local_a) in &compound_a.children {
            let world_a = CompoundShape::combine_transform(tf_a, local_a);

            for (child_b, local_b) in &compound_b.children {
                let world_b = CompoundShape::combine_transform(tf_b, local_b);

                self.detect_simple_collision(
                    pair,
                    child_a.as_ref(),
                    &world_a,
                    child_b.as_ref(),
                    &world_b,
                );
            }
        }
    }
}
