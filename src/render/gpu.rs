use anyhow::{Context, Result};

pub(super) struct GpuContext {
    pub(super) instance: wgpu::Instance,
    pub(super) adapter: wgpu::Adapter,
    pub(super) device: wgpu::Device,
    pub(super) queue: wgpu::Queue,
}

impl GpuContext {
    pub(super) async fn new(instance: wgpu::Instance, surface: &wgpu::Surface<'_>) -> Result<Self> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(surface),
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .context("failed to find a compatible GPU adapter")?;
        Self::log_adapter_info(&adapter.get_info());

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .context("failed to create WGPU device")?;

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
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
