#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub position: [f32; 2], // NDC
    pub rotation: f32,      // radians
    pub scale: [f32; 2],    // NDC
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
}

impl Physics {
    pub fn new() -> Self {
        Self {
            velocity: [0.0, 0.0],
            acceleration: [0.0, 0.0],
            mass: 1.0,
            apply_gravity: true,
        }
    }

    pub fn with_velocity(mut self, velocity: [f32; 2]) -> Self {
        self.velocity = velocity;
        self
    }
}

#[derive(Clone, Debug)]
pub enum Shape {
    Circle {
        radius: f32,     // NDC
        color: [f32; 3], // RGB-format
    },
    Text {
        content: String, // I.E. "Hey whats up guys"
        font_size: f32,
        color: [f32; 3],
    },
    Rectangle {
        length: f32,
        height: f32,
        color: [f32; 3],
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
