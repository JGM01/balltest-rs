// src/systems/physics.rs
use crate::collision_system::{CollisionManifold, CollisionSystem};
use crate::shape::Vec2;
use crate::world::World;
use std::time::Duration;

pub struct PhysicsSystem {
    gravity: Vec2,
    collision_system: CollisionSystem,
    position_iterations: usize,
    velocity_iterations: usize,
}

impl PhysicsSystem {
    pub fn new() -> Self {
        Self {
            gravity: Vec2::new(0.0, -9.8),
            collision_system: CollisionSystem::new(),
            position_iterations: 2,
            velocity_iterations: 6,
        }
    }

    pub fn update(&mut self, world: &mut World, dt: Duration) {
        let dt_secs = dt.as_secs_f32();

        // Phase 1: Apply forces and integrate velocities
        for entity in world.entities_mut() {
            if let Some(rb) = &mut entity.rigid_body {
                if rb.is_static {
                    continue;
                }

                // Apply gravity
                let gravity_force = self.gravity * rb.mass;
                rb.apply_force(gravity_force);

                // Integrate velocity
                let acceleration = rb.force * rb.inv_mass;
                rb.velocity = rb.velocity + acceleration * dt_secs;

                // Integrate angular velocity
                let angular_acc = rb.torque * rb.inv_inertia;
                rb.angular_velocity += angular_acc * dt_secs;

                // Apply damping
                rb.velocity = rb.velocity * rb.linear_damping;
                rb.angular_velocity *= rb.angular_damping;

                // Reset forces
                rb.force = Vec2::zero();
                rb.torque = 0.0;
            }
        }

        // Phase 2: Integrate positions (semi-implicit Euler)
        for entity in world.entities_mut() {
            if let Some(rb) = &entity.rigid_body {
                if rb.is_static {
                    continue;
                }

                entity.transform.position[0] += rb.velocity.x * dt_secs;
                entity.transform.position[1] += rb.velocity.y * dt_secs;
                entity.transform.rotation += rb.angular_velocity * dt_secs;
            }
        }

        // Phase 3: Collision detection
        let collider_data: Vec<_> = world
            .entities()
            .iter()
            .enumerate()
            .filter_map(|(idx, e)| e.collider.as_ref().map(|c| (idx, c, &e.transform)))
            .collect();

        let pairs = self.collision_system.broad_phase(&collider_data);
        self.collision_system.narrow_phase(pairs, &collider_data);

        // Phase 4: Solve collisions
        for _ in 0..self.velocity_iterations {
            for manifold in &self.collision_system.manifolds.clone() {
                self.solve_velocity(world, manifold);
            }
        }

        for _ in 0..self.position_iterations {
            for manifold in &self.collision_system.manifolds.clone() {
                self.solve_position(world, manifold);
            }
        }
    }

    fn solve_velocity(&self, world: &mut World, manifold: &CollisionManifold) {
        let entities = world.entities();

        let rb_a = entities[manifold.entity_a].rigid_body.as_ref();
        let rb_b = entities[manifold.entity_b].rigid_body.as_ref();

        if rb_a.is_none() || rb_b.is_none() {
            return;
        }

        let (inv_mass_a, inv_inertia_a, vel_a, ang_vel_a, rest_a, fric_a) = {
            let rb = rb_a.unwrap();
            (
                rb.inv_mass,
                rb.inv_inertia,
                rb.velocity,
                rb.angular_velocity,
                rb.restitution,
                rb.friction,
            )
        };

        let (inv_mass_b, inv_inertia_b, vel_b, ang_vel_b, rest_b, fric_b) = {
            let rb = rb_b.unwrap();
            (
                rb.inv_mass,
                rb.inv_inertia,
                rb.velocity,
                rb.angular_velocity,
                rb.restitution,
                rb.friction,
            )
        };

        if inv_mass_a == 0.0 && inv_mass_b == 0.0 {
            return;
        }

        let pos_a = Vec2::new(
            entities[manifold.entity_a].transform.position[0],
            entities[manifold.entity_a].transform.position[1],
        );
        let pos_b = Vec2::new(
            entities[manifold.entity_b].transform.position[0],
            entities[manifold.entity_b].transform.position[1],
        );

        // Contact points relative to centers
        let ra = manifold.contact_point - pos_a;
        let rb_vec = manifold.contact_point - pos_b;

        // Relative velocity at contact point
        let vel_a_at_contact = vel_a + Vec2::new(-ang_vel_a * ra.y, ang_vel_a * ra.x);
        let vel_b_at_contact = vel_b + Vec2::new(-ang_vel_b * rb_vec.y, ang_vel_b * rb_vec.x);
        let rel_vel = vel_b_at_contact - vel_a_at_contact;

        // Velocity along normal
        let vel_along_normal = rel_vel.dot(&manifold.normal);

        // Don't resolve if velocities are separating
        if vel_along_normal > 0.0 {
            return;
        }

        // Calculate restitution
        let restitution = (rest_a * rest_b).sqrt();

        // Calculate impulse scalar
        let ra_perp_dot = ra.perp().dot(&manifold.normal);
        let rb_perp_dot = rb_vec.perp().dot(&manifold.normal);

        let impulse_scalar = -(1.0 + restitution) * vel_along_normal
            / (inv_mass_a
                + inv_mass_b
                + ra_perp_dot * ra_perp_dot * inv_inertia_a
                + rb_perp_dot * rb_perp_dot * inv_inertia_b);

        let impulse = manifold.normal * impulse_scalar;

        // Apply normal impulse
        let entities = world.entities_mut();

        if let Some(rb) = entities[manifold.entity_a].rigid_body.as_mut() {
            rb.apply_impulse(-impulse);
            rb.apply_angular_impulse(-ra.cross(&impulse));
        }

        if let Some(rb) = entities[manifold.entity_b].rigid_body.as_mut() {
            rb.apply_impulse(impulse);
            rb.apply_angular_impulse(rb_vec.cross(&impulse));
        }

        // Friction
        let entities = world.entities();
        let rb_a = entities[manifold.entity_a].rigid_body.as_ref().unwrap();
        let rb_b = entities[manifold.entity_b].rigid_body.as_ref().unwrap();

        let vel_a_at_contact =
            rb_a.velocity + Vec2::new(-rb_a.angular_velocity * ra.y, rb_a.angular_velocity * ra.x);
        let vel_b_at_contact = rb_b.velocity
            + Vec2::new(
                -rb_b.angular_velocity * rb_vec.y,
                rb_b.angular_velocity * rb_vec.x,
            );
        let rel_vel = vel_b_at_contact - vel_a_at_contact;

        let tangent = (rel_vel - manifold.normal * rel_vel.dot(&manifold.normal)).normalized();
        let vel_along_tangent = rel_vel.dot(&tangent);

        if vel_along_tangent.abs() < 1e-6 {
            return;
        }

        let friction = (fric_a + fric_b) * 0.5;

        let friction_impulse_scalar = -vel_along_tangent
            / (inv_mass_a
                + inv_mass_b
                + ra.perp().dot(&tangent).powi(2) * inv_inertia_a
                + rb_vec.perp().dot(&tangent).powi(2) * inv_inertia_b);

        let max_friction = impulse_scalar.abs() * friction;
        let friction_impulse_scalar = friction_impulse_scalar.clamp(-max_friction, max_friction);
        let friction_impulse = tangent * friction_impulse_scalar;

        let entities = world.entities_mut();

        if let Some(rb) = entities[manifold.entity_a].rigid_body.as_mut() {
            rb.apply_impulse(-friction_impulse);
            rb.apply_angular_impulse(-ra.cross(&friction_impulse));
        }

        if let Some(rb) = entities[manifold.entity_b].rigid_body.as_mut() {
            rb.apply_impulse(friction_impulse);
            rb.apply_angular_impulse(rb_vec.cross(&friction_impulse));
        }
    }

    fn solve_position(&self, world: &mut World, manifold: &CollisionManifold) {
        let entities = world.entities();

        let rb_a = entities[manifold.entity_a].rigid_body.as_ref();
        let rb_b = entities[manifold.entity_b].rigid_body.as_ref();

        if rb_a.is_none() || rb_b.is_none() {
            return;
        }

        let inv_mass_a = rb_a.unwrap().inv_mass;
        let inv_mass_b = rb_b.unwrap().inv_mass;

        if inv_mass_a == 0.0 && inv_mass_b == 0.0 {
            return;
        }

        let slop = 0.01;
        let percent = 0.4;

        let correction =
            (manifold.penetration - slop).max(0.0) / (inv_mass_a + inv_mass_b) * percent;
        let correction_vec = manifold.normal * correction;

        let entities = world.entities_mut();

        if inv_mass_a > 0.0 {
            let entity = &mut entities[manifold.entity_a];
            entity.transform.position[0] -= correction_vec.x * inv_mass_a;
            entity.transform.position[1] -= correction_vec.y * inv_mass_a;
        }

        if inv_mass_b > 0.0 {
            let entity = &mut entities[manifold.entity_b];
            entity.transform.position[0] += correction_vec.x * inv_mass_b;
            entity.transform.position[1] += correction_vec.y * inv_mass_b;
        }
    }
}
