use std::sync::Arc;

use anyhow::Result;
use winit::window::Window;
use wisp_config::{Color as ConfigColor, Theme};
use wisp_core::snapshot::{
    FLAG_BOLD, FLAG_DOUBLE_UNDERLINE, FLAG_HIDDEN, FLAG_ITALIC, FLAG_STRIKEOUT,
    FLAG_UNDERLINE, CursorKind, Snapshot,
};

use crate::atlas::GlyphAtlas;
use crate::font::{FontStack, StyleVariant};
use crate::gpu::GpuContext;
use crate::pipeline::{CellInstance, CellPipeline, KIND_GLYPH, KIND_SOLID, ScreenUniform};

fn rgb(color: [u8; 3]) -> [f32; 4] {
    [
        color[0] as f32 / 255.0,
        color[1] as f32 / 255.0,
        color[2] as f32 / 255.0,
        1.0,
    ]
}

fn rgb_alpha(color: [u8; 3], alpha: f32) -> [f32; 4] {
    let mut rgba = rgb(color);
    rgba[3] = alpha;
    rgba
}

pub struct Renderer {
    gpu: GpuContext,
    background: [u8; 3],
    cursor: [u8; 3],
    cursor_text: [u8; 3],
    pub font_stack: FontStack,
    pub atlas: GlyphAtlas,
    cell_pipeline: CellPipeline,
    cell_size: (f32, f32),
    origin: (f32, f32),
}

impl Renderer {
    pub fn new(window: Arc<Window>, theme: &Theme, font_config: &wisp_config::FontConfig) -> Result<Self> {
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
            background: palette_rgb(&theme.palette, "background"),
            cursor: palette_rgb(&theme.palette, "cursor"),
            cursor_text: palette_rgb(&theme.palette, "cursor_text"),
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

    pub fn set_cursor_colors(&mut self, cursor: [u8; 3], cursor_text: [u8; 3]) {
        self.cursor = cursor;
        self.cursor_text = cursor_text;
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

    pub fn render(&mut self, snapshot: &Snapshot, cursor_visible: bool, focused: bool) -> Result<()> {
        let (cw, ch) = self.cell_size;
        let (ox, oy) = self.origin;
        let background = self.background;
        let cursor_color = self.cursor;
        let cursor_text_color = self.cursor_text;
        let baseline = self.font_stack.baseline_offset + self.font_stack.metrics.ascent;
        let line_thickness = (ch * 0.06).clamp(1.0, 2.5);

        let cursor = if cursor_visible {
            snapshot.cursor.map(|mut cursor| {
                if !focused && cursor.kind != CursorKind::HollowBlock {
                    cursor.kind = CursorKind::HollowBlock;
                }
                cursor
            })
        } else {
            None
        };

        let mut instances: Vec<CellInstance> = Vec::with_capacity(snapshot.cells.len() + 64);
        let mut glyph_instances: Vec<CellInstance> = Vec::with_capacity(snapshot.cells.len());
        let mut solid_count: u32 = 0;

        for cell in &snapshot.cells {
            let x = ox + cell.column as f32 * cw;
            let y = oy + cell.row as f32 * ch;
            let cursor_cell = cursor
                .map(|cursor| cursor.row == cell.row && cursor.column == cell.column)
                .unwrap_or(false);

            let fg = if cursor_cell {
                cursor_text_color
            } else {
                cell.fg
            };

            if cell.bg != background {
                instances.push(CellInstance {
                    pos: [x, y],
                    size: [cw, ch],
                    uv: [0.0, 0.0, 0.0, 0.0],
                    fg: rgb(cell.bg),
                    bg: [0.0, 0.0, 0.0, 0.0],
                    kind: KIND_SOLID,
                });
                solid_count += 1;
            }

            if cell.flags & FLAG_HIDDEN == 0 && cell.character != ' ' {
                let variant = StyleVariant::from_flags(
                    cell.flags & FLAG_BOLD != 0,
                    cell.flags & FLAG_ITALIC != 0,
                );
                if let Some((uv_rect, screen_rect)) = self.ensure_glyph(variant, cell.character) {
                    let pen_x = x;
                    let pen_y = y + baseline;
                    let gx = pen_x + screen_rect.min.x as f32;
                    let gy = pen_y + screen_rect.min.y as f32;
                    let gw = (screen_rect.max.x - screen_rect.min.x) as f32;
                    let gh = (screen_rect.max.y - screen_rect.min.y) as f32;
                    glyph_instances.push(CellInstance {
                        pos: [gx, gy],
                        size: [gw, gh],
                        uv: [
                            uv_rect.min.x,
                            uv_rect.min.y,
                            uv_rect.width(),
                            uv_rect.height(),
                        ],
                        fg: rgb(fg),
                        bg: [0.0, 0.0, 0.0, 0.0],
                        kind: KIND_GLYPH,
                    });
                }
            }

            if cell.flags & FLAG_UNDERLINE != 0 {
                instances.push(CellInstance {
                    pos: [x, y + baseline + 2.0],
                    size: [cw, line_thickness],
                    uv: [0.0, 0.0, 0.0, 0.0],
                    fg: rgb(cell.fg),
                    bg: [0.0, 0.0, 0.0, 0.0],
                    kind: KIND_SOLID,
                });
                solid_count += 1;
            }

            if cell.flags & FLAG_DOUBLE_UNDERLINE != 0 {
                for offset in [2.0, 4.5] {
                    instances.push(CellInstance {
                        pos: [x, y + baseline + offset],
                        size: [cw, line_thickness],
                        uv: [0.0, 0.0, 0.0, 0.0],
                        fg: rgb(cell.fg),
                        bg: [0.0, 0.0, 0.0, 0.0],
                        kind: KIND_SOLID,
                    });
                    solid_count += 1;
                }
            }

            if cell.flags & FLAG_STRIKEOUT != 0 {
                instances.push(CellInstance {
                    pos: [x, y + baseline - ch * 0.28],
                    size: [cw, line_thickness],
                    uv: [0.0, 0.0, 0.0, 0.0],
                    fg: rgb(cell.fg),
                    bg: [0.0, 0.0, 0.0, 0.0],
                    kind: KIND_SOLID,
                });
                solid_count += 1;
            }
        }

        if let Some(cursor) = cursor {
            let cx = ox + cursor.column as f32 * cw;
            let cy = oy + cursor.row as f32 * ch;
            match cursor.kind {
                CursorKind::Block => {
                    instances.push(CellInstance {
                        pos: [cx, cy],
                        size: [cw, ch],
                        uv: [0.0, 0.0, 0.0, 0.0],
                        fg: rgb_alpha(cursor_color, 0.85),
                        bg: [0.0, 0.0, 0.0, 0.0],
                        kind: KIND_SOLID,
                    });
                    solid_count += 1;
                }
                CursorKind::Beam => {
                    instances.push(CellInstance {
                        pos: [cx + 1.0, cy + 1.0],
                        size: [line_thickness, ch - 2.0],
                        uv: [0.0, 0.0, 0.0, 0.0],
                        fg: rgb(cursor_color),
                        bg: [0.0, 0.0, 0.0, 0.0],
                        kind: KIND_SOLID,
                    });
                    solid_count += 1;
                }
                CursorKind::Underline => {
                    instances.push(CellInstance {
                        pos: [cx, cy + ch - line_thickness * 1.5],
                        size: [cw, line_thickness * 1.5],
                        uv: [0.0, 0.0, 0.0, 0.0],
                        fg: rgb(cursor_color),
                        bg: [0.0, 0.0, 0.0, 0.0],
                        kind: KIND_SOLID,
                    });
                    solid_count += 1;
                }
                CursorKind::HollowBlock => {
                    for quad in hollow_block_quads(cx, cy, cw, ch, line_thickness) {
                        instances.push(CellInstance {
                            pos: [quad.0, quad.1],
                            size: [quad.2, quad.3],
                            uv: [0.0, 0.0, 0.0, 0.0],
                            fg: rgb(cursor_color),
                            bg: [0.0, 0.0, 0.0, 0.0],
                            kind: KIND_SOLID,
                        });
                        solid_count += 1;
                    }
                }
            }
        }

        let glyph_count = glyph_instances.len() as u32;
        instances.extend(glyph_instances);

        let (width, height) = self.gpu.size;
        let uniform = ScreenUniform {
            screen_size: [width as f32, height as f32],
            cell_size: [cw, ch],
            origin: [ox, oy],
            pad: [0.0, 0.0],
        };
        self.cell_pipeline.upload_uniform(&self.gpu.queue, &uniform);
        self.cell_pipeline.upload_instances(
            &self.gpu.device,
            &self.gpu.queue,
            &instances,
        );

        let bg = [
            self.background[0] as f64 / 255.0,
            self.background[1] as f64 / 255.0,
            self.background[2] as f64 / 255.0,
            1.0,
        ];

        let pipeline = &self.cell_pipeline.pipeline;
        let bind_group = &self.cell_pipeline.bind_group;
        let instances_buffer = &self.cell_pipeline.instances;

        self.gpu.render(bg, |pass| {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.set_vertex_buffer(0, instances_buffer.slice(..));
            pass.draw(0..6, 0..solid_count);
            pass.draw(0..6, solid_count..solid_count + glyph_count);
        })
    }
}

fn hollow_block_quads(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    t: f32,
) -> [(f32, f32, f32, f32); 4] {
    [
        (x, y, w, t),
        (x, y + h - t, w, t),
        (x, y, t, h),
        (x + w - t, y, t, h),
    ]
}

fn palette_rgb(palette: &wisp_config::Palette, key: &str) -> [u8; 3] {
    palette
        .color(key)
        .map(|color: ConfigColor| color.to_tuple())
        .map(|(r, g, b)| [r, g, b])
        .unwrap_or([255, 0, 255])
}
