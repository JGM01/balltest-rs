// Declare all the modules in your project
mod components;
mod entity;
mod systems;
mod world;

// Import the types we need for our main function
use components::{Clickable, Physics};
use entity::Entity;
use systems::{InputCommand, InputSystem, PhysicsSystem, Renderer, TimeSystem};
use world::World;

use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};
mod collision;

// This is your application state
struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    world: World,
    physics_system: PhysicsSystem,
    input_system: InputSystem,
    time_system: TimeSystem,
}

impl App {
    fn new() -> Self {
        let mut world = World::new();

        // Create some initial entities (customize this as you like)
        // Ground platform
        world.add_entity(
            Entity::new_rectangle([0.0, -0.8], 1.8, 0.1, [0.5, 0.5, 0.5])
                .with_physics(Physics::new_static()),
        );

        // Some bouncy balls
        for i in 0..5 {
            let x = -0.6 + (i as f32) * 0.3;
            world.add_entity(
                Entity::new_circle([x, 0.5], 0.08, [1.0, 0.3, 0.3])
                    .with_physics(Physics::new().with_velocity([0.0, 0.0]))
                    .with_clickable(Clickable::new()),
            );
        }

        Self {
            window: None,
            renderer: None,
            world,
            physics_system: PhysicsSystem::new(),
            input_system: InputSystem::new(),
            time_system: TimeSystem::new(),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attrs = Window::default_attributes()
                .with_title("balltest-rs")
                .with_inner_size(winit::dpi::LogicalSize::new(800, 600));

            let window = Arc::new(event_loop.create_window(window_attrs).unwrap());
            let renderer = pollster::block_on(Renderer::new(window.clone()));

            self.window = Some(window);
            self.renderer = Some(renderer);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(new_size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(new_size);
                    self.input_system
                        .update_window_size(new_size.width, new_size.height);
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.input_system.update_cursor(position);

                // Update hover states for clickable entities
                if let Some(ndc) = self.input_system.cursor_ndc {
                    for entity in self.world.entities_mut() {
                        // First check if the cursor is over this entity
                        let is_hovered = entity.contains_point(ndc);

                        // Then update the clickable component if it exists
                        // We access the field directly now instead of using a method
                        if let Some(clickable) = entity.clickable.as_mut() {
                            clickable.hovered = is_hovered;
                        }
                    }
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if state == ElementState::Pressed {
                    if let Some(cmd) = self.input_system.handle_mouse_button(button, true) {
                        match cmd {
                            InputCommand::Click { position } => {
                                // Add a new ball where clicked
                                self.world.add_entity(
                                    Entity::new_circle(position, 0.08, [0.3, 0.6, 1.0])
                                        .with_physics(Physics::new())
                                        .with_clickable(Clickable::new()),
                                );
                            }
                            _ => {}
                        }
                    }
                }
            }

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(keycode),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                if let Some(cmd) = self.input_system.handle_key(keycode) {
                    match cmd {
                        InputCommand::Exit => event_loop.exit(),
                        InputCommand::TogglePause => self.time_system.toggle_pause(),
                        _ => {}
                    }
                }
            }

            WindowEvent::ModifiersChanged(modifiers) => {
                self.input_system.update_modifiers(modifiers.state());
            }

            WindowEvent::RedrawRequested => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.render(&self.world);
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let (sim_steps, _fps_update, needs_redraw, _alpha) = self.time_system.tick(now);

        // Run physics simulation steps
        for _ in 0..sim_steps {
            self.physics_system
                .update(&mut self.world, self.time_system.sim_dt());
            if let Some(renderer) = &mut self.renderer {
                renderer.frame_stats.sim_steps_accum += 1;
            }
        }

        // Request redraw if needed
        if needs_redraw {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }

        // Set up next wake time for smooth simulation
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.time_system.next_wakeup()));
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new();

    event_loop.run_app(&mut app).unwrap();
}
