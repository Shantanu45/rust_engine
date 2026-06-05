use crate::shader_reflect::{reflect_wgsl, validate_vertex_layout, BindGroupReflection};

use super::mesh::{Mesh, Vertex};

pub(super) struct TrianglePass {
    pipeline: wgpu::RenderPipeline,
}

impl TrianglePass {
    pub(super) fn new(device: &wgpu::Device, color_format: wgpu::TextureFormat) -> Self {
        let shader_source = include_str!("../shaders/triangle.wgsl");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Triangle Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let reflection = reflect_wgsl("triangle", shader_source).unwrap();
        validate_vertex_layout("triangle", &reflection, "vs_main", &[Vertex::desc()]).unwrap();

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

        Self { pipeline }
    }

    pub(super) fn draw_mesh<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        mesh: &'a Mesh,
    ) {
        render_pass.set_pipeline(&self.pipeline);
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
