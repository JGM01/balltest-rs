use crate::{components::Shape, entity::Entity, world::World};
use std::time::Duration;

pub struct PhysicsSystem {
    gravity: [f32; 2],
}

impl PhysicsSystem {
    pub fn new() -> Self {
        Self {
            gravity: [0.0, -0.1], // Downward gravity in NDC space
        }
    }

    /// Run one fixed timestep of physics simulation
    pub fn update(&mut self, world: &mut World, dt: Duration) {
        let dt_secs = dt.as_secs_f32();

        for entity in world.entities_mut() {
            if let Some((physics, transform)) = entity.physics_and_transform_mut() {
                // Apply gravity
                physics.acceleration[0] += self.gravity[0] * f32::from(physics.apply_gravity);
                physics.acceleration[1] += self.gravity[1] * f32::from(physics.apply_gravity);

                // Update velocity
                physics.velocity[0] += physics.acceleration[0] * dt_secs;
                physics.velocity[1] += physics.acceleration[1] * dt_secs;

                // Update position
                transform.position[0] += physics.velocity[0] * dt_secs;
                transform.position[1] += physics.velocity[1] * dt_secs;

                // Reset acceleration
                physics.acceleration = [0.0, 0.0];
            }
        }
    }
}
fn resolve_collision(
    entity_a: &mut Entity,
    entity_b: &mut Entity,
    normal: [f32; 2],
    penetration: f32,
) {
    let (phys_a, phys_b) = (
        entity_a.physics_mut().unwrap(),
        entity_b.physics_mut().unwrap(),
    );

    // Separate objects first (position correction)
    let total_inv_mass = 1.0 / phys_a.mass + 1.0 / phys_b.mass;
    let correction_a = penetration * (1.0 / phys_a.mass) / total_inv_mass;
    let correction_b = penetration * (1.0 / phys_b.mass) / total_inv_mass;

    // Move them apart proportional to inverse mass
    // (heavier objects move less)

    // Velocity correction (the bounce)
    let relative_velocity = [
        phys_a.velocity[0] - phys_b.velocity[0],
        phys_a.velocity[1] - phys_b.velocity[1],
    ];

    let vel_along_normal = todo!();

    // Don't resolve if velocities are separating
    if vel_along_normal > 0.0 {
        return;
    }

    // Combine restitution (this is where your Option C matters!)
    let restitution = (phys_a.restitution + phys_b.restitution) / 2.0; // Or geometric mean?

    let impulse_scalar = -(1.0 + restitution) * vel_along_normal / total_inv_mass;

    let impulse = [impulse_scalar * normal[0], impulse_scalar * normal[1]];

    // Apply impulse (force over time) to velocities
    if phys_a.dynamic {
        phys_a.velocity[0] += impulse[0] / phys_a.mass;
        phys_a.velocity[1] += impulse[1] / phys_a.mass;
    }

    if phys_b.dynamic {
        phys_b.velocity[0] -= impulse[0] / phys_b.mass;
        phys_b.velocity[1] -= impulse[1] / phys_b.mass;
    }
}
