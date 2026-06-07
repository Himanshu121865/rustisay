use std::fs;
use std::path::Path;

use log::info;

pub fn resolve(name: &str, alphabets_dir: &Path) -> Vec<char> {
    if !alphabets_dir.exists() {
        info!("alphabets dir not found, treating as literal: {}", name);
        return name.chars().collect();
    }

    let alphabet_path = alphabets_dir.join(format!("{}.txt", name));
    if alphabet_path.exists() {
        info!("reading alphabet from file\t{:?}", alphabet_path);
        let content = fs::read_to_string(&alphabet_path).unwrap();
        return content.chars().collect();
    }

    name.chars().collect()
}
