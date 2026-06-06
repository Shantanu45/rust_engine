use std::sync::Arc;

use anyhow::{Context, Result};
use winit::dpi::PhysicalSize;
use winit::window::Window;

pub(super) struct SurfaceState {
    pub(super) window: Arc<Window>,
    pub(super) surface: wgpu::Surface<'static>,
    size: PhysicalSize<u32>,
    surface_format: wgpu::TextureFormat,
}

impl SurfaceState {
    pub(super) fn new(
        window: Arc<Window>,
        surface: wgpu::Surface<'static>,
        adapter: &wgpu::Adapter,
    ) -> Self {
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

    pub(super) fn configure(&self, device: &wgpu::Device) {
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

    pub(super) fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;
    }

    pub(super) fn recreate(&mut self, instance: &wgpu::Instance) -> Result<()> {
        self.surface = instance
            .create_surface(self.window.clone())
            .context("failed to recreate WGPU surface")?;
        Ok(())
    }

    pub(super) fn window(&self) -> &Window {
        &self.window
    }

    pub(super) fn view_format(&self) -> wgpu::TextureFormat {
        self.surface_format.add_srgb_suffix()
    }
}
