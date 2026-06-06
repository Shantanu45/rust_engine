use anyhow::{Context, Result};

use crate::shader_reflect::{reflect_wgsl, validate_vertex_layout, BindGroupReflection};

use super::mesh::{Mesh, Vertex};
use super::texture::Texture;

pub(super) struct TrianglePass {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    texture: Texture,
}

impl TrianglePass {
    pub(super) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color_format: wgpu::TextureFormat,
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

        let texture = Texture::new(texture_data, queue, device).context("failed to create tree texture")?;
        let bind_group_layout = bind_group_layouts
            .first()
            .context("triangle shader must declare a texture bind group")?;
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Triangle Texture Bind Group"),
            layout: bind_group_layout,
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
            bind_group,
            texture,
        })
    }

    pub(super) fn draw_mesh<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        mesh: &'a Mesh,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
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
