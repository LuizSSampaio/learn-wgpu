use std::sync::Arc;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::wgpu_utils;

pub struct RendererState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    alt_render_pipeline: wgpu::RenderPipeline,
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
        let alt_shader = device.create_shader_module(wgpu::include_wgsl!("second.wgsl"));
        let alt_render_pipeline = wgpu_utils::create_render_pipeline(&device, &config, alt_shader);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            alt_render_pipeline,
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
                        physical_key: PhysicalKey::Code(KeyCode::Space),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                let temp = self.render_pipeline.clone();
                self.render_pipeline = self.alt_render_pipeline.clone();
                self.alt_render_pipeline = temp;
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
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
