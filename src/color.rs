use anyhow::{Result, bail};
use image::Rgba;

/// Parses a hex color (`#rgb`, `#rrggbb`, or bare `rrggbb`) or the named
/// colors `black` and `white` into an opaque RGBA pixel.
pub fn parse_color(s: &str) -> Result<Rgba<u8>> {
    let hex = s.strip_prefix('#').unwrap_or(s);
    if hex.eq_ignore_ascii_case("black") {
        return Ok(Rgba([0, 0, 0, 255]));
    }
    if hex.eq_ignore_ascii_case("white") {
        return Ok(Rgba([255, 255, 255, 255]));
    }
    let digit = |c: char| -> Option<u8> {
        match c {
            '0'..='9' => Some(c as u8 - b'0'),
            'a'..='f' => Some(c as u8 - b'a' + 10),
            'A'..='F' => Some(c as u8 - b'A' + 10),
            _ => None,
        }
    };
    let channels = match hex.len() {
        3 => Some(
            hex.chars()
                .map(|c| digit(c).map(|d| d * 17))
                .collect::<Option<Vec<u8>>>()
                .ok_or_else(|| anyhow::anyhow!("invalid hex digit"))?,
        ),
        6 => Some(
            hex.chars()
                .collect::<Vec<_>>()
                .chunks(2)
                .map(|pair| match (digit(pair[0]), digit(pair[1])) {
                    (Some(hi), Some(lo)) => Some(hi * 16 + lo),
                    _ => None,
                })
                .collect::<Option<Vec<u8>>>()
                .ok_or_else(|| anyhow::anyhow!("invalid hex digit"))?,
        ),
        _ => None,
    };
    let Some(channels) = channels else {
        bail!(
            "invalid color '{}': expected #RRGGBB, #RGB, black, or white",
            s
        );
    };
    Ok(Rgba([channels[0], channels[1], channels[2], 255]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_hex() {
        assert_eq!(
            parse_color("#1a2b3c").unwrap(),
            Rgba([0x1a, 0x2b, 0x3c, 255])
        );
    }

    #[test]
    fn parses_bare_hex() {
        assert_eq!(parse_color("ff0000").unwrap(), Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn parses_short_hex() {
        assert_eq!(parse_color("#abc").unwrap(), Rgba([0xaa, 0xbb, 0xcc, 255]));
    }

    #[test]
    fn parses_named_colors() {
        assert_eq!(parse_color("black").unwrap(), Rgba([0, 0, 0, 255]));
        assert_eq!(parse_color("WHITE").unwrap(), Rgba([255, 255, 255, 255]));
    }

    #[test]
    fn rejects_invalid_colors() {
        assert!(parse_color("#12345").is_err());
        assert!(parse_color("#xyz").is_err());
        assert!(parse_color("").is_err());
    }
}
