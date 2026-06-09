use std::ops::Index;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    pub(super) fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

pub(super) struct Mesh {
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) vertex_count: u32,
    pub(super) index_buffer: wgpu::Buffer,
    pub(super) index_count: u32,
}

impl Mesh {
    /*pub(super) fn new(device: &wgpu::Device) -> Self {
        const VERTICES: &[Vertex] = &[
            Vertex {
                position: [-0.5, 0.5, 0.0],
                tex_coords: [0.0, 0.0],
            },
            Vertex {
                position: [0.5, 0.5, 0.0],
                tex_coords: [1.0, 0.0],
            },
            Vertex {
                position: [-0.5, -0.5, 0.0],
                tex_coords: [0.0, 1.0],
            },
            Vertex {
                position: [0.5, -0.5, 0.0],
                tex_coords: [1.0, 1.0],
            },
        ];
        const INDICES: &[u16] = &[0, 1, 2, 1, 3, 2];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            vertex_buffer,
            vertex_count: VERTICES.len() as u32,
            index_buffer,
            index_count: INDICES.len() as u32,
        }
    }*/

    pub fn new(device: &wgpu::Device, vertices: &[Vertex], indices: &[u16] ) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            vertex_buffer,
            vertex_count: vertices.len() as u32,
            index_buffer,
            index_count: indices.len() as u32,
        }
    }
}

pub fn mesh_triangle(device: &wgpu::Device) -> Mesh {
    const VERTICES: &[Vertex] = &[
        Vertex {
            position: [0.0, 0.5, 0.0],
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [0.5, -0.25, 0.0],
            tex_coords: [1.0, 0.0],
        },
        Vertex {
            position: [-0.5, -0.25, 0.0],
            tex_coords: [0.0, 1.0],
        },
    ];
    const INDICES: &[u16] = &[0, 1, 2];

    Mesh::new(device, VERTICES, INDICES)
}

pub fn mesh_quad(device: &wgpu::Device) -> Mesh {
    const VERTICES: &[Vertex] = &[
        Vertex {
            position: [-0.5, 0.5, 0.0],
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [0.5, 0.5, 0.0],
            tex_coords: [1.0, 0.0],
        },
        Vertex {
            position: [-0.5, -0.5, 0.0],
            tex_coords: [0.0, 1.0],
        },
        Vertex {
            position: [0.5, -0.5, 0.0],
            tex_coords: [1.0, 1.0],
        },
    ];
    const INDICES: &[u16] = &[0, 1, 2, 1, 3, 2];

    Mesh::new(device, VERTICES, INDICES)
}

pub fn mesh_cube(device: &wgpu::Device) -> Mesh {
    const VERTICES: &[Vertex] = &[
        // Front (+Z)
        Vertex { position: [-0.5, -0.5,  0.5], tex_coords: [0.0, 1.0] },
        Vertex { position: [ 0.5, -0.5,  0.5], tex_coords: [1.0, 1.0] },
        Vertex { position: [ 0.5,  0.5,  0.5], tex_coords: [1.0, 0.0] },
        Vertex { position: [-0.5,  0.5,  0.5], tex_coords: [0.0, 0.0] },

        // Back (-Z)
        Vertex { position: [ 0.5, -0.5, -0.5], tex_coords: [0.0, 1.0] },
        Vertex { position: [-0.5, -0.5, -0.5], tex_coords: [1.0, 1.0] },
        Vertex { position: [-0.5,  0.5, -0.5], tex_coords: [1.0, 0.0] },
        Vertex { position: [ 0.5,  0.5, -0.5], tex_coords: [0.0, 0.0] },

        // Left (-X)
        Vertex { position: [-0.5, -0.5, -0.5], tex_coords: [0.0, 1.0] },
        Vertex { position: [-0.5, -0.5,  0.5], tex_coords: [1.0, 1.0] },
        Vertex { position: [-0.5,  0.5,  0.5], tex_coords: [1.0, 0.0] },
        Vertex { position: [-0.5,  0.5, -0.5], tex_coords: [0.0, 0.0] },

        // Right (+X)
        Vertex { position: [ 0.5, -0.5,  0.5], tex_coords: [0.0, 1.0] },
        Vertex { position: [ 0.5, -0.5, -0.5], tex_coords: [1.0, 1.0] },
        Vertex { position: [ 0.5,  0.5, -0.5], tex_coords: [1.0, 0.0] },
        Vertex { position: [ 0.5,  0.5,  0.5], tex_coords: [0.0, 0.0] },

        // Top (+Y)
        Vertex { position: [-0.5,  0.5,  0.5], tex_coords: [0.0, 1.0] },
        Vertex { position: [ 0.5,  0.5,  0.5], tex_coords: [1.0, 1.0] },
        Vertex { position: [ 0.5,  0.5, -0.5], tex_coords: [1.0, 0.0] },
        Vertex { position: [-0.5,  0.5, -0.5], tex_coords: [0.0, 0.0] },

        // Bottom (-Y)
        Vertex { position: [-0.5, -0.5, -0.5], tex_coords: [0.0, 1.0] },
        Vertex { position: [ 0.5, -0.5, -0.5], tex_coords: [1.0, 1.0] },
        Vertex { position: [ 0.5, -0.5,  0.5], tex_coords: [1.0, 0.0] },
        Vertex { position: [-0.5, -0.5,  0.5], tex_coords: [0.0, 0.0] },
    ];

    const INDICES: &[u16] = &[
        // Front
        0, 1, 2, 0, 2, 3,
        // Back
        4, 5, 6, 4, 6, 7,
        // Left
        8, 9, 10, 8, 10, 11,
        // Right
        12, 13, 14, 12, 14, 15,
        // Top
        16, 17, 18, 16, 18, 19,
        // Bottom
        20, 21, 22, 20, 22, 23,
    ];

    Mesh::new(device, VERTICES, INDICES)
}
