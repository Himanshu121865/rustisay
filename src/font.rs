use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;

use anyhow::{Context, Result};

#[derive(Clone)]
pub struct Character {
    pub value: char,
    pub width: usize,
    pub height: usize,
    pub intensity: f32,
    pub direction: (f32, f32),
    pub bitmap: Vec<f32>,
}

impl Character {
    pub fn new(value: char, bitmap: &[f32], width: usize, height: usize) -> Self {
        let intensity = bitmap.iter().sum();
        let direction = Self::compute_direction(bitmap, width);
        Self {
            value,
            width,
            height,
            intensity,
            direction,
            bitmap: bitmap.to_vec(),
        }
    }

    fn compute_direction(bitmap: &[f32], width: usize) -> (f32, f32) {
        let grid: Vec<Vec<f32>> = bitmap.chunks(width).map(|r| r.to_vec()).collect();

        let total_kernel: Vec<Vec<f32>> = vec![
            vec![0.0, 0.0, 0.0],
            vec![0.0, 1.0, 1.0],
            vec![1.0, 1.0, 1.0],
        ];
        let total = masked_convolve_sum(&grid, &total_kernel);
        if total == 0.0 {
            return (0.0, 0.0);
        }

        let x_kernel: Vec<Vec<f32>> = vec![
            vec![0.0, 0.0, 0.0],
            vec![0.0, 0.0, 1.0],
            vec![-1.0, 0.0, 1.0],
        ];
        let y_kernel: Vec<Vec<f32>> = vec![
            vec![0.0, 0.0, 0.0],
            vec![0.0, 0.0, 0.0],
            vec![1.0, 1.0, 1.0],
        ];

        (
            masked_convolve_sum(&grid, &x_kernel) / total,
            masked_convolve_sum(&grid, &y_kernel) / total,
        )
    }
}

fn masked_convolve_sum(grid: &[Vec<f32>], kernel: &[Vec<f32>]) -> f32 {
    let h = grid.len() as isize;
    let w = grid[0].len() as isize;
    let krh = ((kernel.len() - 1) / 2) as isize;
    let kch = ((kernel[0].len() - 1) / 2) as isize;
    let mut total = 0.0;
    for r in 0..h {
        for c in 0..w {
            if grid[r as usize][c as usize] == 0.0 {
                continue;
            }
            for (kr, row) in kernel.iter().enumerate() {
                for (kc, &k) in row.iter().enumerate() {
                    let dr = r + kr as isize - krh;
                    let dc = c + kc as isize - kch;
                    if dr >= 0 && dr < h && dc >= 0 && dc < w {
                        total += k * grid[dr as usize][dc as usize];
                    }
                }
            }
        }
    }
    total
}

#[derive(Clone)]
pub struct Font {
    pub width: usize,
    pub height: usize,
    pub chars: Vec<Character>,
}

impl Font {
    #[allow(dead_code)]
    pub fn from_bdf(path: &Path, alphabet: &[char], invert: bool) -> Result<Self> {
        let mut buf = Vec::new();
        File::open(path)
            .with_context(|| format!("failed to open font '{}'", path.display()))?
            .read_to_end(&mut buf)
            .with_context(|| format!("failed to read font '{}'", path.display()))?;
        Self::from_bdf_bytes(&buf, alphabet, invert)
    }

    pub fn from_bdf_bytes(bytes: &[u8], alphabet: &[char], invert: bool) -> Result<Self> {
        let reader = BufReader::new(Cursor::new(bytes));
        let font: bdf_reader::Font =
            bdf_reader::Font::read(reader).context("failed to parse BDF font data")?;

        let on = !invert as u8 as f32;
        let off = 1.0 - on;
        let mut chars: Vec<Character> = font
            .glyphs()
            .into_iter()
            .filter_map(|glyph| {
                let value: char = glyph.encoding() as u8 as char;
                if !alphabet.contains(&value) {
                    return None;
                }
                let bbox = glyph.bounding_box();
                let width = bbox.width as usize;
                let height = bbox.height as usize;
                let glyph_bitmap = glyph.bitmap();
                let mut bitmap = vec![0.0; width * height];
                for y in 0..height {
                    for x in 0..width {
                        bitmap[y * width + x] = if glyph_bitmap.get(x, y).unwrap_or(false) {
                            on
                        } else {
                            off
                        };
                    }
                }
                Some(Character::new(value, &bitmap, width, height))
            })
            .collect();

        chars.sort_by_key(|c| c.value as u8);

        let first = chars
            .first()
            .context("font contains no glyphs matching the alphabet")?;
        let width = first.width;
        let height = first.height;

        Ok(Self {
            width,
            height,
            chars,
        })
    }

    /// Builds a char -> glyph lookup table for fast per-frame rendering.
    pub fn char_lookup(&self) -> std::collections::HashMap<char, &Character> {
        self.chars.iter().map(|c| (c.value, c)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_intensity_is_sum_of_bitmap() {
        let bitmap = vec![1.0, 0.0, 1.0, 1.0];
        let c = Character::new('#', &bitmap, 2, 2);
        assert!((c.intensity - 3.0).abs() < 1e-6);
        assert_eq!(c.bitmap, bitmap);
    }

    #[test]
    fn character_direction_is_zero_for_blank_bitmap() {
        let c = Character::new(' ', &[0.0; 4], 2, 2);
        assert_eq!(c.direction, (0.0, 0.0));
    }

    #[test]
    fn font_from_bdf_is_sorted_and_uniform() {
        let font = Font::from_bdf_bytes(
            include_bytes!("../fonts/bitocra-13.bdf"),
            &['#', 'a'],
            false,
        )
        .unwrap();
        assert_eq!(font.chars.len(), 2);
        assert_eq!(font.chars[0].value, '#');
        assert_eq!(font.chars[1].value, 'a');
        assert_eq!(font.chars[0].width, font.width);
        assert_eq!(font.chars[0].height, font.height);
    }

    #[test]
    fn font_bitmap_marks_glyph_pixels() {
        let font =
            Font::from_bdf_bytes(include_bytes!("../fonts/bitocra-13.bdf"), &['#'], false).unwrap();
        let glyph = &font.chars[0];
        assert_eq!(glyph.bitmap.len(), font.width * font.height);
        assert!(glyph.bitmap.contains(&1.0));
    }

    #[test]
    fn font_rejects_empty_alphabet() {
        assert!(
            Font::from_bdf_bytes(include_bytes!("../fonts/bitocra-13.bdf"), &[], false).is_err()
        );
    }
}
