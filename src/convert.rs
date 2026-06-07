use std::f32::consts::PI;

use image::DynamicImage;
use image::GenericImageView;
use image::imageops::FilterType;

use crate::font::Font;

fn gaussian_kernel(sigma: f32, size: isize) -> Vec<f32> {
    let a = 2.0 * sigma.powi(2);
    let b = 1.0 / (PI * a).sqrt();
    let mut kernel: Vec<f32> = (-size..=size).map(|x| b * (x.pow(2) as f32 / -a).exp()).collect();
    let sum: f32 = kernel.iter().sum();
    kernel.iter_mut().for_each(|v| *v /= sum);
    kernel
}

fn convolve_1d(pixels: &mut [f32], w: usize, h: usize, kernel: &[f32], horizontal: bool) {
    let offset = ((kernel.len() - 1) / 2) as isize;
    let mut out = vec![0.0; pixels.len()];
    if horizontal {
        for y in 0..h {
            for x in 0..w {
                let mut total = 0.0;
                for (kx, &k) in kernel.iter().enumerate() {
                    let sx = x as isize + kx as isize - offset;
                    if sx >= 0 && sx < w as isize {
                        total += pixels[y * w + sx as usize] * k;
                    }
                }
                out[y * w + x] = total;
            }
        }
    } else {
        for y in 0..h {
            for x in 0..w {
                let mut total = 0.0;
                for (ky, &k) in kernel.iter().enumerate() {
                    let sy = y as isize + ky as isize - offset;
                    if sy >= 0 && sy < h as isize {
                        total += pixels[sy as usize * w + x] * k;
                    }
                }
                out[y * w + x] = total;
            }
        }
    }
    pixels.copy_from_slice(&out);
}

fn gaussian_blur(pixels: &mut [f32], w: usize, h: usize, sigma: f32) {
    let size = (sigma * 2.0).ceil() as isize;
    let kernel = gaussian_kernel(sigma, size);
    convolve_1d(pixels, w, h, &kernel, true);
    convolve_1d(pixels, w, h, &kernel, false);
}

fn laplacian(pixels: &[f32], w: usize, h: usize) -> Vec<f32> {
    let mut out = vec![0.0; pixels.len()];
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let i = y * w + x;
            let mut total = 0.0;
            for (ky, row) in [0, -1, 0, -1, 4, -1, 0, -1, 0].chunks(3).enumerate() {
                for (kx, &k) in row.iter().enumerate() {
                    total += pixels[(y + ky - 1) * w + (x + kx - 1)] * k as f32;
                }
            }
            out[i] = total;
        }
    }
    out
}

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
    let mut x_offset = 0;
    for _ in 0..vertical_chunks {
        let mut chunk_row: Vec<Vec<f32>> = (0..horizontal_chunks)
            .map(|_| Vec::with_capacity(chunk_size))
            .collect();

        for _ in 0..chunk_height {
            for x in 0..horizontal_chunks {
                let start = y_offset + x_offset;
                let end = start + chunk_width;
                chunk_row[x].extend_from_slice(&pixels[start..end]);
                x_offset += chunk_width;
            }
            y_offset += width;
            x_offset = 0;
        }

        chunks.extend(chunk_row);
    }

    chunks
}

fn pixels_to_chars(
    pixels: &[f32],
    width: usize,
    height: usize,
    font: &Font,
) -> Vec<char> {
    let chunks = pixels_to_chunks(pixels, width, height, font.width, font.height);
    chunks.iter().map(|chunk| direction_and_intensity_convert(font, chunk)).collect()
}

fn round_up_to_multiple(x: i32, m: i32) -> i32 {
    x + (-x % m)
}

pub fn img_to_char_rows(
    font: &Font,
    img: &DynamicImage,
    out_width: Option<usize>,
) -> Vec<Vec<char>> {
    let (width, height) = (img.width() as usize, img.height() as usize);

    let out_width = if let Some(out_width) = out_width {
        out_width
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

    let resized_f32 = resize_f32(&luma_pixels, luma_w, luma_h, out_img_width, out_img_height);

    let mut edge_pixels = resized_f32.clone();
    gaussian_blur(&mut edge_pixels, out_img_width, out_img_height, 1.0);
    let edge_pixels = laplacian(&edge_pixels, out_img_width, out_img_height);

    let combined: Vec<f32> = resized_f32
        .iter()
        .zip(edge_pixels.iter())
        .map(|(l, e)| l + e)
        .collect();

    let chars = pixels_to_chars(&combined, out_img_width, out_img_height, font);

    chars.chunks(out_width).map(|c| c.to_vec()).collect()
}

fn resize_f32(
    pixels: &[f32],
    src_w: usize,
    src_h: usize,
    dst_w: usize,
    dst_h: usize,
) -> Vec<f32> {
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

pub fn char_rows_to_terminal_color_string(char_rows: &[Vec<char>], img: &DynamicImage) -> String {
    use colored::Colorize;

    let n_cols = char_rows[0].len();
    let n_rows = char_rows.len();
    let color_img = img.resize_exact(n_cols as u32, n_rows as u32, FilterType::Nearest);

    let mut result = String::new();
    for (j, row) in char_rows.iter().enumerate() {
        for (i, &c) in row.iter().enumerate() {
            let p = color_img.get_pixel(i as u32, j as u32);
            let r = p[0];
            let g = p[1];
            let b = p[2];
            let a = p[3];
            let intensity = a as f32 / 255.0;
            result.push_str(
                &format!("{}", c.to_string().truecolor(
                    (r as f32 * intensity) as u8,
                    (g as f32 * intensity) as u8,
                    (b as f32 * intensity) as u8,
                ))
            );
        }
        if j < n_rows - 1 {
            result.push('\n');
        }
    }
    result
}
