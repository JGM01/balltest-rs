use crate::collision::detect_collision;
use crate::world::World;
use std::time::Duration;

pub struct PhysicsSystem {
    gravity: [f32; 2],
    collision_iterations: u32,
}

impl PhysicsSystem {
    pub fn new() -> Self {
        Self {
            gravity: [0.0, -0.5],
            collision_iterations: 4,
        }
    }

    pub fn update(&mut self, world: &mut World, dt: Duration) {
        let dt_secs = dt.as_secs_f32();

        // Phase 1: Apply forces and integrate velocity
        for entity in world.entities_mut() {
            if let Some(physics) = &mut entity.physics {
                if !physics.dynamic {
                    continue;
                }
                // Reset acceleration
                physics.acceleration = [0.0, 0.0];

                // Apply gravity
                if physics.apply_gravity {
                    physics.acceleration[0] += self.gravity[0];
                    physics.acceleration[1] += self.gravity[1];
                }

                // Integrate velocity
                physics.velocity[0] += physics.acceleration[0] * dt_secs;
                physics.velocity[1] += physics.acceleration[1] * dt_secs;

                // Apply damping
                physics.velocity[0] *= 0.98;
                physics.velocity[1] *= 0.98;

                // Sleep threshold
                let speed_sq = physics.velocity[0].powi(2) + physics.velocity[1].powi(2);
                if speed_sq < 1e-6 {
                    physics.velocity = [0.0, 0.0];
                }
            }
        }

        // Phase 2: Integrate position
        for entity in world.entities_mut() {
            if let Some(physics) = &entity.physics {
                if physics.dynamic {
                    entity.transform.position[0] += physics.velocity[0] * dt_secs;
                    entity.transform.position[1] += physics.velocity[1] * dt_secs;
                }
            }
        }

        // Phase 3: Collision detection and resolution
        for _ in 0..self.collision_iterations {
            self.resolve_collisions(world);
        }
    }

    fn resolve_collisions(&mut self, world: &mut World) {
        let entity_count = world.entities().len();

        for i in 0..entity_count {
            for j in (i + 1)..entity_count {
                // Check if both entities have colliders
                let (has_collider_i, has_collider_j) = {
                    let entities = world.entities();
                    (
                        entities[i].collider.is_some(),
                        entities[j].collider.is_some(),
                    )
                };

                if !has_collider_i || !has_collider_j {
                    continue;
                }

                // Detect collision using our unified system
                let manifold = {
                    let entities = world.entities();
                    let collider_a = entities[i].collider.as_ref().unwrap();
                    let collider_b = entities[j].collider.as_ref().unwrap();

                    detect_collision(
                        collider_a.shape(),
                        &entities[i].transform,
                        collider_b.shape(),
                        &entities[j].transform,
                    )
                };

                if let Some(manifold) = manifold {
                    self.resolve_collision(world, i, j, manifold);
                }
            }
        }
    }

    fn resolve_collision(
        &mut self,
        world: &mut World,
        idx_a: usize,
        idx_b: usize,
        manifold: crate::collision::CollisionManifold,
    ) {
        // Get physics properties
        let (mass_a, mass_b, dynamic_a, dynamic_b, rest_a, rest_b, fric_a, fric_b) = {
            let entities = world.entities();
            let phys_a = entities[idx_a].physics.as_ref();
            let phys_b = entities[idx_b].physics.as_ref();

            (
                phys_a.map(|p| p.mass).unwrap_or(f32::INFINITY),
                phys_b.map(|p| p.mass).unwrap_or(f32::INFINITY),
                phys_a.map(|p| p.dynamic).unwrap_or(false),
                phys_b.map(|p| p.dynamic).unwrap_or(false),
                phys_a.map(|p| p.restitution).unwrap_or(0.5),
                phys_b.map(|p| p.restitution).unwrap_or(0.5),
                phys_a.map(|p| p.friction).unwrap_or(0.3),
                phys_b.map(|p| p.friction).unwrap_or(0.3),
            )
        };

        if !dynamic_a && !dynamic_b {
            return;
        }

        // Calculate inverse masses
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

        // Position correction
        if total_inv_mass > 0.0 {
            let correction_percent = 0.8;
            let slop = 0.01;
            let correction_depth = (manifold.penetration - slop).max(0.0);

            let entities = world.entities_mut();

            if dynamic_a && inv_mass_a > 0.0 {
                let ratio = inv_mass_a / total_inv_mass;
                let correction = correction_depth * ratio * correction_percent;
                entities[idx_a].transform.position[0] -= manifold.normal[0] * correction;
                entities[idx_a].transform.position[1] -= manifold.normal[1] * correction;
            }

            if dynamic_b && inv_mass_b > 0.0 {
                let ratio = inv_mass_b / total_inv_mass;
                let correction = correction_depth * ratio * correction_percent;
                entities[idx_b].transform.position[0] += manifold.normal[0] * correction;
                entities[idx_b].transform.position[1] += manifold.normal[1] * correction;
            }
        }

        // Velocity resolution
        let (vel_a, vel_b) = {
            let entities = world.entities();
            (
                entities[idx_a]
                    .physics
                    .as_ref()
                    .map(|p| p.velocity)
                    .unwrap_or([0.0, 0.0]),
                entities[idx_b]
                    .physics
                    .as_ref()
                    .map(|p| p.velocity)
                    .unwrap_or([0.0, 0.0]),
            )
        };

        let rel_vel = [vel_a[0] - vel_b[0], vel_a[1] - vel_b[1]];
        let vel_along_normal = rel_vel[0] * manifold.normal[0] + rel_vel[1] * manifold.normal[1];

        // Don't resolve if separating
        if vel_along_normal > 0.0 {
            return;
        }

        // Use zero restitution for resting contacts
        let restitution = if vel_along_normal.abs() < 0.1 {
            0.0
        } else {
            (rest_a * rest_b).sqrt()
        };

        // Calculate impulse
        let j = -(1.0 + restitution) * vel_along_normal / total_inv_mass;
        let impulse_n = [manifold.normal[0] * j, manifold.normal[1] * j];

        // Friction
        let tangent = [-manifold.normal[1], manifold.normal[0]];
        let vel_along_tangent = rel_vel[0] * tangent[0] + rel_vel[1] * tangent[1];
        let friction = (fric_a + fric_b) * 0.5;
        let friction_mag =
            (-vel_along_tangent / total_inv_mass).clamp(-j.abs() * friction, j.abs() * friction);
        let impulse_t = [tangent[0] * friction_mag, tangent[1] * friction_mag];

        let total_impulse = [impulse_n[0] + impulse_t[0], impulse_n[1] + impulse_t[1]];

        // Apply impulses
        {
            let entities = world.entities_mut();

            if dynamic_a && inv_mass_a > 0.0 {
                if let Some(physics) = &mut entities[idx_a].physics {
                    physics.velocity[0] -= total_impulse[0] * inv_mass_a;
                    physics.velocity[1] -= total_impulse[1] * inv_mass_a;
                }
            }

            if dynamic_b && inv_mass_b > 0.0 {
                if let Some(physics) = &mut entities[idx_b].physics {
                    physics.velocity[0] += total_impulse[0] * inv_mass_b;
                    physics.velocity[1] += total_impulse[1] * inv_mass_b;
                }
            }
        }
    }
}
