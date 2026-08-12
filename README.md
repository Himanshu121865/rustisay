# rustisay

Convert images and animated GIFs/APNGs/WebPs to colored ASCII art in the terminal.

## Install

```
cargo install rustisay
```

## Usage

```bash
rustisay <image_path> [options]
```

## Options

| Flag | Short | Description | Default |
|---|---|---|---|
| `<image_path>` | — | Path to image, GIF, APNG, or animated WebP file | **(required)** |
| `--alphabet` | `-a` | Character set to use | `alphabet` |
| `--width` | `-w` | Output width in characters | auto |
| `--no-color` | `-n` | Disable color (B&W output) | `false` |
| `--output` | `-o` | Write ASCII art to a file (no terminal playback); `.gif` extension writes a real animated GIF | — |
| `--gif` | — | Force GIF output regardless of the `--output` file extension | `false` |
| `--repeat` | — | Loop count for GIF output; `0` loops forever | `0` |
| `--bg-color` | — | Background color of the GIF output (`#RRGGBB`, `#RGB`, `black`, `white`) | `black` |
| `--invert` | — | Invert luminance (photo negative) | `false` |
| `--brightness` | — | Brightness adjustment in `-1.0..1.0` | `0.0` |
| `--contrast` | — | Contrast multiplier | `1.0` |
| `--fps` | — | Frames per second (animated files only) | `30.0` |

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

## Web demo & npm package

Run in the browser, or use the same engine from JavaScript:

```bash
# rebuild the wasm package after changing Rust code
wasm-pack build --target web --out-dir web/pkg

# local demo server
python3 -m http.server 8000 --directory web
# → http://localhost:8000
```

The demo (drag & drop, controls, terminal-style text playback, GIF/TXT downloads) is
self-contained in `web/`. The `web/pkg/` output is a publishable npm package:

```bash
cd web/pkg
npm publish
```

JavaScript usage:

```js
import init, { ArtOptions, art_from_bytes } from "rustisay";
await init();

const opts = new ArtOptions();
opts.width = 80;          // 0 = auto
opts.charset = "letters"; // built-in, or any literal string
opts.bg_color = "#101010";

const art = art_from_bytes(bytes, "anim.gif", opts);
// art.gif          → Uint8Array of the ASCII GIF
// art.text_frames  → one string per frame (monochrome)
// art.delays_ms    → per-frame playback delays
// art.width/height → rendered char dimensions
```

## Features

- Animated playback (GIF, APNG, animated WebP) at configurable FPS, with frames decoded on demand
- Static images render once; animated files loop forever
- Customizable alphabets for different density/style
- `--no-color` for monochrome terminal output
- Auto-fit to terminal size with centered output
- Clean enter/exit of terminal alternate screen buffer
- `--output` saves animation frames joined by form-feed characters (suitable for `cat` pagers)
- `--output out.gif` renders each frame back into pixels with the font glyphs and encodes a real animated GIF, preserving the source frame delays
- Parallel frame rendering (rayon)

## License

MIT
