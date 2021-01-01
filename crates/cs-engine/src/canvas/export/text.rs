use std::sync::OnceLock;
use tiny_skia::PremultipliedColorU8;

const FONT_TTF: &[u8] = include_bytes!("../../../../../resources/fonts/Ubuntu-R.ttf");
const FONT_BOLD_TTF: &[u8] = include_bytes!("../../../../../resources/fonts/Ubuntu-B.ttf");

pub fn font() -> &'static fontdue::Font {
    static FONT: OnceLock<fontdue::Font> = OnceLock::new();
    FONT.get_or_init(|| {
        fontdue::Font::from_bytes(FONT_TTF, fontdue::FontSettings::default()).expect("Ubuntu-R.ttf")
    })
}

pub fn font_bold() -> &'static fontdue::Font {
    static FONT_BOLD: OnceLock<fontdue::Font> = OnceLock::new();
    FONT_BOLD.get_or_init(|| {
        fontdue::Font::from_bytes(FONT_BOLD_TTF, fontdue::FontSettings::default())
            .expect("Ubuntu-B.ttf")
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub bold: bool,
    pub ch: char,
    pub size_quant: u32,
}

#[derive(Clone)]
pub struct CachedGlyph {
    pub metrics: fontdue::Metrics,
    pub bitmap: std::sync::Arc<[u8]>,
}

thread_local! {
    static GLYPH_CACHE: std::cell::RefCell<rustc_hash::FxHashMap<GlyphKey, CachedGlyph>> =
        std::cell::RefCell::new(rustc_hash::FxHashMap::default());
}
const MAX_GLYPH_CACHE_SIZE: usize = 4096;

pub fn get_cached_glyph(bold: bool, ch: char, raster_px: f32) -> CachedGlyph {
    let size_quant = ((raster_px * 4.0).round() as u32).max(1);
    let key = GlyphKey {
        bold,
        ch,
        size_quant,
    };

    GLYPH_CACHE.with(|cell| {
        let mut map = cell.borrow_mut();
        if let Some(entry) = map.get(&key) {
            return entry.clone();
        }
        if map.len() >= MAX_GLYPH_CACHE_SIZE {
            map.clear();
        }
        let font_ref = if bold { font_bold() } else { font() };
        let actual_px = (size_quant as f32) * 0.25;
        let (metrics, raw_bitmap) = font_ref.rasterize(ch, actual_px);
        let entry = CachedGlyph {
            metrics,
            bitmap: std::sync::Arc::from(raw_bitmap.into_boxed_slice()),
        };
        map.insert(key, entry.clone());
        entry
    })
}

pub fn text_width(s: &str, size: f64, bold: bool) -> f64 {
    let font_ref = if bold { font_bold() } else { font() };
    let mut w = 0.0f32;
    for ch in s.chars() {
        let metrics = font_ref.metrics(ch, size as f32);
        w += metrics.advance_width;
    }
    w as f64
}

pub fn strip_tags(s: &str) -> std::borrow::Cow<'_, str> {
    if !s.contains('<') {
        return std::borrow::Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            out.push(ch);
        }
    }
    std::borrow::Cow::Owned(out)
}

#[inline(always)]
pub fn blend_over(src: PremultipliedColorU8, dst: PremultipliedColorU8) -> PremultipliedColorU8 {
    if src.alpha() == 255 || dst.alpha() == 0 {
        return src;
    }
    if src.alpha() == 0 {
        return dst;
    }
    let inv_a = 255 - src.alpha() as u16;
    let r = (src.red() as u16 + ((dst.red() as u16 * inv_a) / 255)).min(255);
    let g = (src.green() as u16 + ((dst.green() as u16 * inv_a) / 255)).min(255);
    let b = (src.blue() as u16 + ((dst.blue() as u16 * inv_a) / 255)).min(255);
    let a = (src.alpha() as u16 + ((dst.alpha() as u16 * inv_a) / 255)).min(255);
    PremultipliedColorU8::from_rgba(r as u8, g as u8, b as u8, a as u8).unwrap_or(src)
}
