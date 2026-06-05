use std::sync::Arc;

use winit::dpi::PhysicalSize;
use winit::window::Window;

use super::gpu::GpuContext;
use super::mesh::Mesh;
use super::pipeline::TrianglePass;
use super::surface::SurfaceState;

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

    pub fn update(&self) {}

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

        let mut encoder =
            self.gpu
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

            self.triangle_pass
                .draw_mesh(&mut render_pass, &self.triangle_mesh);
        }

        self.gpu.queue.submit([encoder.finish()]);
        self.surface.window.pre_present_notify();
        surface_texture.present();
    }
}
