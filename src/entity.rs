use crate::components::{Clickable, Physics, Shape, Transform};

/// Entity is an enum of different types, each with their own component composition
#[derive(Clone, Debug)]
pub enum Entity {
    Circle {
        transform: Transform,
        physics: Option<Physics>,
        shape: Shape, // Must be Shape::Circle variant
        clickable: Option<Clickable>,
    },
    Text {
        transform: Transform,
        physics: Option<Physics>,
        shape: Shape, // Must be Shape::Text variant
        clickable: Option<Clickable>,
    },
    Rectangle {
        transform: Transform,
        physics: Option<Physics>,
        shape: Shape, // Must be Shape::Rectangle variant
        clickable: Option<Clickable>,
    },
}

impl Entity {
    // Factories!!
    pub fn new_circle(position: [f32; 2], radius: f32, color: [f32; 3]) -> Self {
        Entity::Circle {
            transform: Transform::new(position),
            physics: None,
            shape: Shape::Circle { radius, color },
            clickable: None,
        }
    }
    pub fn new_rectangle(position: [f32; 2], length: f32, height: f32, color: [f32; 3]) -> Self {
        Entity::Rectangle {
            transform: Transform::new(position),
            physics: None,
            shape: Shape::Rectangle {
                length,
                height,
                color,
            },
            clickable: None,
        }
    }

    pub fn new_text(position: [f32; 2], content: String, font_size: f32, color: [f32; 3]) -> Self {
        Entity::Text {
            transform: Transform::new(position),
            physics: None,
            shape: Shape::Text {
                content,
                font_size,
                color,
            },
            clickable: None,
        }
    }

    // Builder-style methods for adding components
    pub fn with_physics(mut self, physics: Physics) -> Self {
        if let Entity::Circle {
            physics: ref mut p, ..
        } = self
        {
            *p = Some(physics);
        }
        self
    }

    pub fn with_clickable(mut self, clickable: Clickable) -> Self {
        match &mut self {
            Entity::Circle { clickable: c, .. }
            | Entity::Text { clickable: c, .. }
            | Entity::Rectangle { clickable: c, .. } => {
                *c = Some(clickable);
            }
        }
        self
    }

    // Component accessors (immutable)
    pub fn transform(&self) -> &Transform {
        match self {
            Entity::Circle { transform, .. }
            | Entity::Text { transform, .. }
            | Entity::Rectangle { transform, .. } => transform,
        }
    }

    pub fn transform_mut(&mut self) -> &mut Transform {
        match self {
            Entity::Circle { transform, .. }
            | Entity::Text { transform, .. }
            | Entity::Rectangle { transform, .. } => transform,
        }
    }

    pub fn physics(&self) -> Option<&Physics> {
        match self {
            Entity::Circle { physics, .. }
            | Entity::Text { physics, .. }
            | Entity::Rectangle { physics, .. } => physics.as_ref(),
        }
    }

    pub fn physics_mut(&mut self) -> Option<&mut Physics> {
        match self {
            Entity::Circle { physics, .. }
            | Entity::Text { physics, .. }
            | Entity::Rectangle { physics, .. } => physics.as_mut(),
        }
    }
    pub fn physics_and_transform_mut(&mut self) -> Option<(&mut Physics, &mut Transform)> {
        match self {
            Entity::Circle {
                physics: Some(p),
                transform,
                ..
            } => Some((p, transform)),
            Entity::Text {
                physics: Some(p),
                transform,
                ..
            } => Some((p, transform)),
            Entity::Rectangle {
                physics: Some(p),
                transform,
                ..
            } => Some((p, transform)),
            _ => None,
        }
    }

    pub fn shape(&self) -> &Shape {
        match self {
            Entity::Circle { shape, .. }
            | Entity::Text { shape, .. }
            | Entity::Rectangle { shape, .. } => shape,
        }
    }

    pub fn shape_mut(&mut self) -> &mut Shape {
        match self {
            Entity::Circle { shape, .. }
            | Entity::Text { shape, .. }
            | Entity::Rectangle { shape, .. } => shape,
        }
    }

    pub fn clickable(&self) -> Option<&Clickable> {
        match self {
            Entity::Circle { clickable, .. }
            | Entity::Text { clickable, .. }
            | Entity::Rectangle { clickable, .. } => clickable.as_ref(),
        }
    }

    pub fn clickable_mut(&mut self) -> Option<&mut Clickable> {
        match self {
            Entity::Circle { clickable, .. }
            | Entity::Text { clickable, .. }
            | Entity::Rectangle { clickable, .. } => clickable.as_mut(),
        }
    }
}
