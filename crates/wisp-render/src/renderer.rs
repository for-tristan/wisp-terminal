use std::sync::Arc;

use anyhow::Result;
use winit::window::Window;
use wisp_config::FontConfig;

use crate::atlas::GlyphAtlas;
use crate::font::{FontStack, StyleVariant};
use crate::gpu::GpuContext;
use crate::pipeline::{CellPipeline, ScreenUniform};

pub struct Renderer {
    gpu: GpuContext,
    background: [u8; 3],
    pub font_stack: FontStack,
    pub atlas: GlyphAtlas,
    cell_pipeline: CellPipeline,
    cell_size: (f32, f32),
    origin: (f32, f32),
}

impl Renderer {
    pub fn new(window: Arc<Window>, background: [u8; 3], font_config: &FontConfig) -> Result<Self> {
        let gpu = GpuContext::new(window)?;
        let font_stack = FontStack::load(font_config)?;
        let atlas = GlyphAtlas::new(&gpu.device);

        let atlas_sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("wisp-atlas-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let cell_pipeline = CellPipeline::new(
            &gpu.device,
            gpu.surface_format,
            &atlas.view,
            &atlas_sampler,
        );

        let cell_size = (
            font_stack.metrics.width.max(1.0),
            font_stack.metrics.height.max(1.0),
        );

        Ok(Self {
            gpu,
            background,
            font_stack,
            atlas,
            cell_pipeline,
            cell_size,
            origin: (0.0, 0.0),
        })
    }

    pub fn metrics(&self) -> (f32, f32) {
        (self.font_stack.metrics.width, self.font_stack.metrics.height)
    }

    pub fn ensure_glyph(
        &mut self,
        variant: StyleVariant,
        ch: char,
    ) -> Option<crate::atlas::CachedGlyphCoords> {
        let font = self.font_stack.font(variant);
        let glyph = font
            .glyph(ch)
            .scaled(self.font_stack.scale)
            .positioned(rusttype::point(0.0, 0.0));
        self.atlas
            .cache_glyph(&self.gpu.queue, glyph, variant.font_id())
    }

    pub fn set_background(&mut self, background: [u8; 3]) {
        self.background = background;
    }

    pub fn set_cell_size(&mut self, width: f32, height: f32) {
        self.cell_size = (width.max(1.0), height.max(1.0));
    }

    pub fn screen_size(&self) -> (f32, f32) {
        (self.gpu.size.0 as f32, self.gpu.size.1 as f32)
    }

    pub fn set_origin(&mut self, x: f32, y: f32) {
        self.origin = (x, y);
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu.resize(width, height);
    }

    pub fn render(&mut self, counts: (u32, u32)) -> Result<()> {
        let (width, height) = self.gpu.size;
        let uniform = ScreenUniform {
            screen_size: [width as f32, height as f32],
            cell_size: [self.cell_size.0, self.cell_size.1],
            origin: [self.origin.0, self.origin.1],
            pad: [0.0, 0.0],
        };
        self.cell_pipeline.upload_uniform(&self.gpu.queue, &uniform);
        self.cell_pipeline
            .upload_instances(&self.gpu.device, &self.gpu.queue, &[]);

        let bg = [
            self.background[0] as f64 / 255.0,
            self.background[1] as f64 / 255.0,
            self.background[2] as f64 / 255.0,
            1.0,
        ];

        let pipeline = &self.cell_pipeline.pipeline;
        let bind_group = &self.cell_pipeline.bind_group;
        let instances = &self.cell_pipeline.instances;

        self.gpu.render(bg, |pass| {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.set_vertex_buffer(0, instances.slice(..));
            pass.draw(0..6, 0..counts.0);
            pass.draw(0..6, counts.0..counts.0 + counts.1);
        })
    }
}
