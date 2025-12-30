// src/components.rs
use crate::collision_system::Collider;
use crate::shape::Vec2;

#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub position: [f32; 2],
    pub rotation: f32,
    pub scale: [f32; 2],
}

impl Transform {
    pub fn new(position: [f32; 2]) -> Self {
        Self {
            position,
            rotation: 0.0,
            scale: [1.0, 1.0],
        }
    }

    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }
}

#[derive(Clone, Debug)]
pub struct RigidBody {
    pub velocity: Vec2,
    pub angular_velocity: f32,
    pub force: Vec2,
    pub torque: f32,

    pub mass: f32,
    pub inv_mass: f32,
    pub inertia: f32,
    pub inv_inertia: f32,

    pub is_static: bool,
    pub restitution: f32,
    pub friction: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
}

impl RigidBody {
    pub fn new(mass: f32, inertia: f32) -> Self {
        let inv_mass = if mass > 0.0 { 1.0 / mass } else { 0.0 };
        let inv_inertia = if inertia > 0.0 { 1.0 / inertia } else { 0.0 };

        Self {
            velocity: Vec2::zero(),
            angular_velocity: 0.0,
            force: Vec2::zero(),
            torque: 0.0,
            mass,
            inv_mass,
            inertia,
            inv_inertia,
            is_static: false,
            restitution: 0.6,
            friction: 0.4,
            linear_damping: 0.98,
            angular_damping: 0.95,
        }
    }

    pub fn new_static() -> Self {
        Self {
            velocity: Vec2::zero(),
            angular_velocity: 0.0,
            force: Vec2::zero(),
            torque: 0.0,
            mass: f32::INFINITY,
            inv_mass: 0.0,
            inertia: f32::INFINITY,
            inv_inertia: 0.0,
            is_static: true,
            restitution: 0.5,
            friction: 0.6,
            linear_damping: 1.0,
            angular_damping: 1.0,
        }
    }

    pub fn with_velocity(mut self, velocity: Vec2) -> Self {
        self.velocity = velocity;
        self
    }

    pub fn apply_force(&mut self, force: Vec2) {
        self.force = self.force + force;
    }

    pub fn apply_impulse(&mut self, impulse: Vec2) {
        if !self.is_static {
            self.velocity = self.velocity + impulse * self.inv_mass;
        }
    }

    pub fn apply_angular_impulse(&mut self, impulse: f32) {
        if !self.is_static {
            self.angular_velocity += impulse * self.inv_inertia;
        }
    }
}

/// Visual appearance - decoupled from collision
#[derive(Clone, Debug)]
pub enum Appearance {
    Circle {
        radius: f32,
        color: [f32; 3],
    },
    Polygon {
        vertices: Vec<[f32; 2]>,
        color: [f32; 3],
    },
    Compound {
        parts: Vec<Appearance>,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct Clickable {
    pub enabled: bool,
    pub hovered: bool,
}

impl Clickable {
    pub fn new() -> Self {
        Self {
            enabled: true,
            hovered: false,
        }
    }
}
