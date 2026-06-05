use std::sync::Arc;
use winit::dpi::PhysicalSize;
use winit::window::Window;
use wgpu::util::DeviceExt;

pub struct Renderer {
    gpu: GpuContext,
    surface: SurfaceState,
    triangle_pass: TrianglePass,
    triangle_mesh: Mesh,
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            flags: wgpu::InstanceFlags::empty(),
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });

        let surface = instance.create_surface(window.clone()).unwrap();
        let gpu = GpuContext::new(instance, &surface).await;

        let surface = SurfaceState::new(window, surface, &gpu.adapter);

        surface.configure(&gpu.device);

        let triangle_pass = TrianglePass::new(&gpu.device, surface.view_format());
        let triangle_mesh = Mesh::new(&gpu.device);
        Self {
            gpu,
            surface,
            triangle_pass,
            triangle_mesh,
        }
    }

    pub fn window(&self) -> &Window {
        self.surface.window()
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.surface.resize(new_size);
        self.surface.configure(&self.gpu.device);
    }

    pub fn update(&self){
    }
    pub fn render(&mut self) {
        let surface_texture = match self.surface.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => return,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                drop(texture);
                self.surface.configure(&self.gpu.device);
                return;
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.gpu.device);
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                unreachable!("No error scope registered, so validation errors will panic")
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.recreate(&self.gpu.instance);
                self.surface.configure(&self.gpu.device);
                return;
            }
        };

        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.surface.view_format()),
                ..Default::default()
            });

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Triangle Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            //self.triangle_pass.draw(&mut render_pass);
            self.triangle_pass.draw_mesh(&mut render_pass, &self.triangle_mesh);
        }

        self.gpu.queue.submit([encoder.finish()]);
        self.surface.window.pre_present_notify();
        surface_texture.present();
    }
}

struct GpuContext {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl GpuContext {
    async fn new(instance: wgpu::Instance, surface: &wgpu::Surface<'_>) -> Self {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(surface),
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .unwrap();
        Self::log_adapter_info(&adapter.get_info());

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();

        Self {
            instance,
            adapter,
            device,
            queue,
        }
    }

    fn log_adapter_info(info: &wgpu::AdapterInfo) {
        let device_type = match info.device_type {
            wgpu::DeviceType::DiscreteGpu => "discrete GPU",
            wgpu::DeviceType::IntegratedGpu => "integrated GPU",
            wgpu::DeviceType::VirtualGpu => "virtual GPU",
            wgpu::DeviceType::Cpu => "CPU",
            wgpu::DeviceType::Other => "other",
        };

        tracing::info!(
            name = %info.name,
            device_type = device_type,
            driver = %info.driver,
            "selected GPU adapter"
        );
    }
}

struct SurfaceState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    size: winit::dpi::PhysicalSize<u32>,
    surface_format: wgpu::TextureFormat,
}

impl SurfaceState {
    fn new(window: Arc<Window>, surface: wgpu::Surface<'static>, adapter: &wgpu::Adapter) -> Self {
        let size = window.inner_size();

        let capabilities = surface.get_capabilities(adapter);

        let surface_format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .unwrap_or(capabilities.formats[0]);

        Self {
            window,
            surface,
            size,
            surface_format,
        }
    }

    fn configure(&self, device: &wgpu::Device) {
        if self.size.width == 0 || self.size.height == 0 {
            return;
        }

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            view_formats: vec![self.view_format()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };

        self.surface.configure(device, &config);
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;
    }

    fn recreate(&mut self, instance: &wgpu::Instance) {
        self.surface = instance.create_surface(self.window.clone()).unwrap();
    }

    fn window(&self) -> &Window {
        &self.window
    }

    fn view_format(&self) -> wgpu::TextureFormat {
        self.surface_format.add_srgb_suffix()
    }
}

struct TrianglePass {
    pipeline: wgpu::RenderPipeline,
}

impl TrianglePass {
    fn new(device: &wgpu::Device, color_format: wgpu::TextureFormat) -> Self {
        let shader_source = include_str!("shaders/triangle.wgsl");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Triangle Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Triangle Pipeline"),
            layout: None,
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

    fn draw(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.draw(0..3, 0..1);
    }

    fn draw_mesh<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, mesh: &'a Mesh)
    {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        //render_pass.draw(0..mesh.vertex_count, 0..1);

        render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16); // 1.
        render_pass.draw_indexed(0..mesh.index_count, 0, 0..1); // 2
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
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
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}
struct Mesh {
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
    index_buffer: wgpu::Buffer,
    index_count: u32,

}

impl Mesh{
    fn new(device: &wgpu::Device) -> Self{
        const VERTICES: &[Vertex] = &[
            Vertex { position: [-0.5, 0.5, 0.0], color: [1.0, 0.0, 0.0] },
            Vertex { position: [0.5, 0.5, 0.0], color: [1.0, 1.0, 0.0] },
            Vertex { position: [-0.5, -0.5, 0.0], color: [0.0, 1.0, 0.0] },
            Vertex { position: [0.5, -0.5, 0.0], color: [0.0, 0.0, 1.0] },
        ];
        const INDICES: &[u16] = &[
            0,1,2,
            1,3,2
        ];
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),// cast_slice: reinterprets a slice of one type as a slice of another type without copying the data. Example: u32 → u8
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsages::INDEX,
            }
        );
        Self{
            vertex_buffer,
            vertex_count: VERTICES.len() as u32,
            index_buffer,
            index_count: INDICES.len() as u32,
        }
    }
}
