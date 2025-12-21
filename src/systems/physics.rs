use crate::{components::Shape, world::World};
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
