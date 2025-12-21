use winit::{
    dpi::PhysicalPosition,
    event::MouseButton,
    keyboard::{KeyCode, ModifiersState},
};

/// InputSystem translates raw input events into application commands/state
pub struct InputSystem {
    pub modifiers: ModifiersState,
    pub cursor_position: Option<PhysicalPosition<f64>>,
    pub cursor_ndc: Option<[f32; 2]>,
    pub window_size: (u32, u32),
}

impl InputSystem {
    pub fn new() -> Self {
        Self {
            modifiers: ModifiersState::empty(),
            cursor_position: None,
            cursor_ndc: None,
            window_size: (800, 600),
        }
    }

    pub fn update_modifiers(&mut self, modifiers: ModifiersState) {
        self.modifiers = modifiers;
    }

    pub fn update_window_size(&mut self, width: u32, height: u32) {
        self.window_size = (width, height);

        // Recalculate NDC if we have a cursor position
        if let Some(pos) = self.cursor_position {
            self.cursor_ndc = Some(self.physical_to_ndc(pos, width, height));
        }
    }

    pub fn update_cursor(&mut self, position: PhysicalPosition<f64>) {
        self.cursor_position = Some(position);
        self.cursor_ndc =
            Some(self.physical_to_ndc(position, self.window_size.0, self.window_size.1));
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

    pub fn handle_mouse_button(&self, button: MouseButton, pressed: bool) -> Option<InputCommand> {
        if !pressed {
            return None;
        }

        match button {
            MouseButton::Left => {
                if let Some(ndc) = self.cursor_ndc {
                    Some(InputCommand::Click { position: ndc })
                } else {
                    None
                }
            }
            MouseButton::Right => {
                if let Some(ndc) = self.cursor_ndc {
                    Some(InputCommand::RightClick { position: ndc })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Convert physical pixel position to NDC coordinates
    pub fn physical_to_ndc(
        &self,
        position: PhysicalPosition<f64>,
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
    Click { position: [f32; 2] },
    RightClick { position: [f32; 2] },
}
