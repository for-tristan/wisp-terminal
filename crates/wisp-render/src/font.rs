use rusttype::{Font, Scale};
use wisp_config::FontConfig;

const BUNDLED_REGULAR: &[u8] = include_bytes!("../../../assets/fonts/IBMPlexMono-Regular.ttf");
const BUNDLED_BOLD: &[u8] = include_bytes!("../../../assets/fonts/IBMPlexMono-Bold.ttf");
const BUNDLED_ITALIC: &[u8] = include_bytes!("../../../assets/fonts/IBMPlexMono-Italic.ttf");
const BUNDLED_BOLD_ITALIC: &[u8] =
    include_bytes!("../../../assets/fonts/IBMPlexMono-BoldItalic.ttf");

pub const FONT_REGULAR: usize = 0;
pub const FONT_BOLD: usize = 1;
pub const FONT_ITALIC: usize = 2;
pub const FONT_BOLD_ITALIC: usize = 3;

#[derive(Debug, Clone, Copy)]
pub struct CellMetrics {
    pub width: f32,
    pub height: f32,
    pub ascent: f32,
    pub descent: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleVariant {
    Regular,
    Bold,
    Italic,
    BoldItalic,
}

impl StyleVariant {
    pub fn font_id(self) -> usize {
        match self {
            StyleVariant::Regular => FONT_REGULAR,
            StyleVariant::Bold => FONT_BOLD,
            StyleVariant::Italic => FONT_ITALIC,
            StyleVariant::BoldItalic => FONT_BOLD_ITALIC,
        }
    }

    pub fn from_flags(bold: bool, italic: bool) -> Self {
        match (bold, italic) {
            (false, false) => StyleVariant::Regular,
            (true, false) => StyleVariant::Bold,
            (false, true) => StyleVariant::Italic,
            (true, true) => StyleVariant::BoldItalic,
        }
    }
}

pub struct FontStack {
    fonts: [Font<'static>; 4],
    pub scale: Scale,
    pub metrics: CellMetrics,
    pub baseline_offset: f32,
}

impl FontStack {
    pub fn load(config: &FontConfig, scale_factor: f32) -> anyhow::Result<Self> {
        let data = load_style_variants(config);
        let parse = |font: Option<Font<'static>>| -> anyhow::Result<Font<'static>> {
            font.ok_or_else(|| anyhow::anyhow!("failed to parse font data"))
        };
        let regular = parse(Font::try_from_bytes(data.0))?;
        let bold = parse(Font::try_from_bytes(data.1))?;
        let italic = parse(Font::try_from_bytes(data.2))?;
        let bold_italic = parse(Font::try_from_bytes(data.3))?;

        let pixel_height = config.size_pt * (96.0 / 72.0) * scale_factor;
        let scale = Scale::uniform(pixel_height);
        let vm = regular.v_metrics(scale);

        let advance = regular
            .glyph('M')
            .scaled(scale)
            .h_metrics()
            .advance_width
            .max(1.0);
        let width = advance.round();

        let base_height = vm.ascent - vm.descent;
        let height = (base_height * config.line_height).round().max(1.0);
        let leading = (height - base_height) * 0.5;

        Ok(Self {
            fonts: [regular, bold, italic, bold_italic],
            scale,
            metrics: CellMetrics {
                width,
                height,
                ascent: vm.ascent,
                descent: vm.descent,
            },
            baseline_offset: leading,
        })
    }

    pub fn font(&self, variant: StyleVariant) -> &Font<'static> {
        &self.fonts[variant.font_id()]
    }
}

type StyleData = (&'static [u8], &'static [u8], &'static [u8], &'static [u8]);

fn load_style_variants(config: &FontConfig) -> StyleData {
    if let Some(data) = load_system_family(&config.family) {
        return data;
    }
    log::info!(
        "system family {:?} unavailable, using bundled IBM Plex Mono",
        config.family
    );
    (
        leak_bytes(BUNDLED_REGULAR),
        leak_bytes(BUNDLED_BOLD),
        leak_bytes(BUNDLED_ITALIC),
        leak_bytes(BUNDLED_BOLD_ITALIC),
    )
}

fn load_system_family(family: &str) -> Option<StyleData> {
    use font_kit::family_name::FamilyName;
    use font_kit::properties::{Properties, Style, Weight};
    use font_kit::source::SystemSource;

    let source = SystemSource::new();
    let mut bold_props = Properties::new();
    bold_props.weight = Weight::BOLD;
    let mut italic_props = Properties::new();
    italic_props.style = Style::Italic;
    let mut bold_italic_props = Properties::new();
    bold_italic_props.weight = Weight::BOLD;
    bold_italic_props.style = Style::Italic;

    let candidates = [FamilyName::Title(family.to_string()), FamilyName::Monospace];
    let regular = load_family_bytes(&source, &candidates, &Properties::new())?;
    let bold = load_family_bytes(&source, &candidates, &bold_props)
        .or_else(|| load_family_bytes(&source, &candidates, &Properties::new()))?;
    let italic = load_family_bytes(&source, &candidates, &italic_props)
        .or_else(|| load_family_bytes(&source, &candidates, &Properties::new()))?;
    let bold_italic = load_family_bytes(&source, &candidates, &bold_italic_props)
        .or_else(|| load_family_bytes(&source, &candidates, &bold_props))?;

    Some((
        leak_owned_bytes(regular),
        leak_owned_bytes(bold),
        leak_owned_bytes(italic),
        leak_owned_bytes(bold_italic),
    ))
}

fn load_family_bytes(
    source: &font_kit::source::SystemSource,
    candidates: &[font_kit::family_name::FamilyName],
    properties: &font_kit::properties::Properties,
) -> Option<Vec<u8>> {
    let handle = source.select_best_match(candidates, properties).ok()?;
    let font = handle.load().ok()?;
    font.copy_font_data().map(|data| data.as_ref().clone())
}

fn leak_bytes(data: &[u8]) -> &'static [u8] {
    Box::leak(data.to_vec().into_boxed_slice())
}

fn leak_owned_bytes(data: Vec<u8>) -> &'static [u8] {
    Box::leak(data.into_boxed_slice())
}
