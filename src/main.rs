#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use anyhow::{Context, Result, bail};

#[cfg(not(target_arch = "wasm32"))]
use crate::player::Player;

#[cfg(not(target_arch = "wasm32"))]
mod alphabet;
#[cfg(not(target_arch = "wasm32"))]
mod cli;
#[cfg(not(target_arch = "wasm32"))]
mod color;
#[cfg(not(target_arch = "wasm32"))]
mod convert;
#[cfg(not(target_arch = "wasm32"))]
mod font;
#[cfg(not(target_arch = "wasm32"))]
mod gif;
#[cfg(not(target_arch = "wasm32"))]
mod player;
#[cfg(not(target_arch = "wasm32"))]
mod render;
#[cfg(not(target_arch = "wasm32"))]
mod term;

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<()> {
    let args = cli::parse();

    if args.gif && args.output.is_none() {
        bail!("`--gif` requires `--output <file>`; pass `-o out.gif`");
    }
    if args.width == Some(0) {
        bail!("`--width` must be at least 1");
    }
    if !(-1.0..=1.0).contains(&args.brightness) {
        bail!(
            "`--brightness` must be in -1.0..1.0, got {}",
            args.brightness
        );
    }
    if args.contrast <= 0.0 {
        bail!("`--contrast` must be greater than 0, got {}", args.contrast);
    }
    if args.fps <= 0.0 {
        bail!("`--fps` must be greater than 0, got {}", args.fps);
    }

    let image_path = Path::new(&args.image_path);
    let alphabet = alphabet::resolve(&args.alphabet, Path::new("alphabets"))?;

    let font =
        font::Font::from_bdf_bytes(include_bytes!("../fonts/bitocra-13.bdf"), &alphabet, false)
            .context("failed to load embedded font")?;

    let tone = convert::ToneAdjust {
        invert: args.invert,
        brightness: args.brightness,
        contrast: args.contrast,
    };

    let bg = cli::parse_color(&args.bg_color)?;

    let player = Player::new(&args, &font, image_path, &tone, bg, term::dimensions());

    if let Some(output_path) = &args.output {
        if args.gif || output_path.to_ascii_lowercase().ends_with(".gif") {
            let frames = player.render_all_gif()?;
            let n = frames.len();
            gif::encode(Path::new(output_path), frames, args.repeat)?;
            eprintln!("wrote {n} frames to {} as ASCII GIF", output_path);
            return Ok(());
        }

        warn_if_unwritable_image(output_path);
        let output = player.render_all()?;
        std::fs::write(output_path, output)
            .with_context(|| format!("failed to write output '{}'", output_path))?;
        eprintln!("wrote ASCII art to {}", output_path);
        return Ok(());
    }

    let _term = term::Terminal::new();
    ctrlc::set_handler(|| {
        term::Terminal::exit_alt();
        std::process::exit(0);
    })
    .context("error setting Ctrl+C handler")?;

    player.play()
}

#[cfg(not(target_arch = "wasm32"))]
fn warn_if_unwritable_image(output: &str) {
    let ext = Path::new(output)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    if matches!(
        ext.as_deref(),
        Some("png" | "jpg" | "jpeg" | "webp" | "bmp" | "ico" | "tiff")
    ) {
        eprintln!(
            "note: only .gif output is supported for images; writing {} as plain text (use a .gif extension or --gif)",
            output
        );
    }
}
