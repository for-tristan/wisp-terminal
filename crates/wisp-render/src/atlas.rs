use rusttype::gpu_cache::{Cache, CacheWriteErr};
use rusttype::PositionedGlyph;

pub struct GlyphAtlas {
    cache: Cache<'static>,
    texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    width: u32,
    height: u32,
}

pub type CachedGlyphCoords = (rusttype::Rect<f32>, rusttype::Rect<i32>);

fn upload_rect(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    rect: rusttype::Rect<u32>,
    data: &[u8],
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d {
                x: rect.min.x,
                y: rect.min.y,
                z: 0,
            },
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(rect.width()),
            rows_per_image: Some(rect.height()),
        },
        wgpu::Extent3d {
            width: rect.width(),
            height: rect.height(),
            depth_or_array_layers: 1,
        },
    );
}

impl GlyphAtlas {
    pub const WIDTH: u32 = 2048;
    pub const HEIGHT: u32 = 2048;

    pub fn new(device: &wgpu::Device) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("wisp-glyph-atlas"),
            size: wgpu::Extent3d {
                width: Self::WIDTH,
                height: Self::HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let cache = Cache::builder()
            .dimensions(Self::WIDTH, Self::HEIGHT)
            .build();
        Self {
            cache,
            texture,
            view,
            width: Self::WIDTH,
            height: Self::HEIGHT,
        }
    }

    pub fn cache_glyph(
        &mut self,
        queue: &wgpu::Queue,
        glyph: PositionedGlyph<'static>,
        font_id: usize,
    ) -> Option<CachedGlyphCoords> {
        self.cache.queue_glyph(font_id, glyph.clone());

        let mut retry = 0;
        loop {
            let result = {
                let texture = &self.texture;
                self.cache.cache_queued(|rect: rusttype::Rect<u32>, data: &[u8]| {
                    upload_rect(queue, texture, rect, data);
                })
            };
            match result {
                Ok(_) => break,
                Err(CacheWriteErr::NoRoomForWholeQueue) if retry < 1 => {
                    retry += 1;
                    self.cache.clear();
                    self.clear_texture(queue);
                }
                Err(_) => return None,
            }
        }

        let coords = self.cache.rect_for(font_id, &glyph).ok()??;
        Some(coords)
    }

    fn clear_texture(&mut self, queue: &wgpu::Queue) {
        let zeros = vec![0u8; (self.width * self.height) as usize];
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &zeros,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.width),
                rows_per_image: Some(self.height),
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
    }
}
