use std::fs;
use std::path::Path;

const DEFAULT_ALPHABET: &str = include_str!("../alphabets/alphabet.txt");

pub fn resolve(name: &str, alphabets_dir: &Path) -> Vec<char> {
    if !alphabets_dir.exists() {
        return if name == "alphabet" {
            DEFAULT_ALPHABET.chars().collect()
        } else {
            name.chars().collect()
        };
    }

    let alphabet_path = alphabets_dir.join(format!("{}.txt", name));
    if alphabet_path.exists() {
        fs::read_to_string(&alphabet_path).unwrap().chars().collect()
    } else {
        name.chars().collect()
    }
}
