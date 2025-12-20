use std::sync::Arc;

use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, ModifiersState, PhysicalKey},
    window::{Window, WindowId},
};

// Just a structure to encompass the state of the program.
struct State {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    render_pipeline: wgpu::RenderPipeline,

    // ECS stuff will go here (positions, colors, radii)
    instance_buffer: wgpu::Buffer,

    // Verticies for the circles (6 each) (doing quads) (2 triangles)
    vertex_buffer: wgpu::Buffer,

    // circle count
    circles: Vec<CircleInstance>,
    cursor_position: Option<winit::dpi::PhysicalPosition<f64>>,

    font_system: glyphon::FontSystem,
    swash_cache: glyphon::SwashCache,
    viewport: glyphon::Viewport,
    atlas: glyphon::TextAtlas,
    text_renderer: glyphon::TextRenderer,
    text_buffer: glyphon::Buffer,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2], // x, y of vertex
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CircleInstance {
    position: [f32; 2], // Circle Center; NDC (Normalized Device Coordinates (-1 -> +1))
    radius: f32,        // Circle Radius; NDC
    color: [f32; 3],    // RGB format
}

// To be copied numerous times :D
// The quad is 2x2 and centered at NDC origin.
// Vertex shader will:
// - scale it by `radius`
// - translate by `pos`
// Fragment shader will:
// - Color each inner pixel using `color`
const QUAD_VERTICES: &[Vertex] = &[
    // Triangle 1
    Vertex {
        position: [-1.0, -1.0],
    }, // bottom-left
    Vertex {
        position: [1.0, -1.0],
    }, // bottom-right
    Vertex {
        position: [1.0, 1.0],
    }, // top-right
    // Triangle 2
    Vertex {
        position: [-1.0, -1.0],
    }, // bottom-left
    Vertex {
        position: [1.0, 1.0],
    }, // top-right
    Vertex {
        position: [-1.0, 1.0],
    }, // top-left
];

impl State {
    async fn new(window: Arc<Window>) -> State {
        // Starts a WebGPU instance, which intializes the underlying graphics API and provides
        // access to the GPUs on the system (enumerated as "adapters"). Also finds presentable
        // targets ("surfaces").
        // - Out-lives adapters and devices.
        // - Handles debugging, tracing, logs, backend-specific stuff (DX12? Vulkan? etc etc).
        // - Loosely corresponds to implicit global state in navigator.gpu from the JS spec.
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

        // An adapter represents physical hardware and/or a software emulation, as well as the
        // browser's own implementation/stack on top of the native API (DX12, Vulkan, Metal).
        // - There can be multiple adapters. I.e. discrete v.s. integrated GPU.
        // - Browser controls what adapters are exposed.
        // - Properties include:
        //  - Features: texture-compression-bc, shader-f16 (corresponds to GPUFeatureName in spec).
        //  - Limits: maxTextureDimension3D, maxUniformBufferBindingSize
        //  - Info: vendor, architecture, device name
        // - Does not execute work! That is for the device, which is acquired through the adapter.
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();

        // A "logical" device. Useful for seperating data between tabs and browser windows. Each
        // tab/window believes it has some arbitrary GPU even if they all share the same GPU.
        // - PRIMARY INTERFACE FOR USING THE WEBGPU!
        //  - Creates resources (buffers, textures, samplers, bind groups, pipelines)
        //  - Has a queue of command buffers
        //  - Errors occur here! resource lifetimes, lost device, etc.
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();

        let size = window.inner_size();
        let scale_factor = window.scale_factor();

        // Explicitly represents a platform-specific drawable area (usually a window or canvas).
        // - NOT IN THE SPEC! This is how native APIs hook into whatever drawable surface it can
        // get. Pretty much anything that can take a draw call. On the web (JS), it's just a
        // canvas. (GPUCanvasContext)
        // - Created from the instance. Just pass in a raw window handle and it's figured out.
        // - Owns the swap chain configuration.
        // - Each frame: surface.get_current_texture() returns a SurfaceTexture (wrapping a
        // GPUTexture view) to render into, then present() it.
        // - get_capabilities will provide what you can do with the device.
        let surface = instance.create_surface(window.clone()).unwrap();

        // Get a surface's capabilities when used with the specific adapter we've gotten.
        // - formats: 8 bit color ? HDR ?
        // - present modes: fifo (vsync)? immediate (tearing)? mailbox (smart queue)?
        // - alpha modes: What happens behind the surface? Opaque? transparent? translucent?
        // - usages: corresponds to usage flags, helps with allocation. Copy source? Texture
        // sample? Storage texture?
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Circle Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // Create render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Circle Pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    // Vertex buffer layout
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                    },
                    // Instance buffer layout
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<CircleInstance>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![
                            1 => Float32x2,  // position
                            2 => Float32,    // radius
                            3 => Float32x3,  // color
                        ],
                    },
                ],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // Create vertex buffer (static)
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Create instance buffer (dynamic, preallocated for 50 circles)
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size: (std::mem::size_of::<CircleInstance>() * 50) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Initialize with some test circles
        let circles = vec![
            CircleInstance {
                position: [0.0, 0.0],
                radius: 0.3,
                color: [1.0, 0.0, 0.0], // Red
            },
            CircleInstance {
                position: [0.5, 0.5],
                radius: 0.2,
                color: [0.0, 1.0, 0.0], // Green
            },
            CircleInstance {
                position: [-0.5, -0.5],
                radius: 0.15,
                color: [0.0, 0.0, 1.0], // Blue
            },
        ];

        let cursor_position: Option<winit::dpi::PhysicalPosition<f64>> = None;

        // Set up text renderer
        let mut font_system = glyphon::FontSystem::new();
        let swash_cache = glyphon::SwashCache::new();
        let cache = glyphon::Cache::new(&device);
        let viewport = glyphon::Viewport::new(&device, &cache);
        let mut atlas =
            glyphon::TextAtlas::new(&device, &queue, &cache, wgpu::TextureFormat::Bgra8UnormSrgb);
        let text_renderer = glyphon::TextRenderer::new(
            &mut atlas,
            &device,
            wgpu::MultisampleState::default(),
            None,
        );
        let mut text_buffer =
            glyphon::Buffer::new(&mut font_system, glyphon::Metrics::new(30.0, 42.0));

        let physical_width = (size.width as f64 * scale_factor) as f32;
        let physical_height = (size.height as f64 * scale_factor) as f32;

        text_buffer.set_size(
            &mut font_system,
            Some(physical_width),
            Some(physical_height),
        );
        text_buffer.set_text(&mut font_system, "Hello world! 👋\nThis is rendered with 🦅 glyphon 🦁\nThe text below should be partially clipped.\na b c d e f g h i j k l m n o p q r s t u v w x y z", &glyphon::Attrs::new().family(glyphon::Family::SansSerif), glyphon::Shaping::Advanced
            ,None,);
        text_buffer.shape_until_scroll(&mut font_system, false);

        let state = State {
            window,
            device,
            queue,
            size,
            surface,
            surface_format,
            render_pipeline,
            vertex_buffer,
            instance_buffer,
            circles,
            cursor_position,
            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,
            text_buffer,
        };

        // Configure surface for the first time
        state.configure_surface();

        state
    }

    fn configure_surface(&self) {
        let surface_config = wgpu::SurfaceConfiguration {
            // RENDER_ATTACHMENT means the texture can be used as a color or depth/stencil attachment in a render pass.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,

            format: self.surface_format,

            // Request compatibility with the sRGB-format texture view we‘re going to create later.
            view_formats: vec![self.surface_format.add_srgb_suffix()],

            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.configure_surface();

        // Allow text to reflow on resize
        self.text_buffer.set_size(
            &mut self.font_system,
            Some(new_size.width as f32),
            Some(new_size.height as f32),
        );
    }

    fn handle_keys(&self, event_loop: &ActiveEventLoop, key: KeyCode, modifiers: ModifiersState) {
        match key {
            KeyCode::Escape => {
                println!("ESC key pressed; stopping");
                event_loop.exit();
            }
            KeyCode::KeyC if modifiers.control_key() => {
                println!("CTRL+C pressed");
                // Copy logic goes here :D
            }
            KeyCode::KeyV if modifiers.control_key() => {
                println!("CTRL+V pressed");
                // Paste logic goes here :D
            }
            KeyCode::KeyA if modifiers.control_key() => {
                println!("CTRL+A pressed");
                // Copy-All logic goes here :D
            }
            _ => (),
        }
    }

    fn render(&mut self) {
        // Update instance buffer with current circle data
        self.queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&self.circles),
        );

        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        let mut encoder = self.device.create_command_encoder(&Default::default());

        {
            let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            // Draw circles
            renderpass.set_pipeline(&self.render_pipeline);
            renderpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            renderpass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            renderpass.draw(0..6, 0..self.circles.len() as u32);

            // Draw text (glyphon middleware style)
            self.text_renderer
                .render(&mut self.atlas, &mut self.viewport, &mut renderpass)
                .unwrap();
        }
        self.atlas.trim();

        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();
    }
}

#[derive(Default)]
struct App {
    state: Option<State>,
    modifiers: ModifiersState,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window object
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );

        let state = pollster::block_on(State::new(window.clone()));
        self.state = Some(state);

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let app_state = self.state.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                app_state.circles[0].position[0] += 0.005;
                if app_state.circles[0].position[0] > 1.2 {
                    app_state.circles[0].position[0] = -1.2;
                }

                // glyphon prep
                app_state.viewport.update(
                    &app_state.queue,
                    glyphon::Resolution {
                        width: app_state.size.width,
                        height: app_state.size.height,
                    },
                );

                app_state
                    .text_renderer
                    .prepare(
                        &app_state.device,
                        &app_state.queue,
                        &mut app_state.font_system,
                        &mut app_state.atlas,
                        &app_state.viewport,
                        [glyphon::TextArea {
                            buffer: &app_state.text_buffer,
                            left: 10.0,
                            top: 10.0,
                            scale: 1.0,
                            bounds: glyphon::TextBounds {
                                left: 0,
                                top: 0,
                                right: 600,
                                bottom: 160,
                            },
                            default_color: glyphon::Color::rgb(255, 255, 255),
                            custom_glyphs: &[],
                        }],
                        &mut app_state.swash_cache,
                    )
                    .unwrap();

                app_state.render();

                app_state.window.request_redraw();
            }
            WindowEvent::Resized(size) => {
                // Reconfigures the size of the surface. We do not re-render
                // here as this event is always followed up by redraw request.
                app_state.resize(size);
            }

            WindowEvent::ModifiersChanged(new_modifiers) => {
                self.modifiers = new_modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state.is_pressed() {
                    if let PhysicalKey::Code(keycode) = event.physical_key {
                        app_state.handle_keys(event_loop, keycode, self.modifiers);
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                app_state.cursor_position = Some(position);
                println!("Cursor position: x={}, y={}", position.x, position.y);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(position) = app_state.cursor_position {
                    println!(
                        "Mouse {:?} {:?} at x={}, y={}",
                        button, state, position.x, position.y
                    );
                }
            }
            _ => (),
        }
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();

    // When the current loop iteration finishes, immediately begin a new
    // iteration regardless of whether or not new events are available to
    // process. Preferred for applications that want to render as fast as
    // possible, like games.
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
