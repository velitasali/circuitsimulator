//! Straight RGBA8 pixels shared by every canvas drawer.
//!
//! Channels are unassociated: a red pixel with partial alpha stores the red
//! value itself, not red already multiplied by alpha.

/// Row-major RGBA8 image, 4 bytes per pixel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageBuf {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl ImageBuf {
    pub fn filled(width: u32, height: u32, rgba: [u8; 4]) -> Option<Self> {
        let n = byte_len(width, height)?;
        let mut pixels = Vec::with_capacity(n);
        for _ in 0..(n / 4) {
            pixels.extend_from_slice(&rgba);
        }
        Some(Self {
            width,
            height,
            pixels,
        })
    }

    pub fn from_rgba(width: u32, height: u32, pixels: Vec<u8>) -> Option<Self> {
        let n = byte_len(width, height)?;
        if pixels.len() != n {
            return None;
        }
        Some(Self {
            width,
            height,
            pixels,
        })
    }

    /// `src` is premultiplied RGBA.
    pub fn from_premultiplied(width: u32, height: u32, src: &[u8]) -> Option<Self> {
        let n = byte_len(width, height)?;
        if src.len() < n {
            return None;
        }
        let mut pixels = vec![0u8; n];
        for (s, d) in src[..n].chunks_exact(4).zip(pixels.chunks_exact_mut(4)) {
            let a = s[3];
            if a == 0 {
                continue;
            }
            if a == 255 {
                d.copy_from_slice(s);
                continue;
            }
            d[0] = ((u16::from(s[0]) * 255) / u16::from(a)).min(255) as u8;
            d[1] = ((u16::from(s[1]) * 255) / u16::from(a)).min(255) as u8;
            d[2] = ((u16::from(s[2]) * 255) / u16::from(a)).min(255) as u8;
            d[3] = a;
        }
        Some(Self {
            width,
            height,
            pixels,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub fn data(&self) -> &[u8] {
        &self.pixels
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }

    /// Copy `src` onto this image with its top-left at `(x, y)`.
    pub fn copy_at(&mut self, x: u32, y: u32, src: &ImageBuf) {
        if x >= self.width || y >= self.height || src.width == 0 || src.height == 0 {
            return;
        }
        let copy_w = (src.width as usize).min(self.width as usize - x as usize);
        let copy_h = (src.height as usize).min(self.height as usize - y as usize);
        for row in 0..copy_h {
            let dst_i = ((y as usize + row) * self.width as usize + x as usize) * 4;
            let src_i = row * src.width as usize * 4;
            let n = copy_w * 4;
            self.pixels[dst_i..dst_i + n].copy_from_slice(&src.pixels[src_i..src_i + n]);
        }
    }

    pub fn encode_png(&self) -> Option<Vec<u8>> {
        use image::ImageEncoder;
        let mut bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(
                &self.pixels,
                self.width,
                self.height,
                image::ExtendedColorType::Rgba8,
            )
            .ok()?;
        Some(bytes)
    }

    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        let i = self.index(x, y)?;
        Some(self.pixels[i..i + 4].try_into().ok()?)
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        let Some(i) = self.index(x, y) else {
            return;
        };
        self.pixels[i..i + 4].copy_from_slice(&rgba);
    }

    fn index(&self, x: u32, y: u32) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some((y as usize * self.width as usize + x as usize) * 4)
    }
}

fn byte_len(width: u32, height: u32) -> Option<usize> {
    if width == 0 || height == 0 {
        return None;
    }
    (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_premultiplied_restores_straight_channels() {
        let image = ImageBuf::from_premultiplied(1, 2, &[10, 20, 30, 255, 0, 0, 0, 0]).unwrap();
        assert_eq!(image.pixel(0, 0), Some([10, 20, 30, 255]));
        assert_eq!(image.pixel(0, 1), Some([0, 0, 0, 0]));
    }
}
