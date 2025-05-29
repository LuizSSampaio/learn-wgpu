use cgmath::prelude::*;
use std::sync::Arc;
use winit::{dpi::PhysicalSize, event::WindowEvent, window::Window};

use crate::{
    camera::{Camera, CameraController, CameraUniform},
    instance::Instance,
    model::{self, DrawModel},
    resources, texture, wgpu_utils,
};

const NUM_INSTANCES_PER_ROW: u32 = 10;

pub struct RendererState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,

    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,

    deth_texture: texture::Texture,

    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: CameraController,

    obj_model: model::Model,
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
        let diffuse_bind_group_layout = wgpu_utils::create_diffuse_bind_group_layout(&device);
        let camera_bind_group_layout = wgpu_utils::create_camera_bind_group_layout(&device);
        let render_pipeline = wgpu_utils::create_render_pipeline(
            &device,
            &config,
            &diffuse_bind_group_layout,
            &camera_bind_group_layout,
            shader,
        );

        let deth_texture = texture::Texture::create_deth_texture(&device, &config, "Deth Texture");

        let camera = Camera {
            eye: (0.0, 1.0, 2.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: config.width as f32 / config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);
        let camera_buffer = wgpu_utils::create_camera_buffer(&device, &camera_uniform);
        let camera_bind_group = wgpu_utils::create_camera_bind_buffer(
            &device,
            &camera_buffer,
            &camera_bind_group_layout,
        );
        let camera_controller = CameraController::new(0.02);

        const SPACE_BETWEEN: f32 = 3.0;
        let instances = (0..NUM_INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                    let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                    let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

                    let position = cgmath::Vector3 { x, y: 0.0, z };

                    let rotation = if position.is_zero() {
                        cgmath::Quaternion::from_axis_angle(
                            cgmath::Vector3::unit_z(),
                            cgmath::Deg(0.0),
                        )
                    } else {
                        cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
                    };

                    Instance { position, rotation }
                })
            })
            .collect::<Vec<_>>();
        let instance_buffer = wgpu_utils::create_instance_buffer(&device, &instances);

        let obj_model =
            resources::load_model("cube.obj", &device, &queue, &diffuse_bind_group_layout)
                .await
                .unwrap();

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            deth_texture,
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_controller,
            instances,
            instance_buffer,
            obj_model,
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
        self.deth_texture =
            texture::Texture::create_deth_texture(&self.device, &self.config, "Deth Texture");
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        self.camera_controller.process_events(event)
    }

    pub fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.deth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_pipeline(&self.render_pipeline);

            render_pass.draw_model_instanced(
                &self.obj_model,
                0..self.instances.len() as u32,
                &self.camera_bind_group,
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
