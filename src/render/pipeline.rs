use anyhow::{Context, Result};
use wgpu::util::DeviceExt;

use crate::render::camera::Camera;
use crate::shader_reflect::{BindGroupReflection, reflect_wgsl, validate_vertex_layout};

use super::mesh::{Mesh, Vertex};
use super::texture::Texture;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().to_cols_array_2d();
    }
}

pub(super) struct TrianglePass {
    pipeline: wgpu::RenderPipeline,
    texture_bind_group: wgpu::BindGroup,
    camera_bind_group: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,
    camera: Camera,
    _texture: Texture,
}

impl TrianglePass {
    pub(super) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color_format: wgpu::TextureFormat,
        aspect: f32,
    ) -> Result<Self> {
        let shader_source = include_str!("../shaders/triangle.wgsl");
        let texture_data = include_bytes!("../../assets/textures/tree.png");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Triangle Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let reflection = reflect_wgsl("triangle", shader_source)?;
        validate_vertex_layout("triangle", &reflection, "vs_main", &[Vertex::desc()])?;

        let bind_group_layouts = reflection
            .bind_groups
            .iter()
            .map(|group| create_bind_group_layout(device, group, "Triangle Bind Group Layout"))
            .collect::<Vec<_>>();

        let bind_group_layout_refs = bind_group_layouts
            .iter()
            .map(Some)
            .collect::<Vec<Option<&wgpu::BindGroupLayout>>>();

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Triangle Pipeline Layout"),
            bind_group_layouts: &bind_group_layout_refs,
            immediate_size: 0,
        });

        //let pipeline_layout = reflection.create_pipeline_layout(device, "triangle");

        let texture =
            Texture::new(texture_data, queue, device).context("failed to create tree texture")?;
        let texture_bind_group_layout = bind_group_layouts
            .first()
            .context("triangle shader must declare a texture bind group")?;
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Triangle Texture Bind Group"),
            layout: texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        });

        let camera = Camera {
            eye: glam::Vec3::new(0.0, 1.0, 2.0),
            target: glam::Vec3::ZERO,
            up: glam::Vec3::Y,
            aspect,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout = bind_group_layouts
            .get(1)
            .context("triangle shader must declare a camera bind group")?;
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Triangle Camera Bind Group"),
            layout: camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Triangle Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Ok(Self {
            pipeline,
            texture_bind_group,
            camera_bind_group,
            camera_buffer,
            camera,
            _texture: texture,
        })
    }

    pub(super) fn resize(&mut self, queue: &wgpu::Queue, aspect: f32) {
        self.camera.aspect = aspect;

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&self.camera);
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );
    }

    pub(super) fn draw_mesh<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, mesh: &'a Mesh) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
        render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
    }
}

fn create_bind_group_layout(
    device: &wgpu::Device,
    group: &BindGroupReflection,
    label: &str,
) -> wgpu::BindGroupLayout {
    let entries = group
        .bindings
        .iter()
        .map(|binding| wgpu::BindGroupLayoutEntry {
            binding: binding.binding,
            visibility: binding.visibility,
            ty: binding.binding_type,
            count: None,
        })
        .collect::<Vec<_>>();

    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
        entries: &entries,
    })
}
