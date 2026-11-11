use std::sync::Arc;

use anyhow::Result;
use winit::window::Window;

use crate::gpu::GpuContext;

pub struct Renderer {
    gpu: GpuContext,
    background: [u8; 3],
}

impl Renderer {
    pub fn new(window: Arc<Window>, background: [u8; 3]) -> Result<Self> {
        let gpu = GpuContext::new(window)?;
        Ok(Self { gpu, background })
    }

    pub fn set_background(&mut self, background: [u8; 3]) {
        self.background = background;
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu.resize(width, height);
    }

    pub fn render(&mut self) -> Result<()> {
        let bg = [
            self.background[0] as f64 / 255.0,
            self.background[1] as f64 / 255.0,
            self.background[2] as f64 / 255.0,
            1.0,
        ];
        self.gpu.render(bg)
    }
}
