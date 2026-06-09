# rustisay

Convert images to colored ASCII art in the terminal, with GIF support.

## Install

```
cargo install rustisay
```

## Usage

```bash
cargo run -- <image_path> [options]
```

## Options

| Flag | Short | Description | Default |
|---|---|---|---|
| `<image_path>` | — | Path to image or GIF file | **(required)** |
| `--alphabet` | `-a` | Character set to use | `alphabet` |
| `--width` | `-w` | Output width in characters | auto |
| `--no-color` | `-n` | Disable color (B&W output) | `false` |
| `--fps` | — | Frames per second (GIF only) | `30.0` |

## Alphabets

| File | Characters |
|---|---|
| `alphabet.txt` | Full printable set (`!"#$...xyz{~}`) |
| `fast.txt` | Single `#` character |
| `letters.txt` | Upper and lowercase letters |
| `lowercase.txt` | `a-z` |
| `uppercase.txt` | `A-Z` |
| `minimal.txt` | `/\!.*^_` |
| `symbols.txt` | Punctuation and symbols |

## Features

- Animated GIF playback at configurable FPS
- Customizable alphabets for different density/style
- `--no-color` for monochrome terminal output
- Progress bar during frame processing
- Clean enter/exit of terminal alternate screen buffer

## License

MIT
