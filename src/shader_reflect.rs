use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Context, Result};
use naga::valid::{Capabilities, ValidationFlags, Validator};
use naga::{AddressSpace, Binding, ImageClass, ImageDimension, ShaderStage, TypeInner};

pub struct ShaderReflection {
    pub entry_points: Vec<EntryPointReflection>,
    pub bind_groups: Vec<BindGroupReflection>,
}

pub struct EntryPointReflection {
    pub name: String,
    pub stage: ShaderStageReflection,
    pub inputs: Vec<ShaderLocationReflection>,
    pub outputs: Vec<ShaderLocationReflection>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShaderStageReflection {
    Vertex,
    Fragment,
    Compute,
}

pub struct ShaderLocationReflection {
    pub location: u32,
    pub name: Option<String>,
}

pub struct BindGroupReflection {
    pub group: u32,
    pub bindings: Vec<BindingReflection>,
}

pub struct BindingReflection {
    pub group: u32,
    pub binding: u32,
    pub name: Option<String>,
    pub visibility: wgpu::ShaderStages,
    pub binding_type: wgpu::BindingType,
}

pub fn reflect_wgsl(label: &str, wgsl_source: &str) -> Result<ShaderReflection> {
    let module = naga::front::wgsl::parse_str(wgsl_source)
        .with_context(|| format!("{label}: failed to parse WGSL"))?;

    Validator::new(ValidationFlags::all(), Capabilities::empty())
        .validate(&module)
        .with_context(|| format!("{label}: WGSL validation failed"))?;

    let mut entry_points = Vec::new();

    for entry in &module.entry_points {
        entry_points.push(reflect_entry_point(&module, entry)?);
    }

    let bind_groups = reflect_bind_groups(label, &module)?;

    Ok(ShaderReflection {
        entry_points,
        bind_groups,
    })
}

impl ShaderReflection {
    pub fn create_pipeline_layout(
        &self,
        device: &wgpu::Device,
        label: &str,
    ) -> wgpu::PipelineLayout {
        let bind_group_layouts = self
            .bind_groups
            .iter()
            .map(|group| {
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
                    label: Some(&format!("{label} Group {}", group.group)),
                    entries: &entries,
                })
            })
            .collect::<Vec<_>>();

        let bind_group_layout_refs = bind_group_layouts
            .iter()
            .map(Some)
            .collect::<Vec<Option<&wgpu::BindGroupLayout>>>();

        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(label),
            bind_group_layouts: &bind_group_layout_refs,
            immediate_size: 0,
        })
    }
}

pub fn validate_vertex_layout(
    shader_name: &str,
    reflection: &ShaderReflection,
    vertex_entry: &str,
    layouts: &[wgpu::VertexBufferLayout<'_>],
) -> Result<()> {
    let entry = reflection
        .entry_points
        .iter()
        .find(|entry| {
            entry.name == vertex_entry && entry.stage == ShaderStageReflection::Vertex
        })
        .with_context(|| format!("{shader_name}: vertex entry point `{vertex_entry}` not found"))?;

    let provided_locations = layouts
        .iter()
        .flat_map(|layout| layout.attributes.iter())
        .map(|attribute| attribute.shader_location)
        .collect::<BTreeSet<_>>();

    for input in &entry.inputs {
        if !provided_locations.contains(&input.location) {
            bail!(
                "{shader_name}: vertex entry `{vertex_entry}` expects @location({}), but no vertex buffer attribute provides it",
                input.location
            );
        }
    }

    Ok(())
}

fn reflect_entry_point(
    module: &naga::Module,
    entry: &naga::EntryPoint,
) -> Result<EntryPointReflection> {
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();

    for arg in &entry.function.arguments {
        collect_locations(module, arg.ty, arg.binding.as_ref(), &arg.name, &mut inputs)?;
    }

    if let Some(result) = &entry.function.result {
        collect_locations(module, result.ty, result.binding.as_ref(), &None, &mut outputs)?;
    }

    Ok(EntryPointReflection {
        name: entry.name.clone(),
        stage: match entry.stage {
            ShaderStage::Vertex => ShaderStageReflection::Vertex,
            ShaderStage::Fragment => ShaderStageReflection::Fragment,
            ShaderStage::Compute => ShaderStageReflection::Compute,
            other => bail!("unsupported shader stage: {other:?}"),
        },
        inputs,
        outputs,
    })
}

fn collect_locations(
    module: &naga::Module,
    ty: naga::Handle<naga::Type>,
    binding: Option<&Binding>,
    name: &Option<String>,
    out: &mut Vec<ShaderLocationReflection>,
) -> Result<()> {
    if let Some(Binding::Location { location, .. }) = binding {
        out.push(ShaderLocationReflection {
            location: *location,
            name: name.clone(),
        });
        return Ok(());
    }

    if let TypeInner::Struct { members, .. } = &module.types[ty].inner {
        for member in members {
            if let Some(Binding::Location { location, .. }) = &member.binding {
                out.push(ShaderLocationReflection {
                    location: *location,
                    name: member.name.clone(),
                });
            }
        }
    }

    Ok(())
}

fn reflect_bind_groups(label: &str, module: &naga::Module) -> Result<Vec<BindGroupReflection>> {
    let mut groups = BTreeMap::<u32, Vec<BindingReflection>>::new();

    for (_, global) in module.global_variables.iter() {
        let Some(resource) = global.binding else {
            continue;
        };

        let binding_type = reflect_binding_type(label, module, global)?;

        groups
            .entry(resource.group)
            .or_default()
            .push(BindingReflection {
                group: resource.group,
                binding: resource.binding,
                name: global.name.clone(),
                visibility: wgpu::ShaderStages::VERTEX
                    | wgpu::ShaderStages::FRAGMENT
                    | wgpu::ShaderStages::COMPUTE,
                binding_type,
            });
    }

    Ok(groups
        .into_iter()
        .map(|(group, mut bindings)| {
            bindings.sort_by_key(|binding| binding.binding);
            BindGroupReflection { group, bindings }
        })
        .collect())
}

fn reflect_binding_type(
    label: &str,
    module: &naga::Module,
    global: &naga::GlobalVariable,
) -> Result<wgpu::BindingType> {
    let ty = &module.types[global.ty];

    match global.space {
        AddressSpace::Uniform => Ok(wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        }),
        AddressSpace::Storage { access } => Ok(wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage {
                read_only: !access.contains(naga::StorageAccess::STORE),
            },
            has_dynamic_offset: false,
            min_binding_size: None,
        }),
        AddressSpace::Handle => match &ty.inner {
            TypeInner::Sampler { comparison } => {
                Ok(wgpu::BindingType::Sampler(if *comparison {
                    wgpu::SamplerBindingType::Comparison
                } else {
                    wgpu::SamplerBindingType::Filtering
                }))
            }
            TypeInner::Image { dim, arrayed, class } => {
                reflect_image_binding(label, *dim, *arrayed, class)
            }
            other => bail!("{label}: unsupported handle binding type: {other:?}"),
        },
        other => bail!("{label}: unsupported resource address space: {other:?}"),
    }
}

fn reflect_image_binding(
    label: &str,
    dim: ImageDimension,
    arrayed: bool,
    class: &ImageClass,
) -> Result<wgpu::BindingType> {
    let view_dimension = match (dim, arrayed) {
        (ImageDimension::D1, false) => wgpu::TextureViewDimension::D1,
        (ImageDimension::D2, false) => wgpu::TextureViewDimension::D2,
        (ImageDimension::D2, true) => wgpu::TextureViewDimension::D2Array,
        (ImageDimension::D3, false) => wgpu::TextureViewDimension::D3,
        (ImageDimension::Cube, false) => wgpu::TextureViewDimension::Cube,
        (ImageDimension::Cube, true) => wgpu::TextureViewDimension::CubeArray,
        _ => bail!("{label}: unsupported texture dimension"),
    };

    match class {
        ImageClass::Sampled { kind, multi } => {
            let sample_type = match kind {
                naga::ScalarKind::Float => wgpu::TextureSampleType::Float { filterable: true },
                naga::ScalarKind::Sint => wgpu::TextureSampleType::Sint,
                naga::ScalarKind::Uint => wgpu::TextureSampleType::Uint,
                other => bail!("{label}: unsupported sampled texture kind: {other:?}"),
            };

            Ok(wgpu::BindingType::Texture {
                sample_type,
                view_dimension,
                multisampled: *multi,
            })
        }
        ImageClass::Depth { multi } => Ok(wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Depth,
            view_dimension,
            multisampled: *multi,
        }),
        other => bail!("{label}: unsupported image class: {other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reflects_triangle_shader() {
        let source = include_str!("shaders/triangle.wgsl");
        let reflection = reflect_wgsl("Triangle Shader", source).unwrap();

        let vertex = reflection
            .entry_points
            .iter()
            .find(|entry| entry.name == "vs_main")
            .unwrap();

        assert_eq!(vertex.stage, ShaderStageReflection::Vertex);
        assert_eq!(
            vertex
                .inputs
                .iter()
                .map(|input| input.location)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert!(reflection.bind_groups.is_empty());
    }

    #[test]
    fn rejects_invalid_wgsl() {
        assert!(reflect_wgsl("Bad Shader", "this is not wgsl").is_err());
    }

    #[test]
    fn reflects_uniform_buffer() {
        let source = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return camera.view_proj * vec4<f32>(position, 1.0);
}
"#;

        let reflection = reflect_wgsl("Uniform Shader", source).unwrap();
        assert_eq!(reflection.bind_groups.len(), 1);
        assert_eq!(reflection.bind_groups[0].group, 0);
        assert_eq!(reflection.bind_groups[0].bindings[0].binding, 0);

        assert!(matches!(
            reflection.bind_groups[0].bindings[0].binding_type,
            wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                ..
            }
        ));
    }
}
