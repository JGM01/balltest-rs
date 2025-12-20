use crate::components::Shape;
use crate::world::World;
use std::sync::Arc;
use std::time::{Duration, Instant};
use wgpu::util::DeviceExt;
use winit::window::Window;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CircleInstance {
    position: [f32; 2],
    radius: f32,
    color: [f32; 3],
}

const QUAD_VERTICES: &[Vertex] = &[
    Vertex {
        position: [-1.0, -1.0],
    },
    Vertex {
        position: [1.0, -1.0],
    },
    Vertex {
        position: [1.0, 1.0],
    },
    Vertex {
        position: [-1.0, -1.0],
    },
    Vertex {
        position: [1.0, 1.0],
    },
    Vertex {
        position: [-1.0, 1.0],
    },
];

pub struct FrameStats {
    pub last_present: Instant,
    pub frame_time_accum: Duration,
    pub frame_count: u32,
    pub avg_frame_time_ms: f32,
    pub present_fps: u32,

    pub sim_steps_accum: u32,
    pub sim_tps: u32,

    pub render_count: u32,
    pub render_fps: u32,

    pub last_report: Instant,
    pub report_dt: Duration,
}

impl FrameStats {
    pub fn new(now: Instant) -> Self {
        Self {
            last_present: now,
            frame_time_accum: Duration::ZERO,
            frame_count: 0,
            avg_frame_time_ms: 0.0,
            present_fps: 0,
            sim_steps_accum: 0,
            sim_tps: 0,
            render_count: 0,
            render_fps: 0,
            last_report: now,
            report_dt: Duration::from_secs(1),
        }
    }

    pub fn needs_update(&self) -> bool {
        Instant::now() - self.last_report >= self.report_dt
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let elapsed = now - self.last_report;
        let secs = elapsed.as_secs_f32();

        if self.frame_count > 0 {
            let avg_dt = self.frame_time_accum.as_secs_f32() / self.frame_count as f32;
            self.avg_frame_time_ms = avg_dt * 1000.0;
            self.present_fps = (1.0 / avg_dt).round() as u32;
        }

        self.sim_tps = (self.sim_steps_accum as f32 / secs).round() as u32;
        self.render_fps = (self.render_count as f32 / secs).round() as u32;

        self.frame_time_accum = Duration::ZERO;
        self.frame_count = 0;
        self.sim_steps_accum = 0;
        self.render_count = 0;
        self.last_report = now;
    }

    pub fn record_frame(&mut self, dt: Duration) {
        self.frame_time_accum += dt;
        self.frame_count += 1;
    }
}

pub struct Renderer {
    pub window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    size: winit::dpi::PhysicalSize<u32>,

    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,

    // Text rendering
    font_system: glyphon::FontSystem,
    swash_cache: glyphon::SwashCache,
    viewport: glyphon::Viewport,
    atlas: glyphon::TextAtlas,
    text_renderer: glyphon::TextRenderer,
    stats_buffer: glyphon::Buffer,
    text_dirty: bool,

    pub frame_stats: FrameStats,
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();

        let size = window.inner_size();
        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Circle Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shader.wgsl").into()),
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Circle Pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<CircleInstance>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![
                            1 => Float32x2,
                            2 => Float32,
                            3 => Float32x3,
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

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size: (std::mem::size_of::<CircleInstance>() * 100) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Text rendering setup
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

        let mut stats_buffer =
            glyphon::Buffer::new(&mut font_system, glyphon::Metrics::new(24.0, 32.0));
        stats_buffer.set_size(
            &mut font_system,
            Some(size.width as f32),
            Some(size.height as f32),
        );
        stats_buffer.set_text(
            &mut font_system,
            "FPS: --",
            &glyphon::Attrs::new().family(glyphon::Family::Monospace),
            glyphon::Shaping::Advanced,
            None,
        );
        stats_buffer.shape_until_scroll(&mut font_system, false);

        let renderer = Self {
            window,
            device,
            queue,
            surface,
            surface_format,
            size,
            render_pipeline,
            vertex_buffer,
            instance_buffer,
            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,
            stats_buffer,
            text_dirty: true,
            frame_stats: FrameStats::new(Instant::now()),
        };

        renderer.configure_surface();
        renderer
    }

    fn configure_surface(&self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.configure_surface();
        self.stats_buffer.set_size(
            &mut self.font_system,
            Some(new_size.width as f32),
            Some(new_size.height as f32),
        );
    }

    pub fn update_stats_text(&mut self) {
        let s = format!(
            "Frame:   {:5.2} ms ({:3} fps)\nSim:     {:3} ticks/s\nRender:  {:3} fps",
            self.frame_stats.avg_frame_time_ms,
            self.frame_stats.present_fps,
            self.frame_stats.sim_tps,
            self.frame_stats.render_fps,
        );

        self.stats_buffer.set_text(
            &mut self.font_system,
            &s,
            &glyphon::Attrs::new()
                .family(glyphon::Family::Monospace)
                .color(glyphon::Color::rgb(255, 255, 160)),
            glyphon::Shaping::Basic,
            None,
        );

        self.stats_buffer
            .shape_until_scroll(&mut self.font_system, false);

        self.text_dirty = true;
    }

    fn prepare_text(&mut self) {
        self.viewport.update(
            &self.queue,
            glyphon::Resolution {
                width: self.size.width,
                height: self.size.height,
            },
        );

        let (w, h) = self.stats_buffer.size();
        let text_width = w.unwrap_or(0.0);
        let text_height = h.unwrap_or(0.0);

        let margin = 12.0;
        let left = (self.size.width as f32 - text_width - margin)
            .max(margin)
            .round();
        let top = (self.size.height as f32 - text_height - margin)
            .max(margin)
            .round();

        self.text_renderer
            .prepare(
                &self.device,
                &self.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                [glyphon::TextArea {
                    buffer: &self.stats_buffer,
                    left,
                    top,
                    scale: 1.0,
                    bounds: glyphon::TextBounds::default(),
                    default_color: glyphon::Color::rgb(255, 255, 160),
                    custom_glyphs: &[],
                }],
                &mut self.swash_cache,
            )
            .unwrap();
    }

    pub fn render(&mut self, world: &World) {
        self.frame_stats.render_count += 1;

        // Collect circle instances from world
        let mut circles = Vec::new();
        for entity in world.entities() {
            if let Shape::Circle { radius, color } = entity.shape() {
                let transform = entity.transform();
                circles.push(CircleInstance {
                    position: transform.position,
                    radius: *radius,
                    color: *color,
                });
            }
        }

        // Upload instances
        self.queue
            .write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&circles));

        // Prepare text from entities
        let mut text_areas = Vec::new();
        let mut text_buffers = Vec::new();

        for entity in world.entities() {
            if let Shape::Text {
                content, font_size, ..
            } = entity.shape()
            {
                // Create a temporary buffer for this text
                let mut buffer = glyphon::Buffer::new(
                    &mut self.font_system,
                    glyphon::Metrics::new(*font_size, font_size * 1.4),
                );

                buffer.set_size(&mut self.font_system, None, None);
                buffer.set_text(
                    &mut self.font_system,
                    content,
                    &glyphon::Attrs::new().family(glyphon::Family::SansSerif),
                    glyphon::Shaping::Advanced,
                    None,
                );
                buffer.shape_until_scroll(&mut self.font_system, false);

                text_buffers.push(buffer);
            }
        }

        // Build text areas (need to borrow buffers after they're all created)
        for (_, entity) in world.entities().iter().enumerate() {
            if let Shape::Text {
                content: _,
                font_size: _,
                color,
            } = entity.shape()
            {
                let transform = entity.transform();

                let screen_x = ((transform.position[0] + 1.0) / 2.0) * self.size.width as f32;
                let screen_y = ((1.0 - transform.position[1]) / 2.0) * self.size.height as f32;

                // Find corresponding buffer index
                let buffer_idx = circles.len() + text_areas.len();
                if let Some(buffer) = text_buffers.get(buffer_idx - circles.len()) {
                    text_areas.push(glyphon::TextArea {
                        buffer,
                        left: screen_x,
                        top: screen_y,
                        scale: 1.0,
                        bounds: glyphon::TextBounds::default(),
                        default_color: glyphon::Color::rgb(
                            (color[0] * 255.0) as u8,
                            (color[1] * 255.0) as u8,
                            (color[2] * 255.0) as u8,
                        ),
                        custom_glyphs: &[],
                    });
                }
            }
        }

        // Prepare stats text
        if self.text_dirty {
            self.prepare_text();
            self.text_dirty = false;
        }

        // Update viewport
        self.viewport.update(
            &self.queue,
            glyphon::Resolution {
                width: self.size.width,
                height: self.size.height,
            },
        );

        // Prepare all text (entity text + stats)
        let (w, h) = self.stats_buffer.size();
        let stats_width = w.unwrap_or(0.0);
        let stats_height = h.unwrap_or(0.0);
        let margin = 12.0;
        let stats_left = (self.size.width as f32 - stats_width - margin)
            .max(margin)
            .round();
        let stats_top = (self.size.height as f32 - stats_height - margin)
            .max(margin)
            .round();

        let mut all_text_areas = text_areas;
        all_text_areas.push(glyphon::TextArea {
            buffer: &self.stats_buffer,
            left: stats_left,
            top: stats_top,
            scale: 1.0,
            bounds: glyphon::TextBounds::default(),
            default_color: glyphon::Color::rgb(255, 255, 160),
            custom_glyphs: &[],
        });

        self.text_renderer
            .prepare(
                &self.device,
                &self.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                all_text_areas,
                &mut self.swash_cache,
            )
            .unwrap();

        let surface_texture = self.surface.get_current_texture().unwrap();
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&Default::default());

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.draw(0..6, 0..circles.len() as u32);

            self.text_renderer
                .render(&mut self.atlas, &mut self.viewport, &mut render_pass)
                .unwrap();
        }

        self.queue.submit([encoder.finish()]);
        surface_texture.present();

        let now = Instant::now();
        let dt = now - self.frame_stats.last_present;
        self.frame_stats.last_present = now;
        self.frame_stats.record_frame(dt);

        if self.frame_stats.needs_update() {
            self.frame_stats.update();
            self.update_stats_text();
        }
    }
}
