use anyhow::{Context, Result, bail};

use crate::shader_reflect::{BindGroupReflection, reflect_wgsl, validate_vertex_layout};

use super::mesh::{Mesh, Vertex};
use super::texture::TextureBinding;

pub(super) struct TrianglePass {
    pipeline: wgpu::RenderPipeline,
    texture: TextureBinding,
    camera_bind_group_layout: wgpu::BindGroupLayout,
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

        let layouts = ReflectedLayouts::new(device, &reflection.bind_groups, "Triangle")?;
        let pipeline_layout = layouts.create_pipeline_layout(device, "Triangle Pipeline Layout");
        let camera_bind_group_layout = layouts
            .bind_group(1, "triangle shader must declare a camera bind group")?
            .clone();
        let texture = TextureBinding::new(
            device,
            queue,
            layouts.bind_group(0, "triangle shader must declare a texture bind group")?,
            texture_data,
            "Triangle",
        )?;

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
            texture,
            camera_bind_group_layout,
        })
    }

    pub(super) fn camera_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.camera_bind_group_layout
    }

    pub(super) fn draw_mesh<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        mesh: &'a Mesh,
        camera_bind_group: &'a wgpu::BindGroup,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, self.texture.bind_group(), &[]);
        render_pass.set_bind_group(1, camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
    }
}

struct ReflectedLayouts {
    bind_groups: Vec<wgpu::BindGroupLayout>,
}

impl ReflectedLayouts {
    fn new(device: &wgpu::Device, groups: &[BindGroupReflection], label: &str) -> Result<Self> {
        let mut bind_groups = Vec::with_capacity(groups.len());

        for (index, group) in groups.iter().enumerate() {
            if group.group as usize != index {
                bail!(
                    "{label}: bind group {} is not contiguous at layout index {index}",
                    group.group
                );
            }

            bind_groups.push(create_bind_group_layout(
                device,
                group,
                &format!("{label} Bind Group Layout {}", group.group),
            ));
        }

        Ok(Self { bind_groups })
    }

    fn bind_group(&self, group: u32, message: &'static str) -> Result<&wgpu::BindGroupLayout> {
        self.bind_groups.get(group as usize).context(message)
    }

    fn create_pipeline_layout(&self, device: &wgpu::Device, label: &str) -> wgpu::PipelineLayout {
        let bind_group_layouts = self
            .bind_groups
            .iter()
            .map(Some)
            .collect::<Vec<Option<&wgpu::BindGroupLayout>>>();

        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(label),
            bind_group_layouts: &bind_group_layouts,
            immediate_size: 0,
        })
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
