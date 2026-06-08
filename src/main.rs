use std::path::Path;
use std::time::{Duration, Instant};

use image::DynamicImage;
use indicatif::ProgressIterator;

mod alphabet;
mod cli;
mod convert;
mod font;
mod progress;

struct Terminal;

impl Terminal {
    fn enter_alt() {
        print!("\x1b[?1049h");
    }

    fn exit_alt() {
        print!("\x1b[?1049l");
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        Self::exit_alt();
    }
}

fn main() {
    let _term = Terminal;

    Terminal::enter_alt();
    ctrlc::set_handler(|| {
        Terminal::exit_alt();
        std::process::exit(0);
    })
    .expect("error setting Ctrl+C handler");

    let args = cli::parse();

    let image_path = Path::new(&args.image_path);
    let alphabet = alphabet::resolve(&args.alphabet, Path::new("alphabets"));

    let font = font::Font::from_bdf_bytes(include_bytes!("../fonts/bitocra-13.bdf"), &alphabet, false);

    let frames: Vec<DynamicImage> = {
        if image_path.extension().map(|e| e == "gif").unwrap_or(false) {
            read_gif_frames(image_path)
        } else {
            vec![image::open(image_path).unwrap()]
        }
    };

    let mut frame_char_rows: Vec<Vec<Vec<char>>> = Vec::new();
    let progress = progress::default_progress_bar("Frames", frames.len());
    for img in frames.iter().progress_with(progress) {
        let ascii = convert::img_to_char_rows(&font, img, args.width);
        frame_char_rows.push(ascii);
    }

    let is_gif = frames.len() > 1;

    loop {
        for (char_rows, frame) in frame_char_rows.iter().zip(frames.iter()) {
            let t0 = Instant::now();
            let output = if args.no_color {
                convert::char_rows_to_string(char_rows)
            } else {
                convert::char_rows_to_terminal_color_string(char_rows, frame)
            };
            print!("{}[2J{}", 27 as char, output);
            let elapsed = t0.elapsed().as_secs_f64();
            let delay = (1.0 / args.fps) - elapsed;
            if delay > 0.0 {
                std::thread::sleep(Duration::from_secs_f64(delay));
            }
        }
        if !is_gif {
            break;
        }
    }
}

fn read_gif_frames(path: &Path) -> Vec<DynamicImage> {
    use image::codecs::gif::GifDecoder;
    use image::AnimationDecoder;

    let file = std::fs::File::open(path).unwrap();
    let decoder = GifDecoder::new(file).unwrap();
    let frames = decoder.into_frames().collect_frames().unwrap();
    frames.iter().map(|f| DynamicImage::ImageRgba8(f.buffer().clone())).collect()
}
