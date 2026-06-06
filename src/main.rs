use std::path::PathBuf;

use log::{info, warn};

use clap::Parser;

#[derive(Parser)]
struct Cli {
    image_path: String,
    #[arg(short, long, default_value = "alphabet")]
    alphabet: String,
    #[arg(long, default_value_t = 30.0)]
    fps: f64,
}

fn main() {
    env_logger::init();
}
