use crate::font::Font;

pub type Converter = fn(&Font, &[f32]) -> char;

pub fn get_converter(metric: &str) -> Converter {
    match metric {
        "direction-and-intensity" => direction_and_intensity_convert,
        _ => panic!("unsupported metric: {}", metric),
    }
}

fn direction_and_intensity_convert(font: &Font, _chunk: &[f32]) -> char {
    font.chars[0].value
}
