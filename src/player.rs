use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use image::codecs::gif::GifDecoder;
use image::codecs::png::PngDecoder;
use image::codecs::webp::WebPDecoder;
use image::{AnimationDecoder, DynamicImage, Frames, Rgba, RgbaImage};
use rayon::prelude::*;

use crate::cli::Cli;
use crate::convert::ToneAdjust;
use crate::font::{Character, Font};
use crate::term;

/// Renders an image or animation to the terminal or to an output file.
pub struct Player<'a> {
    args: &'a Cli,
    font: &'a Font,
    path: &'a Path,
    tone: &'a ToneAdjust,
    bg: Rgba<u8>,
    terminal_dims: Option<(usize, usize)>,
}

impl<'a> Player<'a> {
    pub fn new(
        args: &'a Cli,
        font: &'a Font,
        path: &'a Path,
        tone: &'a ToneAdjust,
        bg: Rgba<u8>,
        terminal_dims: Option<(usize, usize)>,
    ) -> Self {
        Self {
            args,
            font,
            path,
            tone,
            bg,
            terminal_dims,
        }
    }

    /// Renders the image or animation to the full device. Loops forever for
    /// animated files, returns after a single frame for static images.
    pub fn play(&self) -> Result<()> {
        match self.open_frames()? {
            Some(frames) => self.play_animated(frames),
            None => self.play_static(),
        }
    }

    /// Renders every frame as a string, joining animated frames with a form
    /// feed (`\x0c`) separator, suitable for writing to a file.
    pub fn render_all(&self) -> Result<String> {
        if !self.args.no_color {
            colored::control::set_override(true);
        }

        let rendered: Vec<String> = match self.open_all_frames()? {
            Some(frames) => frames.par_iter().map(|f| self.frame_to_art(f)).collect(),
            None => {
                let img = self.open_image()?;
                vec![self.image_to_art(&img)]
            }
        };

        Ok(rendered.join("\x0c"))
    }

    /// Renders every frame as pixel art suitable for GIF encoding, returning
    /// each frame along with its source delay.
    pub fn render_all_gif(&self) -> Result<Vec<(RgbaImage, Duration)>> {
        let glyphs = self.font.char_lookup();

        let frames: Vec<(RgbaImage, Duration)> = match self.open_all_frames()? {
            Some(frames) => frames
                .par_iter()
                .map(|f| {
                    let img = image::DynamicImage::ImageRgba8(f.buffer().clone());
                    let art = self.render_pixels(&img, &glyphs);
                    (art, Duration::from(f.delay()))
                })
                .collect(),
            None => {
                let img = self.open_image()?;
                let art = self.render_pixels(&img, &glyphs);
                vec![(art, Duration::from_millis(100))]
            }
        };

        Ok(frames)
    }

    /// Decodes the animation into owned frames, or returns `None` for static
    /// images. Fails if the file is animated but yields no frames.
    fn open_all_frames(&self) -> Result<Option<Vec<image::Frame>>> {
        let Some(frames) = self.open_frames()? else {
            return Ok(None);
        };
        let frames = frames
            .collect::<std::result::Result<Vec<_>, _>>()
            .with_context(|| format!("failed to decode frames of '{}'", self.path.display()))?;
        if frames.is_empty() {
            bail!(
                "animated image '{}' contains no frames",
                self.path.display()
            );
        }
        Ok(Some(frames))
    }

    fn render_pixels(&self, img: &DynamicImage, glyphs: &HashMap<char, &Character>) -> RgbaImage {
        let char_rows = self.char_rows(img, None);
        crate::render::render_char_grid(
            self.font,
            glyphs,
            &char_rows,
            img,
            self.tone,
            self.args.no_color,
            self.bg,
        )
    }

    fn play_static(&self) -> Result<()> {
        let img = self.open_image()?;
        self.print_frame(&img);
        Ok(())
    }

    fn play_animated(&self, mut frames: Frames<'static>) -> Result<()> {
        loop {
            let mut frame_count = 0usize;
            for frame in &mut frames {
                let frame = frame.with_context(|| {
                    format!("failed to decode a frame of '{}'", self.path.display())
                })?;
                self.print_frame(&image::DynamicImage::ImageRgba8(frame.buffer().clone()));
                frame_count += 1;
            }

            if frame_count == 0 {
                bail!(
                    "animated image '{}' contains no frames",
                    self.path.display()
                );
            }

            frames = match self.open_frames()? {
                Some(frames) => frames,
                None => bail!("file '{}' is no longer animated", self.path.display()),
            };
        }
    }

    fn frame_to_art(&self, frame: &image::Frame) -> String {
        let img = image::DynamicImage::ImageRgba8(frame.buffer().clone());
        self.image_to_art(&img)
    }

    fn image_to_art(&self, img: &DynamicImage) -> String {
        let char_rows = self.char_rows(img, None);
        self.render_string(&char_rows, img)
    }

    fn print_frame(&self, img: &DynamicImage) {
        let t0 = Instant::now();
        let char_rows = self.char_rows(img, self.terminal_dims);
        let output = self.render_string(&char_rows, img);
        let (row, col) = term::center_offset(&char_rows, self.terminal_dims);
        print!("{}[2J{}[{};{}H{}", 27 as char, 27 as char, row, col, output);
        let elapsed = t0.elapsed().as_secs_f64();
        let delay = (1.0 / self.args.fps) - elapsed;
        if delay > 0.0 {
            std::thread::sleep(Duration::from_secs_f64(delay));
        }
    }

    fn char_rows(
        &self,
        img: &DynamicImage,
        terminal_dims: Option<(usize, usize)>,
    ) -> Vec<Vec<char>> {
        crate::convert::img_to_char_rows(self.font, img, self.args.width, terminal_dims, self.tone)
    }

    fn render_string(&self, char_rows: &[Vec<char>], img: &DynamicImage) -> String {
        if self.args.no_color {
            crate::convert::char_rows_to_string(char_rows)
        } else {
            crate::convert::char_rows_to_terminal_color_string(char_rows, img, self.tone)
        }
    }

    fn open_image(&self) -> Result<DynamicImage> {
        image::open(self.path)
            .with_context(|| format!("failed to open image '{}'", self.path.display()))
    }

    fn open_frames(&self) -> Result<Option<Frames<'static>>> {
        let ext = self
            .path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);

        match ext.as_deref() {
            Some("gif") => {
                let file = self.open_image_file()?;
                let decoder = GifDecoder::new(file)
                    .with_context(|| format!("failed to decode GIF '{}'", self.path.display()))?;
                Ok(Some(decoder.into_frames()))
            }
            Some("png" | "apng") => {
                let decoder = PngDecoder::new(self.open_image_file()?)
                    .with_context(|| format!("failed to decode PNG '{}'", self.path.display()))?;
                if decoder.is_apng() {
                    Ok(Some(decoder.apng().into_frames()))
                } else {
                    Ok(None)
                }
            }
            Some("webp") => {
                let decoder = WebPDecoder::new(self.open_image_file()?)
                    .with_context(|| format!("failed to decode WebP '{}'", self.path.display()))?;
                if decoder.has_animation() {
                    Ok(Some(decoder.into_frames()))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    fn open_image_file(&self) -> Result<std::io::BufReader<std::fs::File>> {
        let file = std::fs::File::open(self.path)
            .with_context(|| format!("failed to open image '{}'", self.path.display()))?;
        Ok(std::io::BufReader::new(file))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use crate::convert::ToneAdjust;
    use crate::font::Font;
    use image::codecs::gif::GifEncoder;
    use image::{Delay, Frame, Rgba};
    use std::io::{Cursor, Write};
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_path(ext: &str) -> std::path::PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!(
            "rustisay_test_{}_{}.{}",
            std::process::id(),
            n,
            ext
        ))
    }

    fn big_font() -> Font {
        Font::from_bdf_bytes(include_bytes!("../fonts/bitocra-13.bdf"), &['#'], false).unwrap()
    }

    fn player<'a>(
        path: &'a std::path::Path,
        cli: &'a Cli,
        font: &'a Font,
        tone: &'a ToneAdjust,
    ) -> Player<'a> {
        Player::new(cli, font, path, tone, Rgba([0, 0, 0, 255]), None)
    }

    fn base_cli(image_path: String, no_color: bool) -> Cli {
        Cli {
            image_path,
            alphabet: "alphabet".into(),
            width: None,
            no_color,
            output: None,
            invert: false,
            brightness: 0.0,
            contrast: 1.0,
            fps: 30.0,
            gif: false,
            repeat: 0,
            bg_color: "black".into(),
        }
    }

    fn write_test_gif(path: &std::path::Path) {
        let red = RgbaImage::from_pixel(60, 40, Rgba([255, 0, 0, 255]));
        let blue = RgbaImage::from_pixel(60, 40, Rgba([0, 0, 255, 255]));
        let mut bytes = Vec::new();
        {
            let mut enc = GifEncoder::new(Cursor::new(&mut bytes));
            enc.encode_frame(Frame::from_parts(
                red,
                0,
                0,
                Delay::from_numer_denom_ms(100, 1),
            ))
            .unwrap();
            enc.encode_frame(Frame::from_parts(
                blue,
                0,
                0,
                Delay::from_numer_denom_ms(250, 1),
            ))
            .unwrap();
        }
        std::fs::File::create(path)
            .unwrap()
            .write_all(&bytes)
            .unwrap();
    }

    #[test]
    fn render_all_gif_preserves_frames_delays_and_colors() {
        let path = temp_path("gif");
        write_test_gif(&path);
        let args = base_cli(path.display().to_string(), false);
        let font = big_font();
        let tone = ToneAdjust::default();
        let p = player(&path, &args, &font, &tone);

        let frames = p.render_all_gif().unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].1, Duration::from_millis(100));
        assert_eq!(frames[1].1, Duration::from_millis(250));

        // 60px / 7px font -> 9 columns, 40px -> 3 rows, canvas 63x39.
        assert_eq!(frames[0].0.dimensions(), (63, 39));
        assert_eq!(frames[1].0.dimensions(), (63, 39));

        // Opaque black background everywhere.
        assert!(frames[0].0.pixels().all(|p| p[3] == 255));

        // Glyph color follows the source frame color.
        let red = Rgba([255, 0, 0, 255]);
        let blue = Rgba([0, 0, 255, 255]);
        assert!(frames[0].0.pixels().filter(|&&p| p == red).count() > 0);
        assert!(frames[1].0.pixels().filter(|&&p| p == blue).count() > 0);
        assert_eq!(frames[1].0.pixels().filter(|&&p| p == red).count(), 0);

        // Frames differ.
        assert_ne!(frames[0].0.as_raw(), frames[1].0.as_raw());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn render_all_joins_frames_with_form_feed() {
        let path = temp_path("gif");
        write_test_gif(&path);
        let args = Cli {
            bg_color: "black".into(),
            gif: false,
            repeat: 0,
            ..base_cli(path.display().to_string(), true)
        };
        let font = big_font();
        let tone = ToneAdjust::default();
        let p = player(&path, &args, &font, &tone);

        let out = p.render_all().unwrap();
        let frames: Vec<&str> = out.split('\u{0c}').collect();
        assert_eq!(frames.len(), 2);
        assert!(
            frames
                .iter()
                .all(|f| f.lines().all(|l| l.chars().all(|c| c == '#')))
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn static_image_produces_single_frame() {
        let path = temp_path("png");
        let img = RgbaImage::from_pixel(26, 26, Rgba([0, 255, 0, 255]));
        img.save(&path).unwrap();

        let args = base_cli(path.display().to_string(), false);
        let font = big_font();
        let tone = ToneAdjust::default();
        let p = player(&path, &args, &font, &tone);

        let frames = p.render_all_gif().unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].1, Duration::from_millis(100));
        // 26x26 with a 7x13 font: 4 columns, 2 rows -> 28x26.
        assert_eq!(frames[0].0.dimensions(), (28, 26));

        let _ = std::fs::remove_file(&path);
    }
}
