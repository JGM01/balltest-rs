use std::{sync::Arc, time::Instant};

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
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
                .with_physics(Physics::new().with_velocity([0.05, 0.0]))
                .with_clickable(Clickable::new()),
        );

        world.add_entity(
            Entity::new_circle([0.5, 0.5], 0.2, [0.0, 1.0, 0.0]).with_clickable(Clickable::new()),
        );

        world.add_entity(
            Entity::new_circle([-0.5, -0.5], 0.15, [0.0, 0.0, 1.0]).with_physics(Physics::new()),
        );

        world.add_entity(
            Entity::new_text(
                [-0.5, -0.5],
                String::from("THIS IS SOME TEXT!!! I HOPE IT RENDERS!!!"),
                24.0,
                [1.0, 1.0, 1.0],
            )
            .with_clickable(Clickable::new()),
        );
        world.add_entity(Entity::new_text(
            [-0.5, -0.5],
            String::from("THIS IS SOME TEXT!!!\nI HOPE IT RENDERS!!!\nIT SHOULD BE LARGER"),
            64.0,
            [1.0, 1.0, 0.0],
        ));
        world.add_entity(Entity::new_text(
            [-0.76, -0.43],
            String::from("THIS IS SOME TEXT!!!\nI HOPE IT RENDERS!!!\nIT SHOULD BE LARGER"),
            16.0,
            [0.0, 1.0, 0.0],
        ));
        world.add_entity(Entity::new_text(
            [0.5, 0.5],
            String::from("THIS IS SOME TEXT!!!\nI HOPE IT RENDERS!!!\nIT SHOULD BE LARGER"),
            48.0,
            [1.0, 0.0, 0.0],
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

        let renderer = pollster::block_on(Renderer::new(window.clone()));
        self.renderer = Some(renderer);

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let renderer = self.renderer.as_mut().unwrap();

        match event {
            WindowEvent::CloseRequested => {
                println!("Window close requested");
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                renderer.render(&self.world);
            }

            WindowEvent::Resized(size) => {
                renderer.resize(size);
                renderer.window.request_redraw();
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
                self.input.update_cursor(position);
                renderer.window.request_redraw();
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(position) = self.input.cursor_position {
                    println!(
                        "Mouse {:?} {:?} at ({}, {})",
                        button, state, position.x, position.y
                    );
                }
            }

            WindowEvent::MouseWheel {
                device_id,
                delta,
                phase,
            } => {
                println!("Mouse {:?} moved {:?} | {:?}", device_id, delta, phase);
            }

            WindowEvent::PinchGesture {
                device_id,
                delta,
                phase,
            } => {
                println!("Mouse {:?} pinched {:?} | {:?}", device_id, delta, phase);
            }
            WindowEvent::TouchpadPressure {
                device_id,
                pressure,
                stage,
            } => {
                println!(
                    "Mouse {:?} pressure {:?} | {:?}",
                    device_id, pressure, stage
                );
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
