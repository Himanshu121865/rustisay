use std::collections::HashMap;

#[derive(Clone)]
pub struct Character {
    pub value: char,
    pub bitmap: Vec<f32>,
    pub width: usize,
    pub height: usize,
    pub intensity: f32,
    pub direction: (f32, f32),
}

#[derive(Clone)]
pub struct Font {
    pub width: usize,
    pub height: usize,
    pub chars: Vec<Character>,
    pub char_map: HashMap<char, Character>,
    pub intensity_chars: Vec<Character>,
}
