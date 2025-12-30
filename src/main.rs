// src/main.rs - Updated version demonstrating the new system

mod collision_system;
mod components;
mod entity;
mod gjk;
mod shape;
mod systems;
mod world;

use collision_system::{Collider, ShapeVariant};
use components::{Appearance, Clickable, RigidBody, Transform};
use entity::Entity;
use shape::{Circle, CompoundShape, ConvexPolygon, ShapeClone, Vec2};
use systems::{InputCommand, InputSystem, PhysicsSystem, Renderer, TimeSystem};
use world::World;

use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::shape::Shape;

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

        // Ground platform (static rectangle)
        let ground_shape = Box::new(ConvexPolygon::box_shape(0.9, 0.05));
        let _ground_mass = ground_shape.compute_mass_properties(1.0);

        world.add_entity(
            Entity::new(
                Transform::new([0.0, -0.8]),
                Appearance::Polygon {
                    vertices: vec![[-0.9, -0.05], [0.9, -0.05], [0.9, 0.05], [-0.9, 0.05]],
                    color: [0.3, 0.3, 0.3],
                },
            )
            .with_rigid_body(RigidBody::new_static())
            .with_collider(Collider::from_shape(ground_shape)),
        );

        // Simple circles
        for i in 0..3 {
            let x = -0.4 + (i as f32) * 0.3;
            let circle_shape = Box::new(Circle::new(0.08));
            let mass_props = circle_shape.compute_mass_properties(1.0);

            world.add_entity(
                Entity::new(
                    Transform::new([x, 0.6]),
                    Appearance::Circle {
                        radius: 0.08,
                        color: [1.0, 0.3, 0.3],
                    },
                )
                .with_rigid_body(RigidBody::new(mass_props.mass, mass_props.inertia))
                .with_collider(Collider::from_shape(circle_shape))
                .with_clickable(Clickable::new()),
            );
        }

        // Compound concave shape: L-shape made from two rectangles
        let mut compound = CompoundShape::new();

        // Vertical part of L
        compound.add_child(
            Box::new(ConvexPolygon::box_shape(0.04, 0.15)),
            Transform::new([0.0, 0.0]),
        );

        // Horizontal part of L
        compound.add_child(
            Box::new(ConvexPolygon::box_shape(0.15, 0.04)),
            Transform::new([0.0, -0.11]),
        );

        let compound_mass = compound.compute_combined_mass_properties(1.0);

        world.add_entity(
            Entity::new(
                Transform::new([0.5, 0.3]).with_rotation(0.3),
                Appearance::Compound {
                    parts: vec![
                        Appearance::Polygon {
                            vertices: vec![
                                [-0.04, -0.15],
                                [0.04, -0.15],
                                [0.04, 0.15],
                                [-0.04, 0.15],
                            ],
                            color: [0.3, 0.6, 1.0],
                        },
                        Appearance::Polygon {
                            vertices: vec![
                                [-0.15, -0.15],
                                [0.15, -0.15],
                                [0.15, -0.07],
                                [-0.15, -0.07],
                            ],
                            color: [0.3, 0.6, 1.0],
                        },
                    ],
                },
            )
            .with_rigid_body(RigidBody::new(compound_mass.mass, compound_mass.inertia))
            .with_collider(Collider::from_compound(compound))
            .with_clickable(Clickable::new()),
        );

        // Pentagon (convex polygon)
        let pentagon = Box::new(ConvexPolygon::regular(0.1, 5));
        let pent_mass = pentagon.compute_mass_properties(1.0);

        let pent_verts: Vec<[f32; 2]> = (0..5)
            .map(|i| {
                let angle = (i as f32) * 2.0 * std::f32::consts::PI / 5.0;
                [0.1 * angle.cos(), 0.1 * angle.sin()]
            })
            .collect();

        world.add_entity(
            Entity::new(
                Transform::new([-0.5, 0.4]),
                Appearance::Polygon {
                    vertices: pent_verts,
                    color: [1.0, 0.8, 0.2],
                },
            )
            .with_rigid_body(RigidBody::new(pent_mass.mass, pent_mass.inertia))
            .with_collider(Collider::from_shape(pentagon))
            .with_clickable(Clickable::new()),
        );

        // Compound shape: H-shape from three rectangles
        let mut h_shape = CompoundShape::new();

        // Left vertical
        h_shape.add_child(
            Box::new(ConvexPolygon::box_shape(0.03, 0.12)),
            Transform::new([-0.08, 0.0]),
        );

        // Right vertical
        h_shape.add_child(
            Box::new(ConvexPolygon::box_shape(0.03, 0.12)),
            Transform::new([0.08, 0.0]),
        );

        // Middle horizontal
        h_shape.add_child(
            Box::new(ConvexPolygon::box_shape(0.08, 0.03)),
            Transform::new([0.0, 0.0]),
        );

        let h_mass = h_shape.compute_combined_mass_properties(1.0);

        world.add_entity(
            Entity::new(
                Transform::new([0.0, 0.5]),
                Appearance::Compound {
                    parts: vec![
                        Appearance::Polygon {
                            vertices: vec![
                                [-0.11, -0.12],
                                [-0.05, -0.12],
                                [-0.05, 0.12],
                                [-0.11, 0.12],
                            ],
                            color: [0.8, 0.3, 0.8],
                        },
                        Appearance::Polygon {
                            vertices: vec![
                                [0.05, -0.12],
                                [0.11, -0.12],
                                [0.11, 0.12],
                                [0.05, 0.12],
                            ],
                            color: [0.8, 0.3, 0.8],
                        },
                        Appearance::Polygon {
                            vertices: vec![
                                [-0.08, -0.03],
                                [0.08, -0.03],
                                [0.08, 0.03],
                                [-0.08, 0.03],
                            ],
                            color: [0.8, 0.3, 0.8],
                        },
                    ],
                },
            )
            .with_rigid_body(RigidBody::new(h_mass.mass, h_mass.inertia))
            .with_collider(Collider::from_compound(h_shape))
            .with_clickable(Clickable::new()),
        );

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
                .with_title("Generic Collision System Demo - GJK/EPA with Compound Shapes")
                .with_inner_size(winit::dpi::LogicalSize::new(1000, 800));

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
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if state == ElementState::Pressed {
                    if let Some(cmd) = self.input_system.handle_mouse_button(button, true) {
                        match cmd {
                            InputCommand::Click { position } => {
                                // Spawn a dynamic circle
                                let circle_shape = Box::new(Circle::new(0.06));
                                let mass_props = circle_shape.compute_mass_properties(1.0);

                                self.world.add_entity(
                                    Entity::new(
                                        Transform::new(position),
                                        Appearance::Circle {
                                            radius: 0.06,
                                            color: [0.2, 1.0, 0.5],
                                        },
                                    )
                                    .with_rigid_body(RigidBody::new(
                                        mass_props.mass,
                                        mass_props.inertia,
                                    ))
                                    .with_collider(Collider::from_shape(circle_shape))
                                    .with_clickable(Clickable::new()),
                                );
                            }
                            InputCommand::RightClick { position } => {
                                // Spawn a box
                                let box_shape = Box::new(ConvexPolygon::box_shape(0.06, 0.06));
                                let mass_props = box_shape.compute_mass_properties(1.0);

                                self.world.add_entity(
                                    Entity::new(
                                        Transform::new(position),
                                        Appearance::Polygon {
                                            vertices: vec![
                                                [-0.06, -0.06],
                                                [0.06, -0.06],
                                                [0.06, 0.06],
                                                [-0.06, 0.06],
                                            ],
                                            color: [1.0, 0.5, 0.2],
                                        },
                                    )
                                    .with_rigid_body(RigidBody::new(
                                        mass_props.mass,
                                        mass_props.inertia,
                                    ))
                                    .with_collider(Collider::from_shape(box_shape))
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

        for _ in 0..sim_steps {
            self.physics_system
                .update(&mut self.world, self.time_system.sim_dt());
            if let Some(renderer) = &mut self.renderer {
                renderer.frame_stats.sim_steps_accum += 1;
            }
        }

        if needs_redraw {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }

        event_loop.set_control_flow(ControlFlow::WaitUntil(self.time_system.next_wakeup()));
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new();

    println!("  Left Click:  Spawn circle");
    println!("  Right Click: Spawn box");
    println!("  Space/P:     Pause/Resume");
    println!("  ESC:         Exit");

    event_loop.run_app(&mut app).unwrap();
}
