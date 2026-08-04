use std::path::Path;

use anyhow::{Context, Result};

use crate::player::Player;

mod alphabet;
mod cli;
mod convert;
mod font;
mod gif;
mod player;
mod render;
mod term;

fn main() -> Result<()> {
    let args = cli::parse();

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
            gif::encode(Path::new(output_path), frames, args.repeat)?;
            return Ok(());
        }

        let output = player.render_all()?;
        std::fs::write(output_path, output)
            .with_context(|| format!("failed to write output '{}'", output_path))?;
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
