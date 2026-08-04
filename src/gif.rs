use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, RgbaImage};

/// Encodes rendered frames into an animated GIF file.
///
/// `repeat` is the loop count, where `0` loops forever.
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
    let mut encoder = GifEncoder::new(writer);
    let repeat = if repeat > 0 {
        Repeat::Finite(repeat)
    } else {
        Repeat::Infinite
    };
    encoder.set_repeat(repeat)?;

    for (art, delay) in frames {
        let ms = delay.as_millis().min(u32::MAX as u128) as u32;
        let frame = Frame::from_parts(art, 0, 0, Delay::from_numer_denom_ms(ms.max(1), 1));
        encoder.encode_frame(frame)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::codecs::gif::GifDecoder;
    use image::{AnimationDecoder, Rgba};
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

    #[test]
    fn encodes_frames_with_preserved_delays() {
        let mut bytes = Vec::new();
        encode_to(Cursor::new(&mut bytes), sample_frames(), 0).unwrap();

        let decoder = GifDecoder::new(&bytes[..]).unwrap();
        let frames: Vec<_> = decoder
            .into_frames()
            .collect::<image::ImageResult<Vec<_>>>()
            .unwrap();
        assert_eq!(frames.len(), 2);
        let delays: Vec<Duration> = frames.iter().map(|f| Duration::from(f.delay())).collect();
        assert_eq!(
            delays,
            vec![Duration::from_millis(80), Duration::from_millis(240)]
        );
    }

    #[test]
    fn writes_loop_count_into_netscape_block() {
        fn loop_count(bytes: &[u8]) -> u16 {
            let i = bytes
                .windows(8)
                .position(|w| w == b"NETSCAPE")
                .expect("NETSCAPE extension missing");
            // ... NETSCAPE2.0 03 01 <lo> <hi> 00
            u16::from_le_bytes([bytes[i + 13], bytes[i + 14]])
        }

        let mut infinite = Vec::new();
        encode_to(Cursor::new(&mut infinite), sample_frames(), 0).unwrap();
        assert_eq!(loop_count(&infinite), 0);

        let mut finite = Vec::new();
        encode_to(Cursor::new(&mut finite), sample_frames(), 3).unwrap();
        assert_eq!(loop_count(&finite), 3);
    }
}
