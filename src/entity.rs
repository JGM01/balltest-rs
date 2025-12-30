// src/entity.rs
use crate::collision_system::Collider;
use crate::components::{Appearance, Clickable, RigidBody, Transform};

#[derive(Clone)]
pub struct Entity {
    pub transform: Transform,
    pub appearance: Appearance,
    pub rigid_body: Option<RigidBody>,
    pub collider: Option<Collider>,
    pub clickable: Option<Clickable>,
}

impl Entity {
    pub fn new(transform: Transform, appearance: Appearance) -> Self {
        Self {
            transform,
            appearance,
            rigid_body: None,
            collider: None,
            clickable: None,
        }
    }

    pub fn with_rigid_body(mut self, rigid_body: RigidBody) -> Self {
        self.rigid_body = Some(rigid_body);
        self
    }

    pub fn with_collider(mut self, collider: Collider) -> Self {
        self.collider = Some(collider);
        self
    }

    pub fn with_clickable(mut self, clickable: Clickable) -> Self {
        self.clickable = Some(clickable);
        self
    }
}
