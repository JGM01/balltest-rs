use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
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
}

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

        let state = State {
            window,
            device,
            queue,
            size,
            surface,
            surface_format,
        };

        // Configure surface for the first time
        state.configure_surface();

        state
    }

    fn get_window(&self) -> &Window {
        &self.window
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

        // reconfigure the surface
        self.configure_surface();
    }

    fn handle_keys(&self, event_loop: &ActiveEventLoop, key: KeyCode) {
        match key {
            KeyCode::Escape => {
                println!("ESC key pressed; stopping");
                event_loop.exit();
            }
            _ => (),
        }
    }
    fn render(&mut self) {
        // Create texture view
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                // Without add_srgb_suffix() the image we will be working with
                // might not be "gamma correct".
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        // Renders a GREEN screen
        let mut encoder = self.device.create_command_encoder(&Default::default());
        // Create the renderpass which will clear the screen.
        let renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // If you wanted to call any drawing commands, they would go here.

        // End the renderpass.
        drop(renderpass);

        // Submit the command in the queue to execute
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();
    }
}

#[derive(Default)]
struct App {
    state: Option<State>,
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
        let state = self.state.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                state.render();
                // Emits a new redraw requested event.
                state.get_window().request_redraw();
            }
            WindowEvent::Resized(size) => {
                // Reconfigures the size of the surface. We do not re-render
                // here as this event is always followed up by redraw request.
                state.resize(size);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state.is_pressed() {
                    if let PhysicalKey::Code(keycode) = event.physical_key {
                        state.handle_keys(event_loop, keycode);
                    }
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
