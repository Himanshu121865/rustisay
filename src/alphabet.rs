#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use anyhow::{Context, Result};

#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_ALPHABET: &str = include_str!("../alphabets/alphabet.txt");

#[cfg(not(target_arch = "wasm32"))]
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

/// Resolves a built-in alphabet by name without touching the filesystem.
/// Unknown names are treated as a literal string of characters, mirroring
/// the CLI fallback for `--alphabet`.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn embedded(name: &str) -> Vec<char> {
    match name {
        "alphabet" => include_str!("../alphabets/alphabet.txt"),
        "fast" => include_str!("../alphabets/fast.txt"),
        "letters" => include_str!("../alphabets/letters.txt"),
        "lowercase" => include_str!("../alphabets/lowercase.txt"),
        "minimal" => include_str!("../alphabets/minimal.txt"),
        "symbols" => include_str!("../alphabets/symbols.txt"),
        "uppercase" => include_str!("../alphabets/uppercase.txt"),
        "block" => include_str!("../alphabets/block.txt"),
        _ => name,
    }
    .chars()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_names_resolve() {
        assert_eq!(embedded("fast"), vec!['#']);
        assert_eq!(embedded("block"), vec!['█']);
        assert_eq!(
            embedded("uppercase"),
            " ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars().collect::<Vec<_>>()
        );
        assert!(embedded("alphabet").len() > 30);
    }

    #[test]
    fn unknown_names_are_literal() {
        assert_eq!(embedded("#@"), vec!['#', '@']);
    }
}
