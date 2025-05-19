use std::sync::Arc;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::wgpu_utils::{self, Vertex};

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.0868241, 0.49240386, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // A
    Vertex {
        position: [-0.49513406, 0.06958647, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // B
    Vertex {
        position: [-0.21918549, -0.44939706, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // C
    Vertex {
        position: [0.35966998, -0.3473291, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // D
    Vertex {
        position: [0.44147372, 0.2347359, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // E
];

const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];

const ALT_VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.5, -0.5, 0.0],
        color: [1.0, 0.0, 0.0],
    }, // bottom-left
    Vertex {
        position: [0.5, -0.5, 0.0],
        color: [0.0, 1.0, 0.0],
    }, // bottom-right
    Vertex {
        position: [0.5, 0.5, 0.0],
        color: [0.0, 0.0, 1.0],
    }, // top-right
    Vertex {
        position: [-0.5, 0.5, 0.0],
        color: [1.0, 1.0, 0.0],
    }, // top-left
];

const ALT_INDICES: &[u16] = &[
    0, 1, 2, // first triangle
    0, 2, 3, // second triangle
];

pub struct RendererState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,

    vertex_buffer: wgpu::Buffer,
    alt_vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    alt_index_buffer: wgpu::Buffer,
    num_indices: u32,
    alt_num_indices: u32,
}

impl RendererState {
    pub async fn new(window: &Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = wgpu_utils::create_gpu_instance();
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = wgpu_utils::create_adapter(instance, &surface).await;
        let (device, queue) = wgpu_utils::create_device(&adapter).await;
        let surface_caps = surface.get_capabilities(&adapter);
        let config = wgpu_utils::create_surface_config(size, surface_caps);
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let render_pipeline = wgpu_utils::create_render_pipeline(&device, &config, shader);
        let vertex_buffer = wgpu_utils::create_vertex_buffer(&device, VERTICES);
        let alt_vertex_buffer = wgpu_utils::create_vertex_buffer(&device, ALT_VERTICES);
        let index_buffer = wgpu_utils::create_index_buffer(&device, INDICES);
        let alt_index_buffer = wgpu_utils::create_index_buffer(&device, ALT_INDICES);
        let num_indices = INDICES.len() as u32;
        let alt_num_indices = ALT_INDICES.len() as u32;

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            alt_vertex_buffer,
            index_buffer,
            alt_index_buffer,
            num_indices,
            alt_num_indices,
        }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }

        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::Space),
                        ..
                    },
                ..
            } => {
                let temp_vert = self.vertex_buffer.clone();
                self.vertex_buffer = self.alt_vertex_buffer.clone();
                self.alt_vertex_buffer = temp_vert;

                let temp_ind = self.index_buffer.clone();
                self.index_buffer = self.alt_index_buffer.clone();
                self.alt_index_buffer = temp_ind;

                std::mem::swap(&mut self.num_indices, &mut self.alt_num_indices);
            }
            _ => return false,
        }

        true
    }

    pub fn update(&mut self) {}

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder =
            self.device
                .create_command_encoder(&wgpu::wgt::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
