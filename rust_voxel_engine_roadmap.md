# Rust Voxel Engine Roadmap

## Overview

This document outlines a practical roadmap for building a Minecraft-style voxel engine in Rust using WGPU.

### Recommended Stack

```toml
wgpu = "29"
winit = "0.30"
pollster = "0.4"

glam = "0.33"
bytemuck = "1.25"

anyhow = "1"

tracing = "0.1"
tracing-subscriber = "0.3"
```

Optional later:

```toml
noise
rayon
crossbeam-channel
egui
hecs
shipyard
bevy_ecs
```

---

# Phase 1: Basic Renderer

Goal: Open a window and draw a cube.

Concepts:

- WGPU Instance
- Surface
- Adapter
- Device
- Queue
- Swapchain / Surface Configuration
- Render Pipeline
- Depth Buffer
- Vertex Buffer
- Index Buffer

Suggested structure:

```text
src/
  main.rs

  engine/
    mod.rs
    app.rs

  render/
    mod.rs
    renderer.rs
    camera.rs
    texture.rs

  shaders/
    voxel.wgsl
```

Milestone:

```text
Draw a single cube.
```

---

# Phase 2: Camera System

Goal: Minecraft-style camera.

Features:

- WASD movement
- Mouse look
- Delta time
- Perspective projection
- View matrix

Camera:

```rust
pub struct Camera {
    pub position: glam::Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}
```

Camera Uniform:

```rust
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}
```

Milestone:

```text
Fly around a cube world.
```

---

# Phase 3: Voxel Data Model

Never render one cube per block.

Use chunks.

Recommended chunk size:

```text
16 x 16 x 16
```

Block definitions:

```rust
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Block {
    Air,
    Dirt,
    Grass,
    Stone,
}
```

Chunk:

```rust
pub struct Chunk {
    blocks: Vec<Block>,
}
```

Indexing:

```rust
fn index(x: usize, y: usize, z: usize) -> usize {
    x + CHUNK_SIZE * (z + CHUNK_SIZE * y)
}
```

Milestone:

```text
Store voxel data in memory.
```

---

# Phase 4: Naive Chunk Meshing

Generate only visible faces.

Algorithm:

```text
For every block:
    If not air:
        Check 6 neighbors
        If neighbor is air:
            Emit face
```

Vertex:

```rust
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VoxelVertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    block_id: u32,
}
```

Milestone:

```text
Render a chunk.
```

---

# Phase 5: Multiple Chunks

Chunk position:

```rust
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct ChunkPos {
    x: i32,
    y: i32,
    z: i32,
}
```

World:

```rust
use std::collections::HashMap;

pub struct World {
    chunks: HashMap<ChunkPos, Chunk>,
}
```

GPU mesh:

```rust
pub struct ChunkMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}
```

Milestone:

```text
Render multiple chunks.
```

---

# Phase 6: Terrain Generation

Start simple.

Example:

```rust
let height =
    8 + ((x as f32 * 0.2).sin() * 4.0) as i32;
```

Then move to:

```toml
noise
```

Terrain:

```text
Stone
Dirt
Grass
Air
```

Milestone:

```text
Generate hills.
```

---

# Phase 7: Greedy Meshing

Problem:

Naive meshing generates too many triangles.

Greedy meshing merges adjacent quads.

Example:

```text
Before:

[] [] [] []

After:

[          ]
```

Benefits:

- Less geometry
- Better performance
- Lower memory usage

Milestone:

```text
Large reduction in triangle count.
```

---

# Phase 8: Texture Atlas

Single texture containing all block textures.

```text
atlas.png

grass_top
grass_side
dirt
stone
sand
```

Block descriptor:

```rust
pub struct BlockDescriptor {
    pub top_texture: u32,
    pub side_texture: u32,
    pub bottom_texture: u32,
    pub solid: bool,
}
```

Milestone:

```text
Textured terrain.
```

---

# Phase 9: Block Interaction

Raycasting from camera.

Actions:

```text
Left click  -> Remove block
Right click -> Place block
```

After editing:

```text
Mark chunk dirty
Regenerate mesh
Upload mesh to GPU
```

Milestone:

```text
Editable world.
```

---

# Phase 10: Chunk Loading

Only keep nearby chunks loaded.

```rust
pub const RENDER_DISTANCE: i32 = 8;
```

States:

```rust
pub enum ChunkState {
    Empty,
    Generated,
    Meshed,
    Uploaded,
}
```

Workflow:

```text
Player moves
Determine visible chunks
Generate missing chunks
Unload distant chunks
```

Milestone:

```text
Infinite world.
```

---

# Phase 11: Multithreading

Main thread:

```text
Input
Rendering
GPU uploads
```

Worker threads:

```text
Terrain generation
Chunk meshing
```

Useful crates:

```toml
rayon
crossbeam-channel
```

Milestone:

```text
Smooth chunk streaming.
```

---

# Phase 12: Lighting

Start simple.

Face lighting:

```text
Top    = Bright
Sides  = Medium
Bottom = Dark
```

Then:

- Ambient Occlusion
- Sunlight
- Block lights
- Light propagation

Milestone:

```text
Good visual depth.
```

---

# Phase 13: Transparency

Examples:

```text
Water
Glass
Leaves
```

Use separate meshes:

```text
Solid mesh
Transparent mesh
```

Render order:

```text
Solid first
Transparent second
```

Milestone:

```text
Basic transparency support.
```

---

# Phase 14: Save / Load

World structure:

```text
world/

  chunks/
    0_0_0.chunk
    0_0_1.chunk
```

Store:

- Chunk position
- Block data
- Version number

Later:

```text
lz4
zstd
RLE
```

Milestone:

```text
Persistent worlds.
```

---

# Phase 15: Engine Architecture

Suggested layout:

```text
src/

  app/

  render/
    renderer.rs
    camera.rs
    pipeline.rs
    texture_atlas.rs

  voxel/
    block.rs
    mesh.rs
    mesher.rs

  world/
    chunk.rs
    chunk_pos.rs
    world.rs
    terrain.rs

  assets/
  input/
  physics/
  utils/
```

---

# Future Optimizations

## Frustum Culling

Only render chunks visible to the camera.

## Occlusion Culling

Skip chunks hidden behind terrain.

## LOD

Lower detail meshes for distant chunks.

## GPU Driven Rendering

Use compute shaders for visibility and draw submission.

## Sparse Voxel Structures

Advanced techniques:

- Sparse Voxel Octrees
- Sparse Voxel DAGs

---

# Learning Roadmap

Build in this order:

1. Window
2. Clear Screen
3. Triangle
4. Cube
5. Camera
6. Single Chunk
7. Hidden Face Meshing
8. Multiple Chunks
9. Terrain Generation
10. Texture Atlas
11. Block Placement
12. Greedy Meshing
13. Chunk Streaming
14. Multithreaded Meshing
15. Lighting
16. Saving / Loading
17. Transparency

---

# Most Important First Goal

Do not start with:

- Infinite worlds
- ECS
- Physics
- Multiplayer
- Greedy meshing

Start with:

```text
Render one 16x16x16 chunk using hidden-face meshing.
```

Once that works, you have the foundation of a voxel engine.
