//! WebAssembly bindings. Compiled only for the `wasm32-unknown-unknown`
//! target; everything here runs on in-memory buffers, no filesystem.
//!
//! `art_from_bytes` mirrors the CLI pipeline: decode the (possibly animated)
//! input, convert each frame to ASCII art with the embedded BDF font, and
//! return the encoded ASCII GIF plus per-frame text and timings.

use std::io::Cursor;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use image::codecs::gif::GifDecoder;
use image::codecs::png::PngDecoder;
use image::codecs::webp::WebPDecoder;
use image::{AnimationDecoder, DynamicImage};
use wasm_bindgen::prelude::*;

use crate::color::parse_color;
use crate::convert::{ToneAdjust, char_rows_to_string, img_to_char_rows};
use crate::font::Font;
use crate::gif;
use crate::render::render_char_grid;

/// Options mirroring the CLI flags. `width` of `0` means auto.
#[wasm_bindgen]
pub struct ArtOptions {
    pub width: u32,
    pub no_color: bool,
    pub invert: bool,
    pub brightness: f32,
    pub contrast: f32,
    repeat: u16,
    bg_color: String,
    charset: String,
}

#[wasm_bindgen]
impl ArtOptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> ArtOptions {
        ArtOptions {
            width: 0,
            no_color: false,
            invert: false,
            brightness: 0.0,
            contrast: 1.0,
            repeat: 0,
            bg_color: "black".into(),
            charset: "alphabet".into(),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn repeat(&self) -> u16 {
        self.repeat
    }

    #[wasm_bindgen(setter)]
    pub fn set_repeat(&mut self, repeat: u16) {
        self.repeat = repeat;
    }

    #[wasm_bindgen(getter)]
    pub fn bg_color(&self) -> String {
        self.bg_color.clone()
    }

    #[wasm_bindgen(setter)]
    pub fn set_bg_color(&mut self, bg_color: String) {
        self.bg_color = bg_color;
    }

    #[wasm_bindgen(getter)]
    pub fn charset(&self) -> String {
        self.charset.clone()
    }

    #[wasm_bindgen(setter)]
    pub fn set_charset(&mut self, charset: String) {
        self.charset = charset;
    }
}

/// Result of a conversion: the encoded ASCII GIF, the text art per frame
/// (form-feed-free, one entry per frame), the per-frame delays in
/// milliseconds, and the pixel dimensions of the rendered art.
#[wasm_bindgen]
pub struct Art {
    gif: Vec<u8>,
    text_frames: Vec<String>,
    delays_ms: Vec<f64>,
    frames: usize,
    width: u32,
    height: u32,
}

#[wasm_bindgen]
impl Art {
    #[wasm_bindgen(getter)]
    pub fn gif(&self) -> Vec<u8> {
        self.gif.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn text_frames(&self) -> Vec<String> {
        self.text_frames.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn delays_ms(&self) -> Vec<f64> {
        self.delays_ms.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn frames(&self) -> usize {
        self.frames
    }

    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.height
    }
}

/// Converts image/GIF/APNG/WebP bytes into an ASCII GIF plus text frames.
///
/// `filename` is only used to detect animation (by extension); the bytes are
/// the source of truth for everything else.
#[wasm_bindgen]
pub fn art_from_bytes(input: &[u8], filename: &str, opts: &ArtOptions) -> Result<Art, JsValue> {
    convert(input, filename, opts).map_err(|e| JsValue::from_str(&format!("{:#}", e)))
}

fn convert(input: &[u8], filename: &str, opts: &ArtOptions) -> Result<Art> {
    let charset = crate::alphabet::embedded(&opts.charset);
    let font = Font::from_bdf_bytes(include_bytes!("../fonts/bitocra-13.bdf"), &charset, false)
        .context("failed to load embedded font")?;
    let tone = ToneAdjust {
        invert: opts.invert,
        brightness: opts.brightness,
        contrast: opts.contrast,
    };
    let bg = parse_color(&opts.bg_color)?;
    let width = if opts.width == 0 {
        None
    } else {
        Some(opts.width as usize)
    };
    let glyphs = font.char_lookup();

    let frames: Vec<(DynamicImage, Duration)> = match open_frames(input, filename)? {
        Some(frames) => {
            if frames.is_empty() {
                bail!("no frames found in '{}'", filename);
            }
            frames
        }
        None => {
            let img = image::load_from_memory(input)
                .context("failed to decode image (unsupported format or corrupt file)")?;
            vec![(img, Duration::from_millis(100))]
        }
    };

    let mut arts = Vec::with_capacity(frames.len());
    let mut text_frames = Vec::with_capacity(frames.len());
    let mut delays_ms = Vec::with_capacity(frames.len());
    for (img, delay) in frames {
        let char_rows = img_to_char_rows(&font, &img, width, None, &tone);
        let art = render_char_grid(&font, &glyphs, &char_rows, &img, &tone, opts.no_color, bg);
        text_frames.push(char_rows_to_string(&char_rows));
        delays_ms.push(delay.as_millis() as f64);
        arts.push((art, delay));
    }

    let (width, height) = {
        let dims = arts[0].0.dimensions();
        (dims.0, dims.1)
    };
    let mut gif_bytes = Vec::new();
    gif::encode_to(&mut gif_bytes, arts, opts.repeat)?;

    Ok(Art {
        gif: gif_bytes,
        frames: text_frames.len(),
        width,
        height,
        text_frames,
        delays_ms,
    })
}

/// Decodes animated formats from bytes; returns `None` for static images.
fn open_frames(input: &[u8], filename: &str) -> Result<Option<Vec<(DynamicImage, Duration)>>> {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);

    let collected: Option<Vec<(DynamicImage, Duration)>> = match ext.as_deref() {
        Some("gif") => {
            let decoder = GifDecoder::new(Cursor::new(input))?;
            Some(collect_frames(decoder.into_frames()))
        }
        Some("png" | "apng") => {
            let decoder = PngDecoder::new(Cursor::new(input))?;
            if decoder.is_apng() {
                Some(collect_frames(decoder.apng().into_frames()))
            } else {
                None
            }
        }
        Some("webp") => {
            let decoder = WebPDecoder::new(Cursor::new(input))?;
            if decoder.has_animation() {
                Some(collect_frames(decoder.into_frames()))
            } else {
                None
            }
        }
        _ => None,
    };
    Ok(collected)
}

fn collect_frames(frames: image::Frames) -> Vec<(DynamicImage, Duration)> {
    frames
        .filter_map(|f| f.ok())
        .map(|f| {
            let delay = Duration::from(f.delay());
            let img = DynamicImage::ImageRgba8(f.into_buffer());
            (img, delay)
        })
        .collect()
}
