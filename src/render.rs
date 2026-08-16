use std::collections::HashMap;

use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};

use crate::convert::ToneAdjust;
use crate::font::{Character, Font};

/// Renders an ASCII art grid into pixels using the font's glyph bitmaps.
/// Characters are drawn on a solid background color. With color enabled each
/// character is tinted with the color of the source image at that cell;
/// otherwise characters are white.
pub fn render_char_grid(
    font: &Font,
    glyphs: &HashMap<char, &Character>,
    char_rows: &[Vec<char>],
    img: &DynamicImage,
    tone: &ToneAdjust,
    no_color: bool,
    bg: Rgba<u8>,
) -> RgbaImage {
    let n_cols = char_rows[0].len() as u32;
    let n_rows = char_rows.len() as u32;
    let mut canvas =
        RgbaImage::from_pixel(n_cols * font.width as u32, n_rows * font.height as u32, bg);

    let color_img = if no_color {
        None
    } else {
        Some(img.resize_exact(n_cols, n_rows, FilterType::Triangle))
    };

    for (row, row_chars) in char_rows.iter().enumerate() {
        for (col, &c) in row_chars.iter().enumerate() {
            let Some(glyph) = glyphs.get(&c) else {
                continue;
            };
            let fg = if let Some(color_img) = &color_img {
                crate::convert::colorize_pixel(&color_img.get_pixel(col as u32, row as u32), tone)
            } else {
                Rgba([255, 255, 255, 255])
            };

            let base_x = col * font.width;
            let base_y = row * font.height;
            for gy in 0..glyph.height {
                for gx in 0..glyph.width {
                    if glyph.bitmap[gy * glyph.width + gx] >= 0.5 {
                        canvas.put_pixel((base_x + gx) as u32, (base_y + gy) as u32, fg);
                    }
                }
            }
        }
    }

    canvas
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::ToneAdjust;
    use image::{Rgba, RgbaImage};

    fn test_font() -> Font {
        let blank = Character::new(' ', &[0.0; 4], 2, 2);
        let filled = Character::new('#', &[1.0; 4], 2, 2);
        Font {
            width: 2,
            height: 2,
            chars: vec![blank, filled],
        }
    }

    #[test]
    fn renders_white_glyphs_on_black_without_color() {
        let font = test_font();
        let char_rows = vec![vec!['#', ' '], vec![' ', '#']];
        let img = RgbaImage::new(2, 2);
        let art = render_char_grid(
            &font,
            &font.char_lookup(),
            &char_rows,
            &DynamicImage::ImageRgba8(img),
            &ToneAdjust::default(),
            true,
            Rgba([0, 0, 0, 255]),
        );

        assert_eq!(art.dimensions(), (4, 4));
        assert_eq!(art.get_pixel(0, 0), &Rgba([255, 255, 255, 255]));
        assert_eq!(art.get_pixel(1, 1), &Rgba([255, 255, 255, 255]));
        assert_eq!(art.get_pixel(2, 0), &Rgba([0, 0, 0, 255]));
        assert_eq!(art.get_pixel(0, 2), &Rgba([0, 0, 0, 255]));
    }

    #[test]
    fn renders_on_custom_background_color() {
        let font = test_font();
        let char_rows = vec![vec![' ']];
        let img = RgbaImage::new(1, 1);
        let art = render_char_grid(
            &font,
            &font.char_lookup(),
            &char_rows,
            &DynamicImage::ImageRgba8(img),
            &ToneAdjust::default(),
            true,
            Rgba([10, 20, 30, 255]),
        );
        assert!(art.pixels().all(|p| p == &Rgba([10, 20, 30, 255])));
    }

    #[test]
    fn renders_tinted_glyphs_with_color() {
        let font = test_font();
        let char_rows = vec![vec!['#']];
        let img = RgbaImage::from_pixel(1, 1, Rgba([255, 0, 0, 255]));
        let art = render_char_grid(
            &font,
            &font.char_lookup(),
            &char_rows,
            &DynamicImage::ImageRgba8(img),
            &ToneAdjust::default(),
            false,
            Rgba([0, 0, 0, 255]),
        );

        assert_eq!(art.get_pixel(0, 0), &Rgba([255, 0, 0, 255]));
        assert_eq!(art.get_pixel(1, 1), &Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn unknown_characters_are_skipped() {
        let font = test_font();
        let char_rows = vec![vec!['?']];
        let img = RgbaImage::new(1, 1);
        let art = render_char_grid(
            &font,
            &font.char_lookup(),
            &char_rows,
            &DynamicImage::ImageRgba8(img),
            &ToneAdjust::default(),
            true,
            Rgba([0, 0, 0, 255]),
        );
        assert!(art.pixels().all(|p| p == &Rgba([0, 0, 0, 255])));
    }
}
