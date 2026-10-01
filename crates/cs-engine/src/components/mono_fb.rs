//! Page-major monochrome framebuffers (SSD1306, SH1107, PCD8544, KS0108).
//!
//! Lit pixels used to be one filled rectangle each. Zoom scales every rectangle,
//! so a 128×64 panel became thousands of antialiased draws per tick. A drawer
//! that can blit receives one nearest-neighbor image. SVG still draws rectangles.

use crate::canvas::ImageBuf;
use crate::canvas::draw::{Color, Draw};

/// Paint the glass, then the page-major framebuffer.
///
/// `hex` is two characters per column byte, bit 0 at the top of the page.
/// When the grid matches the glass rectangle and the target can blit, the
/// pixels are one nearest-neighbor image. Otherwise the glass is a filled
/// rectangle and each lit pixel is its own rectangle (SVG export).
pub(super) fn paint_mono_screen(
    d: &mut dyn Draw,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    cols: usize,
    pages: usize,
    hex: &str,
    fg: Color,
    bg: Color,
    border: Color,
    border_width: f64,
    merge_columns: bool,
) {
    let covers = cols > 0 && pages > 0 && cols as f64 == w && (pages * 8) as f64 == h;
    let blitted = covers && blit_mono_pages(d, x, y, cols, pages, hex, fg, bg);
    if !blitted {
        d.fill_rect(x, y, w, h, bg);
    }
    d.stroke_rect(x, y, w, h, border, border_width);
    if !blitted {
        fill_mono_page_rects(d, x, y, cols, pages, hex, fg, merge_columns);
    }
}

/// Draw `cols` by `pages * 8` pixels as one image. Returns false when the
/// target cannot blit or the grid is empty.
fn blit_mono_pages(
    d: &mut dyn Draw,
    x: f64,
    y: f64,
    cols: usize,
    pages: usize,
    hex: &str,
    fg: Color,
    bg: Color,
) -> bool {
    let Some(image) = mono_page_image(cols, pages, hex, fg, bg) else {
        return false;
    };
    d.blit_pixmap(x, y, &image, 1.0)
}

/// Rectangle fallback. `merge_columns` draws a fully lit byte as one 1×8 rect.
fn fill_mono_page_rects(
    d: &mut dyn Draw,
    x: f64,
    y: f64,
    cols: usize,
    pages: usize,
    hex: &str,
    fg: Color,
    merge_columns: bool,
) {
    if cols == 0 || pages == 0 || hex.is_empty() {
        return;
    }
    let bytes = hex.as_bytes();
    for page in 0..pages {
        let Some(page_offset) = page.checked_mul(cols).and_then(|n| n.checked_mul(2)) else {
            return;
        };
        let py = y + (page * 8) as f64;
        for col in 0..cols {
            let Some(idx) = page_offset.checked_add(col * 2) else {
                return;
            };
            if idx + 2 > bytes.len() {
                return;
            }
            let val = (hex_nibble(bytes[idx]) << 4) | hex_nibble(bytes[idx + 1]);
            if val == 0 {
                continue;
            }
            let px = x + col as f64;
            if merge_columns && val == 0xFF {
                d.fill_rect(px, py, 1.0, 8.0, fg);
            } else {
                for bit in 0..8 {
                    if (val & (1 << bit)) != 0 {
                        d.fill_rect(px, py + bit as f64, 1.0, 1.0, fg);
                    }
                }
            }
        }
    }
}

fn mono_page_image(cols: usize, pages: usize, hex: &str, fg: Color, bg: Color) -> Option<ImageBuf> {
    if cols == 0 || pages == 0 || hex.is_empty() {
        return None;
    }
    let width = u32::try_from(cols).ok()?;
    let height = u32::try_from(pages.checked_mul(8)?).ok()?;
    let mut image = ImageBuf::filled(width, height, [bg.r, bg.g, bg.b, bg.a])?;
    let fg_px = [fg.r, fg.g, fg.b, fg.a];
    let bytes = hex.as_bytes();
    for page in 0..pages {
        let Some(page_offset) = page.checked_mul(cols).and_then(|n| n.checked_mul(2)) else {
            break;
        };
        if page_offset >= bytes.len() {
            break;
        }
        let row0 = page * 8;
        for col in 0..cols {
            let Some(idx) = page_offset.checked_add(col.saturating_mul(2)) else {
                return Some(image);
            };
            if idx + 2 > bytes.len() {
                return Some(image);
            }
            let val = (hex_nibble(bytes[idx]) << 4) | hex_nibble(bytes[idx + 1]);
            if val == 0 {
                continue;
            }
            for bit in 0..8 {
                if (val & (1 << bit)) != 0 {
                    image.set_pixel(col as u32, (row0 + bit) as u32, fg_px);
                }
            }
        }
    }
    Some(image)
}

#[inline]
fn hex_nibble(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::{Canvas, Palette, Point, render_viewport};
    use crate::instruments::ReadingView;

    #[test]
    fn mono_page_pixmap_expands_column_bytes() {
        let fg = (10, 20, 30);
        let bg = (1, 2, 3);
        let image = mono_page_image(
            4,
            1,
            "01FF8000",
            Color::rgb(fg.0, fg.1, fg.2),
            Color::rgb(bg.0, bg.1, bg.2),
        )
        .expect("image");
        assert_eq!((image.width(), image.height()), (4, 8));
        let at = |x, y| {
            let p = image.pixel(x, y).expect("pixel");
            (p[0], p[1], p[2])
        };
        assert_eq!(at(0, 0), fg);
        assert_eq!(at(0, 1), bg);
        assert_eq!(at(1, 0), fg);
        assert_eq!(at(1, 7), fg);
        assert_eq!(at(2, 7), fg);
        assert_eq!(at(2, 0), bg);
        assert_eq!(at(3, 4), bg);
    }

    #[test]
    fn ssd1306_zoomed_pixel_stays_crisp() {
        let mut canvas = Canvas::new();
        canvas
            .scene_mut()
            .add_saved_item(crate::canvas::Item::ssd1306(
                "OLED-1", 0.0, 0.0, 128, 64, 0x3C, "White", true, 100.0,
            ));
        let mut hex = "00".repeat(128 * 8).into_bytes();
        // Page 0, column 10, bit 0: one lit pixel at the top of that column.
        hex[20] = b'0';
        hex[21] = b'1';
        canvas.set_reading(
            "OLED-1",
            ReadingView {
                text: String::from_utf8(hex).unwrap(),
                max_text: String::new(),
                avg_text: String::new(),
                extra: String::new(),
                high: false,
                low: false,
                hz_text: String::new(),
            },
        );

        // Screen origin is local (-64, -42) on a 128×64 panel.
        let lit = Point::new(-64.0 + 10.0 + 0.5, -42.0 + 0.5);
        let dark = Point::new(-64.0 + 11.0 + 0.5, -42.0 + 0.5);
        for zoom in [2.0, 8.0, 20.0] {
            let on = sample(&canvas, lit, zoom);
            let off = sample(&canvas, dark, zoom);
            assert_eq!(on, (245, 245, 245), "lit pixel at zoom {zoom}");
            assert_eq!(off, (0, 0, 0), "unlit pixel at zoom {zoom}");
        }
    }

    fn sample(canvas: &Canvas, scene: Point, zoom: f64) -> (u8, u8, u8) {
        let mut canvas = canvas.clone();
        canvas.set_zoom(zoom);
        canvas.set_center(scene.x, scene.y);
        let pm = render_viewport(&canvas, &Palette::light(), 32, 32, 1.0, false).expect("raster");
        let p = pm.pixel(16, 16).expect("center");
        (p[0], p[1], p[2])
    }
}
