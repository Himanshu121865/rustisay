use std::{collections::HashMap, path::Path};

use log::{info, warn};

use clap::Parser;

#[derive(Parser)]
struct Cli {
    image_path: String,
    #[arg(short, long, default_value = "alphabet")]
    alphabet: String,
    #[arg(short, long)]
    width: Option<usize>,
    #[arg(short, long)]
    no_color: bool,
    #[arg(long, default_value_t = 30.0)]
    fps: f64,
    #[arg(short, long, default_value = "edge-argumented")]
    conversion_algorithm: String,
}

const ALPHABETS: [(&str, &str); 6] = [
    ("alphabet", include_str!("../alphabets/alphabet.txt")),
    ("letters", include_str!("../alphabets/letters.txt")),
    ("lowercase", include_str!("../alphabets/lowercase.txt")),
    ("minimal", include_str!("../alphabets/minimal.txt")),
    ("symbols", include_str!("../alphabets/symbols.txt")),
    ("uppercase", include_str!("../alphabets/uppercase.txt")),
];

fn main() {
    env_logger::init();

    let cli = Cli::parse();

    let image_path = Path::new(&cli.image_path);
    info!("image path\t{:?}", image_path);
    let in_extension = image_path.extension().unwrap();

    let alphabet_str = &cli.alphabet;
    let alphabet_map: HashMap<&str, &str> = ALPHABETS.iter().cloned().collect();
    let alphabet: Vec<char> = if alphabet_map.contains_key(&alphabet_str.as_ref()) {
        info!("alphabet name\t{:?}", alphabet_str);
        alphabet_map
            .get(&alphabet_str.as_ref())
            .unwrap()
            .chars()
            .collect()
    } else {
        let alphabet_path = Path::new(alphabet_str);
        alphabet_str.chars().collect()
    };

    let fps = cli.fps;
    info!("fps\t{}", fps);

    let conversion_algorithm = cli.conversion_algorithm;
    info!("conversion algorithm\t{}", conversion_algorithm)
}
