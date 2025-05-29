use wgpu::{
    Adapter, BindGroup, BindGroupLayout, Buffer, Device, Instance, Queue, RenderPipeline,
    ShaderModule, Surface, SurfaceCapabilities, SurfaceConfiguration, util::DeviceExt,
};
use winit::dpi::PhysicalSize;

use crate::{
    camera::CameraUniform,
    instance::{self, InstanceRaw},
    model::{ModelVertex, Vertex},
    texture,
};

pub fn create_instance_buffer(device: &Device, instances: &[instance::Instance]) -> Buffer {
    let instance_data = instances
        .iter()
        .map(instance::Instance::to_raw)
        .collect::<Vec<_>>();

    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Instance Buffer"),
        contents: bytemuck::cast_slice(&instance_data),
        usage: wgpu::BufferUsages::VERTEX,
    })
}

pub fn create_camera_bind_buffer(
    device: &Device,
    camera_buffer: &Buffer,
    camera_bind_group_layout: &BindGroupLayout,
) -> BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Camera Bind Group"),
        layout: camera_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }],
    })
}

pub fn create_camera_bind_group_layout(device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Camera Bind Group Layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

pub fn create_camera_buffer(device: &Device, camera_uniform: &CameraUniform) -> Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Camera Buffer"),
        contents: bytemuck::cast_slice(&[camera_uniform.to_owned()]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    })
}

pub fn create_diffuse_bind_group_layout(device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Texture Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

pub fn create_render_pipeline(
    device: &Device,
    config: &SurfaceConfiguration,
    texture_bind_group_layout: &BindGroupLayout,
    camera_bind_group_layout: &BindGroupLayout,
    shader_module: ShaderModule,
) -> RenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[texture_bind_group_layout, camera_bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[ModelVertex::desc(), InstanceRaw::desc()],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: config.format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: texture::Texture::DETH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    })
}

pub fn create_surface_config(
    size: PhysicalSize<u32>,
    capabilities: SurfaceCapabilities,
) -> SurfaceConfiguration {
    let surface_format = capabilities
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(capabilities.formats[0]);

    SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width,
        height: size.height,
        present_mode: capabilities.present_modes[0],
        desired_maximum_frame_latency: 2,
        alpha_mode: capabilities.alpha_modes[0],
        view_formats: vec![],
    }
}

pub async fn create_device(adapter: &Adapter) -> (Device, Queue) {
    adapter
        .request_device(&wgpu::wgt::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: Default::default(),
            trace: wgpu::Trace::Off,
        })
        .await
        .unwrap()
}

pub async fn create_adapter(instance: Instance, surface: &Surface<'static>) -> Adapter {
    instance
        .request_adapter(&wgpu::RequestAdapterOptionsBase {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(surface),
        })
        .await
        .unwrap()
}

pub fn create_gpu_instance() -> Instance {
    Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    })
}
