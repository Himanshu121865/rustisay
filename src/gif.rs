use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use gif::{DisposalMethod, Encoder, Frame, Repeat};
use image::RgbaImage;

const MAX_COLORS: usize = 255;

struct Quantized {
    palette: Vec<u8>,
    lookup: Vec<u8>,
}

fn bucket(r: u8, g: u8, b: u8) -> usize {
    ((r >> 3) as usize) << 10 | ((g >> 3) as usize) << 5 | (b >> 3) as usize
}

fn bucket_center(key: usize) -> [u8; 3] {
    [
        ((key >> 10) << 3 | (1 << 2)) as u8,
        (((key >> 5) & 31) << 3 | (1 << 2)) as u8,
        ((key & 31) << 3 | (1 << 2)) as u8,
    ]
}

fn quantize(frames: &[(RgbaImage, Duration)]) -> Quantized {
    let sample_total: usize = frames.iter().map(|(f, _)| f.len() / 4).sum();
    let stride = (sample_total / 1_000_000).max(1);
    let mut hist: HashMap<usize, u32> = HashMap::new();

    let mut sample = 0usize;
    for (frame, _) in frames {
        for pixel in frame.pixels() {
            if pixel[3] >= 128 && sample.is_multiple_of(stride) {
                *hist
                    .entry(bucket(pixel[0], pixel[1], pixel[2]))
                    .or_insert(0) += 1;
            }
            sample += 1;
        }
    }

    let mut entries: Vec<([u8; 3], u32)> = hist
        .into_iter()
        .map(|(key, count)| (bucket_center(key), count))
        .collect();
    entries.sort_by_key(|(color, _)| {
        color[0] as u32 * 65536 + color[1] as u32 * 256 + color[2] as u32
    });

    let mut colors = median_cut(&entries, MAX_COLORS);
    colors.sort_by_key(|c| c[0] as u32 * 65536 + c[1] as u32 * 256 + c[2] as u32);

    let mut palette: Vec<u8> = Vec::with_capacity((colors.len() + 1) * 3);
    palette.extend_from_slice(&[0, 0, 0]);
    for color in &colors {
        palette.extend_from_slice(color);
    }
    if palette.len() == 3 {
        palette.extend_from_slice(&[0, 0, 0]);
    }

    let mut lookup = vec![0u8; 32 * 32 * 32];
    if palette.len() > 3 {
        for (key, slot) in lookup.iter_mut().enumerate() {
            let c = bucket_center(key);
            *slot = nearest(&palette, c);
        }
    }

    Quantized { palette, lookup }
}

fn nearest(palette: &[u8], c: [u8; 3]) -> u8 {
    let mut best = 1u8;
    let mut best_dist = u32::MAX;
    for (i, color) in palette.chunks_exact(3).enumerate().skip(1) {
        let dr = color[0] as i32 - c[0] as i32;
        let dg = color[1] as i32 - c[1] as i32;
        let db = color[2] as i32 - c[2] as i32;
        let dist = (dr * dr + dg * dg + db * db) as u32;
        if dist < best_dist {
            best_dist = dist;
            best = i as u8;
        }
    }
    best
}

fn median_cut(entries: &[([u8; 3], u32)], max_colors: usize) -> Vec<[u8; 3]> {
    struct BoxColors {
        colors: Vec<[u8; 3]>,
        counts: Vec<u32>,
    }

    let mut boxes = vec![BoxColors {
        colors: entries.iter().map(|(c, _)| *c).collect(),
        counts: entries.iter().map(|(_, n)| *n).collect(),
    }];

    loop {
        if boxes.len() >= max_colors {
            break;
        }
        let Some((split_index, channel)) = boxes
            .iter()
            .enumerate()
            .filter_map(|(i, b)| {
                let (range, channel) = [0, 1, 2]
                    .map(|ch| {
                        let min = b.colors.iter().map(|c| c[ch]).min().unwrap_or(0);
                        let max = b.colors.iter().map(|c| c[ch]).max().unwrap_or(0);
                        (max as u16 - min as u16, ch)
                    })
                    .into_iter()
                    .max_by_key(|(range, _)| *range)?;
                if range == 0 { None } else { Some((i, channel)) }
            })
            .max_by_key(|(i, _)| {
                let b = &boxes[*i];
                let weight: u32 = b.counts.iter().sum();
                weight
            })
        else {
            break;
        };

        let box_colors = boxes.remove(split_index);
        let mut order: Vec<usize> = (0..box_colors.colors.len()).collect();
        order.sort_by_key(|&i| box_colors.colors[i][channel]);

        let total: u32 = order.iter().map(|&i| box_colors.counts[i]).sum();
        let mut acc = 0u32;
        let mut cut = order.len() / 2;
        for (pos, &i) in order.iter().enumerate() {
            acc += box_colors.counts[i];
            if acc * 2 >= total {
                cut = pos + 1;
                break;
            }
        }
        cut = cut.clamp(1, order.len() - 1);

        let right = order.split_off(cut);
        let left = order;
        let mut right_box = BoxColors {
            colors: Vec::new(),
            counts: Vec::new(),
        };
        for &i in &right {
            right_box.colors.push(box_colors.colors[i]);
            right_box.counts.push(box_colors.counts[i]);
        }
        let left_box = BoxColors {
            colors: left.iter().map(|&i| box_colors.colors[i]).collect(),
            counts: left.iter().map(|&i| box_colors.counts[i]).collect(),
        };
        boxes.push(left_box);
        boxes.push(right_box);
    }

    boxes
        .into_iter()
        .map(|b| {
            let total: u32 = b.counts.iter().sum();
            let mut mean = [0u32; 3];
            for (color, count) in b.colors.iter().zip(&b.counts) {
                for ch in 0..3 {
                    mean[ch] += color[ch] as u32 * count;
                }
            }
            [
                (mean[0] / total.max(1)) as u8,
                (mean[1] / total.max(1)) as u8,
                (mean[2] / total.max(1)) as u8,
            ]
        })
        .collect()
}

/// Encodes rendered frames into an animated GIF file.
///
/// `repeat` is the loop count, where `0` loops forever.
///
/// All frames share a single global color table built with median-cut
/// quantization over every frame, which keeps colors stable across the
/// animation (per-frame palettes cause visible color shimmer).
pub fn encode(path: &Path, frames: Vec<(RgbaImage, Duration)>, repeat: u16) -> Result<()> {
    let file = std::fs::File::create(path)
        .with_context(|| format!("failed to create output '{}'", path.display()))?;
    encode_to(std::io::BufWriter::new(file), frames, repeat)
        .with_context(|| format!("failed to encode GIF '{}'", path.display()))
}

/// Encodes frames into any writer, preserving each frame's delay.
pub fn encode_to<W: std::io::Write>(
    writer: W,
    frames: Vec<(RgbaImage, Duration)>,
    repeat: u16,
) -> anyhow::Result<()> {
    ensure_frames(&frames)?;
    let (width, height) = {
        let (first, _) = &frames[0];
        (first.width() as u16, first.height() as u16)
    };

    let quant = quantize(&frames);
    let mut encoder = Encoder::new(writer, width, height, &quant.palette)?;
    let repeat = if repeat > 0 {
        Repeat::Finite(repeat)
    } else {
        Repeat::Infinite
    };
    encoder.set_repeat(repeat)?;

    for (art, delay) in frames {
        let mut indexed = Vec::with_capacity((width as usize) * (height as usize));
        let mut frame_has_transparency = false;
        for pixel in art.pixels() {
            if pixel[3] < 128 {
                indexed.push(0);
                frame_has_transparency = true;
            } else {
                indexed.push(quant.lookup[bucket(pixel[0], pixel[1], pixel[2])]);
            }
        }

        let ms = delay.as_millis().clamp(0, u16::MAX as u128 * 10) as u16;
        let delay_cs = ((ms as u32 * 10 + 50) / 100).clamp(1, u16::MAX as u32) as u16;
        let frame = Frame {
            width,
            height,
            left: 0,
            top: 0,
            interlaced: false,
            buffer: Cow::Owned(indexed),
            delay: delay_cs,
            dispose: DisposalMethod::Background,
            transparent: frame_has_transparency.then_some(0),
            needs_user_input: false,
            palette: None,
        };
        encoder.write_frame(&frame)?;
    }
    Ok(())
}

fn ensure_frames(frames: &[(RgbaImage, Duration)]) -> anyhow::Result<()> {
    if frames.is_empty() {
        anyhow::bail!("no frames to encode");
    }
    let (first, _) = &frames[0];
    for (frame, _) in &frames[1..] {
        if frame.width() != first.width() || frame.height() != first.height() {
            anyhow::bail!("all frames must share the same dimensions");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gif::DecodeOptions;
    use image::Rgba;
    use std::io::Cursor;

    fn sample_frames() -> Vec<(RgbaImage, Duration)> {
        vec![
            (
                RgbaImage::from_pixel(20, 20, Rgba([255, 0, 0, 255])),
                Duration::from_millis(80),
            ),
            (
                RgbaImage::from_pixel(20, 20, Rgba([0, 0, 255, 255])),
                Duration::from_millis(240),
            ),
        ]
    }

    fn decoded_frames(bytes: &[u8]) -> Vec<Frame<'static>> {
        DecodeOptions::new()
            .read_info(bytes)
            .unwrap()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    #[test]
    fn encodes_frames_with_preserved_delays() {
        let mut bytes = Vec::new();
        encode_to(Cursor::new(&mut bytes), sample_frames(), 0).unwrap();

        let frames = decoded_frames(&bytes);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].delay, 8);
        assert_eq!(frames[1].delay, 24);
    }

    #[test]
    fn shares_one_global_palette_across_frames() {
        let mut bytes = Vec::new();
        encode_to(Cursor::new(&mut bytes), sample_frames(), 0).unwrap();

        let decoder = DecodeOptions::new().read_info(&bytes[..]).unwrap();
        assert!(!decoder.global_palette().unwrap_or_default().is_empty());
        let frames = decoder.into_iter().collect::<Result<Vec<_>, _>>().unwrap();
        for frame in &frames {
            assert!(
                frame.palette.is_none(),
                "local palette present: shimmer source"
            );
        }
    }

    #[test]
    fn global_palette_holds_every_decoded_index() {
        let mut bytes = Vec::new();
        encode_to(Cursor::new(&mut bytes), sample_frames(), 0).unwrap();

        let decoder = DecodeOptions::new().read_info(&bytes[..]).unwrap();
        let palette_len = decoder.global_palette().unwrap_or_default().len();
        drop(decoder);
        for frame in decoded_frames(&bytes) {
            for &index in frame.buffer.iter() {
                assert!(
                    usize::from(index) < palette_len / 3,
                    "index out of palette range"
                );
            }
        }
        assert_eq!(decoded_frames(&bytes)[0].buffer.len(), 20 * 20);
    }

    #[test]
    fn writes_loop_count_into_netscape_block() {
        fn loop_count(bytes: &[u8]) -> u16 {
            let i = bytes
                .windows(8)
                .position(|w| w == b"NETSCAPE")
                .expect("NETSCAPE extension missing");
            u16::from_le_bytes([bytes[i + 13], bytes[i + 14]])
        }

        let mut infinite = Vec::new();
        encode_to(Cursor::new(&mut infinite), sample_frames(), 0).unwrap();
        assert_eq!(loop_count(&infinite), 0);

        let mut finite = Vec::new();
        encode_to(Cursor::new(&mut finite), sample_frames(), 3).unwrap();
        assert_eq!(loop_count(&finite), 3);
    }

    #[test]
    fn many_distinct_colors_still_fit_256_entry_table() {
        let mut frame = RgbaImage::from_pixel(128, 128, Rgba([0, 0, 0, 255]));
        let mut n = 0u32;
        for pixel in frame.pixels_mut() {
            pixel.0 = [
                (n >> 16 & 255) as u8,
                (n >> 8 & 255) as u8,
                (n & 255) as u8,
                255,
            ];
            n = n.wrapping_add(1);
        }
        let mut bytes = Vec::new();
        encode_to(
            Cursor::new(&mut bytes),
            vec![(frame, Duration::from_millis(50))],
            0,
        )
        .unwrap();
        let decoder = DecodeOptions::new().read_info(&bytes[..]).unwrap();
        let palette_len = decoder.global_palette().unwrap_or_default().len();
        assert!(palette_len / 3 <= 256);
        drop(decoder);
        let frames = decoded_frames(&bytes);
        for &index in frames[0].buffer.iter() {
            assert!(usize::from(index) < palette_len / 3);
        }
    }

    #[test]
    fn gradient_frame_keeps_color_diversity() {
        let mut frame = RgbaImage::from_pixel(320, 320, Rgba([0, 0, 0, 255]));
        for (x, y, p) in frame.enumerate_pixels_mut() {
            p.0 = [
                (x * 255 / 319) as u8,
                (y * 255 / 319) as u8,
                ((x + y) * 255 / 638) as u8,
                255,
            ];
        }
        let mut bytes = Vec::new();
        encode_to(
            Cursor::new(&mut bytes),
            vec![(frame, Duration::from_millis(50))],
            0,
        )
        .unwrap();
        let decoder = DecodeOptions::new().read_info(&bytes[..]).unwrap();
        let palette = decoder.global_palette().unwrap_or_default();
        let distinct: std::collections::HashSet<_> = palette
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect();
        assert!(
            distinct.len() >= 128,
            "palette collapsed to {} distinct colors",
            distinct.len()
        );
    }

    #[test]
    fn transparency_in_any_frame_reserves_index_zero() {
        let mut frame = RgbaImage::from_pixel(10, 10, Rgba([10, 20, 30, 0]));
        frame.put_pixel(0, 0, Rgba([255, 255, 255, 255]));
        let mut bytes = Vec::new();
        encode_to(
            Cursor::new(&mut bytes),
            vec![(frame, Duration::from_millis(100))],
            0,
        )
        .unwrap();
        let frames = decoded_frames(&bytes);
        assert_eq!(frames[0].transparent, Some(0));
        assert_eq!(frames[0].buffer[1], 0);
    }
}
