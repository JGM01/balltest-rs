use winit::keyboard::{KeyCode, ModifiersState};

/// InputSystem translates raw input events into application commands/state
pub struct InputSystem {
    pub modifiers: ModifiersState,
    pub cursor_position: Option<winit::dpi::PhysicalPosition<f64>>,
}

impl InputSystem {
    pub fn new() -> Self {
        Self {
            modifiers: ModifiersState::empty(),
            cursor_position: None,
        }
    }

    pub fn update_modifiers(&mut self, modifiers: ModifiersState) {
        self.modifiers = modifiers;
    }

    pub fn update_cursor(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        self.cursor_position = Some(position);
    }

    pub fn handle_key(&self, keycode: KeyCode) -> Option<InputCommand> {
        match keycode {
            KeyCode::Escape => Some(InputCommand::Exit),
            KeyCode::KeyP => Some(InputCommand::TogglePause),
            KeyCode::Space => Some(InputCommand::TogglePause),
            KeyCode::KeyC if self.modifiers.control_key() => {
                println!("CTRL+C pressed");
                None // Could be InputCommand::Copy
            }
            KeyCode::KeyV if self.modifiers.control_key() => {
                println!("CTRL+V pressed");
                None // Could be InputCommand::Paste
            }
            _ => None,
        }
    }

    /// Convert physical pixel position to NDC coordinates
    pub fn physical_to_ndc(
        &self,
        position: winit::dpi::PhysicalPosition<f64>,
        width: u32,
        height: u32,
    ) -> [f32; 2] {
        let width = width as f32;
        let height = height as f32;

        if width <= 0.0 || height <= 0.0 {
            return [0.0, 0.0];
        }

        let x = (position.x as f32 / width) * 2.0 - 1.0;
        let y = 1.0 - (position.y as f32 / height) * 2.0;

        [x, y]
    }
}

/// Commands that the input system can emit
#[derive(Debug, Clone, Copy)]
pub enum InputCommand {
    Exit,
    TogglePause,
    // Future: Click(x, y), Drag(x, y), etc.
}
