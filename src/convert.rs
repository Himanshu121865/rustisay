use image::DynamicImage;
use image::GenericImageView;
use image::imageops::FilterType;

use crate::font::Font;

fn chunk_direction(chunk: &[f32], width: usize, height: usize) -> (f32, f32) {
    let mut x_grad = 0.0;
    let mut y_grad = 0.0;
    for i in 0..height {
        for j in 0..width - 1 {
            x_grad += chunk[i * width + 1 + j] - chunk[i * width + j];
        }
    }
    for i in 0..height - 1 {
        for j in 0..width {
            y_grad += chunk[(i + 1) * width + j] - chunk[i * width + j];
        }
    }
    (-y_grad, x_grad)
}

fn direction_and_intensity_convert(font: &Font, chunk: &[f32]) -> char {
    let max_direction = (font.width * font.height * 4) as f32;
    let (x_dir, y_dir) = chunk_direction(chunk, font.width, font.height);
    let intensity = chunk.iter().sum::<f32>();

    let mut best_char = font.chars[0].value;
    let mut best_score = f32::NEG_INFINITY;

    for c in &font.chars {
        let grad = -((x_dir - c.direction.0).powi(2) + (y_dir - c.direction.1).powi(2)).sqrt();
        let score = (max_direction - grad) / (1.0 + (intensity - c.intensity).abs());
        if score > best_score {
            best_score = score;
            best_char = c.value;
        }
    }

    best_char
}

fn pixels_to_chunks(
    pixels: &[f32],
    width: usize,
    height: usize,
    chunk_width: usize,
    chunk_height: usize,
) -> Vec<Vec<f32>> {
    let chunk_size = chunk_width * chunk_height;
    let vertical_chunks = height / chunk_height;
    let horizontal_chunks = width / chunk_width;

    let mut chunks: Vec<Vec<f32>> = Vec::with_capacity(vertical_chunks * horizontal_chunks);
    let mut y_offset = 0;
    for _ in 0..vertical_chunks {
        let mut chunk_row: Vec<Vec<f32>> = (0..horizontal_chunks)
            .map(|_| Vec::with_capacity(chunk_size))
            .collect();

        for _ in 0..chunk_height {
            let mut x_offset = 0;
            for chunk in &mut chunk_row {
                let start = y_offset + x_offset;
                let end = start + chunk_width;
                chunk.extend_from_slice(&pixels[start..end]);
                x_offset += chunk_width;
            }
            y_offset += width;
        }

        chunks.extend(chunk_row);
    }

    chunks
}

fn pixels_to_chars(pixels: &[f32], width: usize, height: usize, font: &Font) -> Vec<char> {
    let chunks = pixels_to_chunks(pixels, width, height, font.width, font.height);
    chunks
        .iter()
        .map(|chunk| direction_and_intensity_convert(font, chunk))
        .collect()
}

#[derive(Clone, Copy)]
pub struct ToneAdjust {
    pub invert: bool,
    pub brightness: f32,
    pub contrast: f32,
}

impl Default for ToneAdjust {
    fn default() -> Self {
        Self {
            invert: false,
            brightness: 0.0,
            contrast: 1.0,
        }
    }
}

pub(crate) fn adjust_value(v: f32, adj: &ToneAdjust) -> f32 {
    let mut v = if adj.invert { 1.0 - v } else { v };
    v = (v - 0.5) * adj.contrast + 0.5 + adj.brightness;
    v.clamp(0.0, 1.0)
}

/// Applies tone adjustments and alpha to a source pixel, producing an opaque
/// color ready for a glyph foreground.
pub(crate) fn colorize_pixel(p: &image::Rgba<u8>, adj: &ToneAdjust) -> image::Rgba<u8> {
    let intensity = p[3] as f32 / 255.0;
    image::Rgba([
        (adjust_value(p[0] as f32 / 255.0, adj) * 255.0 * intensity) as u8,
        (adjust_value(p[1] as f32 / 255.0, adj) * 255.0 * intensity) as u8,
        (adjust_value(p[2] as f32 / 255.0, adj) * 255.0 * intensity) as u8,
        255,
    ])
}

fn adjust_luma(pixels: &mut [f32], adj: &ToneAdjust) {
    if adj.invert || adj.brightness != 0.0 || adj.contrast != 1.0 {
        pixels.iter_mut().for_each(|v| *v = adjust_value(*v, adj));
    }
}

fn round_up_to_multiple(x: i32, m: i32) -> i32 {
    if m <= 0 {
        return x;
    }
    let rem = x.rem_euclid(m);
    if rem == 0 { x } else { x + (m - rem) }
}

pub fn img_to_char_rows(
    font: &Font,
    img: &DynamicImage,
    out_width: Option<usize>,
    terminal_dims: Option<(usize, usize)>,
    tone: &ToneAdjust,
) -> Vec<Vec<char>> {
    let (width, height) = (img.width() as usize, img.height() as usize);

    let out_width = if let Some(out_width) = out_width {
        out_width
    } else if let Some((term_w, term_h)) = terminal_dims {
        let by_width = term_w;
        let by_height = ((term_h as f64 * width as f64 * font.height as f64)
            / (height as f64 * font.width as f64))
            .floor() as usize;
        by_width.min(by_height)
    } else {
        round_up_to_multiple(width as i32, font.width as i32) as usize / font.width
    };

    let out_height = (height as f64
        * (out_width as f64 / width as f64)
        * (font.width as f64 / font.height as f64))
        .round() as usize;

    let out_img_width = out_width * font.width;
    let out_img_height = out_height * font.height;

    let luma = img.to_luma32f();
    let luma_pixels = luma.into_raw();
    let (luma_w, luma_h) = (img.width() as usize, img.height() as usize);

    let mut resized_f32 = resize_f32(&luma_pixels, luma_w, luma_h, out_img_width, out_img_height);
    adjust_luma(&mut resized_f32, tone);

    let combined: Vec<f32> = resized_f32.clone();

    let chars = pixels_to_chars(&combined, out_img_width, out_img_height, font);

    chars.chunks(out_width).map(|c| c.to_vec()).collect()
}

fn resize_f32(pixels: &[f32], src_w: usize, src_h: usize, dst_w: usize, dst_h: usize) -> Vec<f32> {
    let mut out = vec![0.0; dst_w * dst_h];
    let x_ratio = src_w as f32 / dst_w as f32;
    let y_ratio = src_h as f32 / dst_h as f32;
    for y in 0..dst_h {
        for x in 0..dst_w {
            let ix = x as f32 * x_ratio;
            let iy = y as f32 * y_ratio;
            let ux = ix.floor() as usize;
            let uy = iy.floor() as usize;
            let dx = ix - ux as f32;
            let dy = iy - uy as f32;
            let tl = pixels[uy * src_w + ux.min(src_w - 1)];
            let tr = pixels[uy * src_w + (ux + 1).min(src_w - 1)];
            let bl = pixels[(uy + 1).min(src_h - 1) * src_w + ux.min(src_w - 1)];
            let br = pixels[(uy + 1).min(src_h - 1) * src_w + (ux + 1).min(src_w - 1)];
            let top = tl + dx * (tr - tl);
            let bot = bl + dx * (br - bl);
            out[y * dst_w + x] = top + dy * (bot - top);
        }
    }
    out
}

pub fn char_rows_to_string(char_rows: &[Vec<char>]) -> String {
    char_rows
        .iter()
        .map(|row| row.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn char_rows_to_terminal_color_string(
    char_rows: &[Vec<char>],
    img: &DynamicImage,
    tone: &ToneAdjust,
) -> String {
    use colored::Colorize;

    if char_rows.is_empty() || char_rows[0].is_empty() {
        return String::new();
    }

    let n_cols = char_rows[0].len();
    let n_rows = char_rows.len();
    let color_img = img.resize_exact(n_cols as u32, n_rows as u32, FilterType::Triangle);

    let mut result = String::new();
    for (j, row) in char_rows.iter().enumerate() {
        for (i, &c) in row.iter().enumerate() {
            let p = color_img.get_pixel(i as u32, j as u32);
            let fg = colorize_pixel(&p, tone);
            result.push_str(&format!("{}", c.to_string().truecolor(fg[0], fg[1], fg[2])));
        }
        if j < n_rows - 1 {
            result.push('\n');
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjust_value_is_identity_by_default() {
        let adj = ToneAdjust::default();
        assert!((adjust_value(0.25, &adj) - 0.25).abs() < 1e-6);
        assert!((adjust_value(0.0, &adj) - 0.0).abs() < 1e-6);
        assert!((adjust_value(1.0, &adj) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn adjust_value_inverts() {
        let adj = ToneAdjust {
            invert: true,
            ..ToneAdjust::default()
        };
        assert!((adjust_value(0.25, &adj) - 0.75).abs() < 1e-6);
    }

    #[test]
    fn adjust_value_applies_brightness_and_contrast() {
        let adj = ToneAdjust {
            brightness: 0.1,
            ..ToneAdjust::default()
        };
        assert!((adjust_value(0.5, &adj) - 0.6).abs() < 1e-6);

        let adj = ToneAdjust {
            contrast: 2.0,
            ..ToneAdjust::default()
        };
        assert!((adjust_value(0.75, &adj) - 1.0).abs() < 1e-6);
        assert!((adjust_value(0.0, &adj) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn adjust_value_clamps() {
        let adj = ToneAdjust {
            brightness: 1.0,
            ..ToneAdjust::default()
        };
        assert_eq!(adjust_value(1.0, &adj), 1.0);
    }

    #[test]
    fn round_up_to_multiple_matches_expectations() {
        assert_eq!(round_up_to_multiple(200, 13), 208);
        assert_eq!(round_up_to_multiple(13, 13), 13);
        assert_eq!(round_up_to_multiple(0, 13), 0);
        assert_eq!(round_up_to_multiple(10, 13), 13);
        assert_eq!(round_up_to_multiple(7, 0), 7);
    }

    #[test]
    fn resize_f32_keeps_uniform_image_uniform() {
        let src = vec![0.5; 4];
        let dst = resize_f32(&src, 2, 2, 4, 4);
        assert_eq!(dst.len(), 16);
        assert!(dst.iter().all(|v| (*v - 0.5).abs() < 1e-6));
    }

    #[test]
    fn resize_f32_picks_top_left_corner_when_shrinking() {
        let src = vec![1.0, 0.0, 0.0, 0.0];
        let dst = resize_f32(&src, 2, 2, 1, 1);
        assert_eq!(dst, vec![1.0]);
    }

    #[test]
    fn char_rows_use_full_grid() {
        // bitocra-13 is 7x13: a 26x26 image yields 4 columns and 2 rows.
        let font = crate::font::Font::from_bdf_bytes(
            include_bytes!("../fonts/bitocra-13.bdf"),
            &['#'],
            false,
        )
        .unwrap();
        let img = DynamicImage::new_rgba8(26, 26);
        let rows = img_to_char_rows(&font, &img, None, None, &ToneAdjust::default());
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.len() == 4));
        assert!(rows.iter().all(|row| row.iter().all(|&c| c == '#')));
    }
}
