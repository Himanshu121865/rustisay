use terminal_size::{Height, Width, terminal_size};

/// RAII guard for the terminal alternate screen buffer: entering the
/// alternate screen on construction, leaving it on drop.
pub struct Terminal;

impl Terminal {
    pub fn new() -> Self {
        print!("\x1b[?1049h");
        Self
    }

    pub fn exit_alt() {
        print!("\x1b[?1049l");
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        Self::exit_alt();
    }
}

pub fn dimensions() -> Option<(usize, usize)> {
    terminal_size().map(|(Width(w), Height(h))| (w as usize, h as usize))
}

/// Upper-left corner (row, col) to print art so it is centered in the terminal.
pub fn center_offset(
    char_rows: &[Vec<char>],
    terminal_dims: Option<(usize, usize)>,
) -> (usize, usize) {
    let (term_w, term_h) = match terminal_dims {
        Some(dims) => dims,
        None => return (1, 1),
    };
    let art_w = char_rows.first().map(|r| r.len()).unwrap_or(0);
    let art_h = char_rows.len();
    let col = term_w.saturating_sub(art_w) / 2 + 1;
    let row = term_h.saturating_sub(art_h) / 2 + 1;
    (row, col)
}
