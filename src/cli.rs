use clap::Parser;

pub use crate::color::parse_color;

#[derive(Parser)]
#[command(
    version,
    about = "Convert images and animated GIFs/APNGs/WebPs to colored ASCII art in the terminal",
    after_help = "EXAMPLES:\n  rustisay meme.gif                     # play ASCII animation in the terminal\n  rustisay photo.png -w 80 -o art.txt    # save plain ASCII text\n  rustisay anim.gif -o anim.gif          # save a real animated ASCII GIF\n  rustisay anim.gif -o out.gif --repeat 3 --bg-color '#101010'\n  rustisay img.png --output out.gif --gif # force GIF file output"
)]
pub struct Cli {
    /// Path to an image, GIF, APNG, or animated WebP file
    pub image_path: String,

    /// Character set to use (file in alphabets/, or a literal string)
    #[arg(short, long, default_value = "alphabet")]
    pub alphabet: String,

    /// Output width in characters (defaults to fitting the image)
    #[arg(short, long)]
    pub width: Option<usize>,

    /// Disable color (B&W output)
    #[arg(short, long)]
    pub no_color: bool,

    /// Write output to a file instead of playing in the terminal;
    /// a .gif extension writes a real animated GIF
    #[arg(short, long)]
    pub output: Option<String>,

    /// Invert luminance (photo negative)
    #[arg(long, default_value_t = false)]
    pub invert: bool,

    /// Brightness adjustment in -1.0..1.0
    #[arg(long, default_value_t = 0.0)]
    pub brightness: f32,

    /// Contrast multiplier
    #[arg(long, default_value_t = 1.0)]
    pub contrast: f32,

    /// Frames per second for terminal playback
    #[arg(long, default_value_t = 30.0)]
    pub fps: f64,

    /// Force GIF output regardless of the `--output` file extension
    #[arg(long, default_value_t = false)]
    pub gif: bool,

    /// Number of times a GIF output loops; 0 loops forever
    #[arg(long, default_value_t = 0)]
    pub repeat: u16,

    /// Background color of the GIF output, as hex (#RRGGBB) or black/white
    #[arg(long, default_value = "black")]
    pub bg_color: String,
}

pub fn parse() -> Cli {
    Cli::parse()
}
