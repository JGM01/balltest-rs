use crate::components::{Appearance, Clickable, Collider, Physics, Transform};

/// Entity with component-based architecture
/// All entities are the same type now, differentiated by which components they have
#[derive(Clone, Debug)]
pub struct Entity {
    pub transform: Transform,
    pub appearance: Appearance,
    pub physics: Option<Physics>,
    pub collider: Option<Collider>,
    pub clickable: Option<Clickable>,
}

impl Entity {
    /// Create a circle entity
    pub fn new_circle(position: [f32; 2], radius: f32, color: [f32; 3]) -> Self {
        Self {
            transform: Transform::new(position),
            appearance: Appearance::Circle { radius, color },
            physics: None,
            collider: Some(Collider::circle(radius)),
            clickable: None,
        }
    }

    /// Create a rectangle entity
    pub fn new_rectangle(position: [f32; 2], width: f32, height: f32, color: [f32; 3]) -> Self {
        Self {
            transform: Transform::new(position),
            appearance: Appearance::Rectangle {
                width,
                height,
                color,
            },
            physics: None,
            collider: Some(Collider::rectangle(width, height)),
            clickable: None,
        }
    }

    /// Create a text entity (no collision by default)
    pub fn new_text(position: [f32; 2], content: String, font_size: f32, color: [f32; 3]) -> Self {
        Self {
            transform: Transform::new(position),
            appearance: Appearance::Text {
                content,
                font_size,
                color,
            },
            physics: None,
            collider: None,
            clickable: None,
        }
    }

    /// Builder methods for adding components
    pub fn with_physics(mut self, physics: Physics) -> Self {
        self.physics = Some(physics);
        self
    }

    pub fn with_clickable(mut self, clickable: Clickable) -> Self {
        self.clickable = Some(clickable);
        self
    }

    pub fn with_collider(mut self, collider: Collider) -> Self {
        self.collider = Some(collider);
        self
    }

    /// Check if a point is inside this entity (for clicking)
    pub fn contains_point(&self, point: [f32; 2]) -> bool {
        if let Some(collider) = &self.collider {
            collider.shape().contains_point(&self.transform, point)
        } else {
            // Fallback for entities without colliders (like text)
            let dx = point[0] - self.transform.position[0];
            let dy = point[1] - self.transform.position[1];
            let dist_sq = dx * dx + dy * dy;
            dist_sq <= 0.1 * 0.1
        }
    }
}
