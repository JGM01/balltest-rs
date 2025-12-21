use std::{sync::Arc, time::Instant};

use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

mod components;
mod entity;
mod systems;
mod world;

use components::{Clickable, Physics};
use entity::Entity;
use systems::{InputCommand, InputSystem, PhysicsSystem, Renderer, TimeSystem};
use world::World;

struct App {
    // Systems
    renderer: Option<Renderer>,
    timing: TimeSystem,
    physics: PhysicsSystem,
    input: InputSystem,

    // World state
    world: World,
}

impl App {
    fn new() -> Self {
        let mut world = World::new();

        // Create test entities
        world.add_entity(
            Entity::new_circle([0.0, 0.0], 0.3, [1.0, 0.0, 0.0])
                //.with_physics(Physics::new().with_velocity([0.05, 0.0]))
                .with_clickable(Clickable::new()),
        );

        world.add_entity(
            Entity::new_circle([0.5, 0.5], 0.2, [0.0, 1.0, 0.0]).with_clickable(Clickable::new()),
        );

        world.add_entity(
            Entity::new_circle([-0.5, -0.5], 0.15, [0.0, 0.0, 1.0]), //.with_physics(Physics::new()),
        );

        // Add some rectangles
        world.add_entity(
            Entity::new_rectangle([-0.6, 0.0], 0.3, 0.4, [1.0, 1.0, 0.0])
                .with_clickable(Clickable::new()),
        );

        world.add_entity(
            Entity::new_rectangle([0.6, -0.3], 0.25, 0.25, [1.0, 0.0, 1.0])
                //.with_physics(Physics::new().with_velocity([0.0, 0.02]))
                .with_clickable(Clickable::new()),
        );

        // Add text entities with various features
        world.add_entity(
            Entity::new_text(
                [-0.8, 0.8],
                String::from("Click the shapes!"),
                32.0,
                [1.0, 1.0, 1.0],
            )
            .with_clickable(Clickable::new()),
        );

        world.add_entity(Entity::new_text(
            [0.0, -0.8],
            String::from("Press P or Space to pause\nPress ESC to exit"),
            24.0,
            [0.8, 0.8, 0.8],
        ));

        Self {
            renderer: None,
            timing: TimeSystem::new(),
            physics: PhysicsSystem::new(),
            input: InputSystem::new(),
            world,
        }
    }

    fn handle_input_command(&mut self, event_loop: &ActiveEventLoop, command: InputCommand) {
        match command {
            InputCommand::Exit => {
                println!("Exiting application");
                event_loop.exit();
            }
            InputCommand::TogglePause => {
                self.timing.toggle_pause();
                println!("Simulation paused: {}", self.timing.sim_time.as_secs());
            }
            InputCommand::Click { position } => {
                self.handle_click(position, false);
            }
            InputCommand::RightClick { position } => {
                self.handle_click(position, true);
            }
        }
    }

    fn handle_click(&mut self, position: [f32; 2], is_right_click: bool) {
        println!(
            "{} click at NDC ({:.3}, {:.3})",
            if is_right_click { "Right" } else { "Left" },
            position[0],
            position[1]
        );

        // Check which entities were clicked
        let mut clicked_any = false;
        for (idx, entity) in self.world.entities_mut().iter_mut().enumerate() {
            if entity.clickable().is_some() && entity.contains_point(position) {
                if let Some(clickable) = entity.clickable_mut() {
                    clickable.hovered = true;
                }

                println!(
                    "  Clicked entity {} at {:?}",
                    idx,
                    entity.transform().position
                );
                clicked_any = true;

                // change color on click
                match entity.shape_mut() {
                    components::Shape::Circle { color, .. } => {
                        if is_right_click {
                            *color = [
                                rand::random::<f32>(),
                                rand::random::<f32>(),
                                rand::random::<f32>(),
                            ];
                        } else {
                            // Invert colors
                            color[0] = 1.0 - color[0];
                            color[1] = 1.0 - color[1];
                            color[2] = 1.0 - color[2];
                        }
                    }
                    components::Shape::Rectangle { color, .. } => {
                        if is_right_click {
                            *color = [
                                rand::random::<f32>(),
                                rand::random::<f32>(),
                                rand::random::<f32>(),
                            ];
                        } else {
                            color[0] = 1.0 - color[0];
                            color[1] = 1.0 - color[1];
                            color[2] = 1.0 - color[2];
                        }
                    }
                    components::Shape::Text { color, .. } => {
                        color[0] = 1.0 - color[0];
                        color[1] = 1.0 - color[1];
                        color[2] = 1.0 - color[2];
                    }
                }

                if is_right_click && entity.physics().is_none() {
                    println!("  Adding physics to entity {}", idx);
                    // fake!
                }
            }
        }

        if !clicked_any {
            println!("  No clickable entities at this position");
        }

        // Request redraw to show changes
        if let Some(renderer) = &self.renderer {
            renderer.window.request_redraw();
        }
    }

    fn handle_hover(&mut self, position: [f32; 2]) {
        for entity in self.world.entities_mut() {
            // Immutable work first
            let is_hovered = entity.contains_point(position);

            // Now mutate
            if let Some(clickable) = entity.clickable_mut() {
                let was_hovered = clickable.hovered;
                clickable.hovered = is_hovered;

                if is_hovered && !was_hovered {
                    // Hover just started
                }
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );

        let size = window.inner_size();
        self.input.update_window_size(size.width, size.height);

        let renderer = pollster::block_on(Renderer::new(window.clone()));
        self.renderer = Some(renderer);

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("Window close requested");
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.render(&self.world);
                }
            }

            WindowEvent::Resized(size) => {
                self.input.update_window_size(size.width, size.height);

                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size);
                    renderer.window.request_redraw();
                }
            }

            WindowEvent::ModifiersChanged(new_modifiers) => {
                self.input.update_modifiers(new_modifiers.state());
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state.is_pressed() {
                    if let PhysicalKey::Code(keycode) = event.physical_key {
                        if let Some(command) = self.input.handle_key(keycode) {
                            self.handle_input_command(event_loop, command);
                        }
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                // Update input first
                self.input.update_cursor(position);

                // Use the derived data without extending borrows
                if let Some(ndc) = self.input.cursor_ndc {
                    self.handle_hover(ndc);
                }

                // Request redraw with a fresh renderer borrow
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.window.request_redraw();
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if state == ElementState::Pressed {
                    if let Some(command) = self.input.handle_mouse_button(button, true) {
                        self.handle_input_command(event_loop, command);
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                println!("Mouse wheel: {:?}", delta);
                // Hook for zoom / scroll
            }

            WindowEvent::PinchGesture { delta, .. } => {
                println!("Pinch gesture: {}", delta);
                // Hook for zoom
            }

            WindowEvent::TouchpadPressure { pressure, .. } => {
                println!("Touchpad pressure: {}", pressure);
            }

            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let renderer = self.renderer.as_mut().unwrap();
        let now = Instant::now();

        let (steps, .., needs_redraw, _alpha) = self.timing.tick(now);

        // Run physics simulation for fixed timesteps
        for _ in 0..steps {
            self.physics.update(&mut self.world, self.timing.sim_dt());
        }

        renderer.frame_stats.sim_steps_accum += steps;

        if needs_redraw {
            renderer.window.request_redraw();
        }

        let next = self.timing.next_wakeup();
        event_loop.set_control_flow(ControlFlow::WaitUntil(next));
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}
