use clap::Parser;

#[derive(Parser)]
pub struct Cli {
    pub image_path: String,
    #[arg(short, long, default_value = "alphabet")]
    pub alphabet: String,
    #[arg(short, long)]
    pub width: Option<usize>,
    #[arg(short, long)]
    pub no_color: bool,
    #[arg(long, default_value_t = 30.0)]
    pub fps: f64,
    #[arg(short, long, default_value = "edge-augmented")]
    pub conversion_algorithm: String,
}

pub fn parse() -> Cli {
    Cli::parse()
}
