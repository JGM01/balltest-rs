use crate::{components::Shape, world::World};
use std::time::Duration;

pub struct PhysicsSystem {
    gravity: [f32; 2],
    collision_iterations: u32,
    // Velocity threshold for considering an object "at rest"
    sleep_velocity_threshold: f32,
    // How much energy is lost per second when sliding (contact friction)
    contact_friction_coefficient: f32,
    // Air resistance (always applied)
    air_damping: f32,
}

impl PhysicsSystem {
    pub fn new() -> Self {
        Self {
            gravity: [0.0, -0.5],
            collision_iterations: 4,
            sleep_velocity_threshold: 0.001,
            contact_friction_coefficient: 2.0, // NDC units/sec² of deceleration
            air_damping: 0.98,                 // Per-frame multiplier (1.0 = no damping)
        }
    }

    pub fn update(&mut self, world: &mut World, dt: Duration) {
        let dt_secs = dt.as_secs_f32();

        // === PHASE 1: Apply forces and integrate velocity ===
        for entity in world.entities_mut() {
            if let Some((physics, _)) = entity.physics_and_transform_mut() {
                if !physics.dynamic {
                    continue;
                }

                // Apply gravity
                if physics.apply_gravity {
                    physics.acceleration[0] += self.gravity[0];
                    physics.acceleration[1] += self.gravity[1];
                }

                // Update velocity from acceleration
                physics.velocity[0] += physics.acceleration[0] * dt_secs;
                physics.velocity[1] += physics.acceleration[1] * dt_secs;

                // Apply air damping (subtle air resistance)
                physics.velocity[0] *= self.air_damping;
                physics.velocity[1] *= self.air_damping;

                // Sleep very slow objects to prevent jitter
                let speed_sq = physics.velocity[0] * physics.velocity[0]
                    + physics.velocity[1] * physics.velocity[1];
                if speed_sq < self.sleep_velocity_threshold * self.sleep_velocity_threshold {
                    physics.velocity = [0.0, 0.0];
                }

                // Reset acceleration for next frame
                physics.acceleration = [0.0, 0.0];
            }
        }

        // === PHASE 2: Integrate position ===
        for entity in world.entities_mut() {
            if let Some((physics, transform)) = entity.physics_and_transform_mut() {
                if !physics.dynamic {
                    continue;
                }

                transform.position[0] += physics.velocity[0] * dt_secs;
                transform.position[1] += physics.velocity[1] * dt_secs;
            }
        }

        // === PHASE 3: Detect and resolve collisions ===
        for _ in 0..self.collision_iterations {
            self.resolve_collisions(world, dt_secs);
        }
    }

    fn resolve_collisions(&mut self, world: &mut World, dt_secs: f32) {
        let entity_count = world.entities().len();

        for i in 0..entity_count {
            for j in (i + 1)..entity_count {
                let collision_data = {
                    let entities = world.entities();
                    self.check_collision(&entities[i], &entities[j])
                };

                if let Some((normal, depth)) = collision_data {
                    self.resolve_collision_pair(world, i, j, normal, depth, dt_secs);
                }
            }
        }
    }

    fn check_collision(
        &self,
        entity_a: &crate::entity::Entity,
        entity_b: &crate::entity::Entity,
    ) -> Option<([f32; 2], f32)> {
        if entity_a.physics().is_none() && entity_b.physics().is_none() {
            return None;
        }

        let pos_a = entity_a.transform().position;
        let pos_b = entity_b.transform().position;

        match (entity_a.shape(), entity_b.shape()) {
            (Shape::Circle { radius: r_a, .. }, Shape::Circle { radius: r_b, .. }) => {
                self.check_circle_circle(pos_a, *r_a, pos_b, *r_b)
            }
            (Shape::Circle { radius, .. }, Shape::Rectangle { length, height, .. }) => {
                self.check_circle_rect(pos_a, *radius, pos_b, *length, *height)
            }
            (Shape::Rectangle { length, height, .. }, Shape::Circle { radius, .. }) => self
                .check_circle_rect(pos_b, *radius, pos_a, *length, *height)
                .map(|(n, d)| ([-n[0], -n[1]], d)),
            (
                Shape::Rectangle {
                    length: l_a,
                    height: h_a,
                    ..
                },
                Shape::Rectangle {
                    length: l_b,
                    height: h_b,
                    ..
                },
            ) => self.check_rect_rect(pos_a, *l_a, *h_a, pos_b, *l_b, *h_b),
            _ => None,
        }
    }

    fn check_circle_circle(
        &self,
        pos_a: [f32; 2],
        r_a: f32,
        pos_b: [f32; 2],
        r_b: f32,
    ) -> Option<([f32; 2], f32)> {
        let dx = pos_b[0] - pos_a[0];
        let dy = pos_b[1] - pos_a[1];
        let dist_sq = dx * dx + dy * dy;
        let min_dist = r_a + r_b;

        if dist_sq < min_dist * min_dist && dist_sq > 0.0001 {
            let dist = dist_sq.sqrt();
            let normal = [dx / dist, dy / dist];
            let depth = min_dist - dist;
            Some((normal, depth))
        } else {
            None
        }
    }

    fn check_circle_rect(
        &self,
        circle_pos: [f32; 2],
        radius: f32,
        rect_pos: [f32; 2],
        length: f32,
        height: f32,
    ) -> Option<([f32; 2], f32)> {
        let half_w = length / 2.0;
        let half_h = height / 2.0;

        // Find closest point on/in rectangle to circle center
        let closest_x = (circle_pos[0] - rect_pos[0]).clamp(-half_w, half_w) + rect_pos[0];
        let closest_y = (circle_pos[1] - rect_pos[1]).clamp(-half_h, half_h) + rect_pos[1];

        let dx = circle_pos[0] - closest_x;
        let dy = circle_pos[1] - closest_y;
        let dist_sq = dx * dx + dy * dy;

        if dist_sq < radius * radius {
            if dist_sq > 0.0001 {
                // Normal case: circle overlapping edge/corner
                let dist = dist_sq.sqrt();
                let normal = [dx / dist, dy / dist];
                let depth = radius - dist;
                Some((normal, depth))
            } else {
                // Circle center inside rectangle - push along shortest axis
                let dx_to_edge = half_w - (circle_pos[0] - rect_pos[0]).abs();
                let dy_to_edge = half_h - (circle_pos[1] - rect_pos[1]).abs();

                if dx_to_edge < dy_to_edge {
                    let sign = if circle_pos[0] > rect_pos[0] {
                        1.0
                    } else {
                        -1.0
                    };
                    Some(([sign, 0.0], radius + dx_to_edge))
                } else {
                    let sign = if circle_pos[1] > rect_pos[1] {
                        1.0
                    } else {
                        -1.0
                    };
                    Some(([0.0, sign], radius + dy_to_edge))
                }
            }
        } else {
            None
        }
    }

    fn check_rect_rect(
        &self,
        pos_a: [f32; 2],
        len_a: f32,
        height_a: f32,
        pos_b: [f32; 2],
        len_b: f32,
        height_b: f32,
    ) -> Option<([f32; 2], f32)> {
        let half_w_a = len_a / 2.0;
        let half_h_a = height_a / 2.0;
        let half_w_b = len_b / 2.0;
        let half_h_b = height_b / 2.0;

        // AABB overlap test
        let dx = pos_b[0] - pos_a[0];
        let dy = pos_b[1] - pos_a[1];

        let overlap_x = (half_w_a + half_w_b) - dx.abs();
        let overlap_y = (half_h_a + half_h_b) - dy.abs();

        if overlap_x > 0.0 && overlap_y > 0.0 {
            // Collision detected - return MTV (Minimum Translation Vector)
            if overlap_x < overlap_y {
                let normal = if dx > 0.0 { [1.0, 0.0] } else { [-1.0, 0.0] };
                Some((normal, overlap_x))
            } else {
                let normal = if dy > 0.0 { [0.0, 1.0] } else { [0.0, -1.0] };
                Some((normal, overlap_y))
            }
        } else {
            None
        }
    }

    fn resolve_collision_pair(
        &mut self,
        world: &mut World,
        idx_a: usize,
        idx_b: usize,
        normal: [f32; 2],
        depth: f32,
        dt_secs: f32,
    ) {
        // Gather immutable data first
        let (
            mass_a,
            mass_b,
            dynamic_a,
            dynamic_b,
            restitution_a,
            restitution_b,
            friction_a,
            friction_b,
        ) = {
            let entities = world.entities();
            let phys_a = entities[idx_a].physics();
            let phys_b = entities[idx_b].physics();

            let mass_a = phys_a.map(|p| p.mass).unwrap_or(f32::INFINITY);
            let mass_b = phys_b.map(|p| p.mass).unwrap_or(f32::INFINITY);
            let dynamic_a = phys_a.map(|p| p.dynamic).unwrap_or(false);
            let dynamic_b = phys_b.map(|p| p.dynamic).unwrap_or(false);
            let restitution_a = phys_a.map(|p| p.restitution).unwrap_or(0.5);
            let restitution_b = phys_b.map(|p| p.restitution).unwrap_or(0.5);
            let friction_a = phys_a.map(|p| p.friction).unwrap_or(0.3);
            let friction_b = phys_b.map(|p| p.friction).unwrap_or(0.3);

            (
                mass_a,
                mass_b,
                dynamic_a,
                dynamic_b,
                restitution_a,
                restitution_b,
                friction_a,
                friction_b,
            )
        };

        // Both static = no collision response
        if !dynamic_a && !dynamic_b {
            return;
        }

        // === POSITION CORRECTION ===
        let inv_mass_a = if dynamic_a && mass_a.is_finite() {
            1.0 / mass_a
        } else {
            0.0
        };
        let inv_mass_b = if dynamic_b && mass_b.is_finite() {
            1.0 / mass_b
        } else {
            0.0
        };
        let total_inv_mass = inv_mass_a + inv_mass_b;

        if total_inv_mass > 0.0 {
            let correction = [
                normal[0] * depth / total_inv_mass,
                normal[1] * depth / total_inv_mass,
            ];

            let entities = world.entities_mut();

            if dynamic_a && inv_mass_a > 0.0 {
                let transform = entities[idx_a].transform_mut();
                transform.position[0] -= correction[0] * inv_mass_a;
                transform.position[1] -= correction[1] * inv_mass_a;
            }

            if dynamic_b && inv_mass_b > 0.0 {
                let transform = entities[idx_b].transform_mut();
                transform.position[0] += correction[0] * inv_mass_b;
                transform.position[1] += correction[1] * inv_mass_b;
            }
        }

        // === VELOCITY RESOLUTION ===
        let (vel_a, vel_b) = {
            let entities = world.entities();
            let vel_a = entities[idx_a]
                .physics()
                .map(|p| p.velocity)
                .unwrap_or([0.0, 0.0]);
            let vel_b = entities[idx_b]
                .physics()
                .map(|p| p.velocity)
                .unwrap_or([0.0, 0.0]);
            (vel_a, vel_b)
        };

        let rel_vel = [vel_a[0] - vel_b[0], vel_a[1] - vel_b[1]];
        let vel_along_normal = rel_vel[0] * normal[0] + rel_vel[1] * normal[1];

        // Objects separating - no impulse needed
        if vel_along_normal > 0.0 {
            return;
        }

        // Combined restitution (how bouncy the collision is)
        let restitution = (restitution_a * restitution_b).sqrt(); // Geometric mean

        // Calculate impulse magnitude
        let j = -(1.0 + restitution) * vel_along_normal / total_inv_mass;
        let impulse_n = [normal[0] * j, normal[1] * j];

        // === FRICTION (tangential impulse) ===
        let tangent = [-normal[1], normal[0]]; // Perpendicular to normal
        let vel_along_tangent = rel_vel[0] * tangent[0] + rel_vel[1] * tangent[1];

        let friction = (friction_a + friction_b) * 0.5;

        // Coulomb friction: friction force can't exceed normal force
        let friction_impulse_mag =
            (-vel_along_tangent / total_inv_mass).clamp(-j.abs() * friction, j.abs() * friction);
        let impulse_t = [
            tangent[0] * friction_impulse_mag,
            tangent[1] * friction_impulse_mag,
        ];

        // Combined impulse
        let total_impulse = [impulse_n[0] + impulse_t[0], impulse_n[1] + impulse_t[1]];

        // Apply impulses
        {
            let entities = world.entities_mut();

            if dynamic_a && inv_mass_a > 0.0 {
                if let Some(physics) = entities[idx_a].physics_mut() {
                    physics.velocity[0] -= total_impulse[0] * inv_mass_a;
                    physics.velocity[1] -= total_impulse[1] * inv_mass_a;
                }
            }

            if dynamic_b && inv_mass_b > 0.0 {
                if let Some(physics) = entities[idx_b].physics_mut() {
                    physics.velocity[0] += total_impulse[0] * inv_mass_b;
                    physics.velocity[1] += total_impulse[1] * inv_mass_b;
                }
            }
        }
    }
}
