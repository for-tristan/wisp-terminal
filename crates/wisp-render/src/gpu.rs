use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use winit::window::Window;

pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_format: wgpu::TextureFormat,
    pub size: (u32, u32),
}

impl GpuContext {
    pub fn new(window: Arc<Window>) -> Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance
            .create_surface(window.clone())
            .map_err(|error| anyhow!("failed to create surface: {error}"))?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| anyhow!("no suitable gpu adapter found"))?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("wisp-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .context("failed to create gpu device")?;

        let caps = surface.get_capabilities(&adapter);
        let surface_format = caps
            .formats
            .iter()
            .copied()
            .find(|format| !format.is_srgb())
            .unwrap_or(caps.formats[0]);

        let size = window.inner_size();
        let mut context = Self {
            device,
            queue,
            surface,
            surface_format,
            size: (size.width, size.height),
        };
        context.reconfigure();
        Ok(context)
    }

    pub fn reconfigure(&mut self) {
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            width: self.size.0.max(1),
            height: self.size.1.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        self.surface.configure(&self.device, &config);
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        if (width, height) != self.size {
            self.size = (width, height);
            self.reconfigure();
        }
    }

    pub fn render<F>(&mut self, background: [f64; 4], draw: F) -> Result<()>
    where
        F: FnOnce(&mut wgpu::RenderPass<'_>),
    {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated) | Err(wgpu::SurfaceError::Lost) => {
                self.reconfigure();
                self.surface
                    .get_current_texture()
                    .context("failed to acquire swapchain frame after reconfigure")?
            }
            Err(error) => {
                return Err(anyhow::anyhow!(
                    "failed to acquire swapchain frame: {error}"
                ));
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("wisp-frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("wisp-draw"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: background[0],
                            g: background[1],
                            b: background[2],
                            a: background[3],
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            draw(&mut pass);
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }
}
