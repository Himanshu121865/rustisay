use std::path::Path;

use log::info;

mod alphabet;
mod cli;

fn main() {
    env_logger::init();

    let args = cli::parse();

    let image_path = Path::new(&args.image_path);
    info!("image path\t{:?}", image_path);

    let _alphabet = alphabet::resolve(&args.alphabet, Path::new("alphabets"));

    let _fps = args.fps;
    info!("fps\t{}", _fps);

    let _conversion_algorithm = args.conversion_algorithm;
    info!("conversion algorithm\t{}", _conversion_algorithm)
}
