use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

const DEFAULT_ALPHABET: &str = include_str!("../alphabets/alphabet.txt");

pub fn resolve(name: &str, alphabets_dir: &Path) -> Result<Vec<char>> {
    if !alphabets_dir.exists() {
        return Ok(if name == "alphabet" {
            DEFAULT_ALPHABET.chars().collect()
        } else {
            name.chars().collect()
        });
    }

    let alphabet_path = alphabets_dir.join(format!("{}.txt", name));
    if alphabet_path.exists() {
        let contents = fs::read_to_string(&alphabet_path).with_context(|| {
            format!("failed to read alphabet file '{}'", alphabet_path.display())
        })?;
        Ok(contents.chars().collect())
    } else {
        Ok(name.chars().collect())
    }
}
