use crate::collision::{CircleShape, CollisionShape, RectangleShape};

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
}

#[derive(Clone, Copy, Debug)]
pub struct Physics {
    pub velocity: [f32; 2],
    pub acceleration: [f32; 2],
    pub mass: f32,
    pub apply_gravity: bool,
    pub dynamic: bool,
    pub restitution: f32,
    pub friction: f32,
}

impl Physics {
    pub fn new() -> Self {
        Self {
            velocity: [0.0, 0.0],
            acceleration: [0.0, 0.0],
            mass: 1.0,
            apply_gravity: true,
            dynamic: true,
            restitution: 0.8,
            friction: 0.5,
        }
    }

    pub fn new_static() -> Self {
        Self {
            mass: f32::INFINITY,
            dynamic: false,
            apply_gravity: false,
            restitution: 0.5,
            ..Self::new()
        }
    }

    pub fn with_velocity(mut self, velocity: [f32; 2]) -> Self {
        self.velocity = velocity;
        self
    }
}

/// Visual appearance of an entity
/// This is separate from collision shape
#[derive(Clone, Debug)]
pub enum Appearance {
    Circle {
        radius: f32,
        color: [f32; 3],
    },
    Rectangle {
        width: f32,
        height: f32,
        color: [f32; 3],
    },
    Text {
        content: String,
        font_size: f32,
        color: [f32; 3],
    },
}

/// Collision geometry component
/// Uses trait objects to support any collision shape
#[derive(Clone, Debug)]
pub struct Collider {
    // We use Box<dyn> here to allow any type implementing CollisionShape
    // This is dynamic dispatch - there's a tiny performance cost but huge flexibility gain
    shape: ColliderShape,
}

// We need our own enum because trait objects can't be cloned directly
#[derive(Clone, Debug)]
enum ColliderShape {
    Circle(CircleShape),
    Rectangle(RectangleShape),
}

impl Collider {
    pub fn circle(radius: f32) -> Self {
        Self {
            shape: ColliderShape::Circle(CircleShape { radius }),
        }
    }

    pub fn rectangle(width: f32, height: f32) -> Self {
        Self {
            shape: ColliderShape::Rectangle(RectangleShape::new(width, height)),
        }
    }

    /// Get the collision shape for collision detection
    pub fn shape(&self) -> &dyn CollisionShape {
        match &self.shape {
            ColliderShape::Circle(s) => s,
            ColliderShape::Rectangle(s) => s,
        }
    }
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
