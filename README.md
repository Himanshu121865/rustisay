# rustisay

Turn images and GIF/APNG/WebP animations into colored ASCII art —
in your terminal or right in the browser.

## Install

```
cargo install rustisay
```

that's it.

## Usage

```
rustisay image.gif                # colored ASCII art, animates in the terminal
rustisay photo.jpg                # static images work too
rustisay image.gif --no-color     # turn color off → clean b&w
rustisay image.gif -a letters     # pick a character set
rustisay anim.gif -o out.gif      # save a real animated GIF
```

`-o out.gif` encodes the art back into pixels with the font glyphs and
writes a proper animated GIF, preserving the source frame delays. No
palette drift between frames — one global color palette for the whole
animation.

## Options

| Flag | Short | Description | Default |
|---|---|---|---|
| `<image_path>` | — | Image, GIF, APNG, or animated WebP | **(required)** |
| `--alphabet` | `-a` | Character set to use | `alphabet` |
| `--width` | `-w` | Output width in characters | auto |
| `--no-color` | `-n` | Disable color (b&w output) | `false` |
| `--output` | `-o` | Write to a file; `.gif` extension → real animated GIF | — |
| `--gif` | — | Force GIF output regardless of the file extension | `false` |
| `--repeat` | — | Loop count, `0` = forever | `0` |
| `--bg-color` | — | GIF background (`#RRGGBB`, `#RGB`, `black`, `white`) | `black` |
| `--invert` | — | Invert luminance (photo negative) | `false` |
| `--brightness` | — | Brightness in `-1.0..1.0` | `0.0` |
| `--contrast` | — | Contrast multiplier | `1.0` |
| `--fps` | — | Playback fps (animated files only) | `30.0` |

## Alphabets

Pick your text style with `-a`:

| File | Characters |
|---|---|
| `block.txt` | Solid block `█` — best for colored output |
| `alphabet.txt` | Full printable set (`!"#$...xyz{~}`) |
| `fast.txt` | Single `#` character |
| `letters.txt` | Upper and lowercase letters |
| `lowercase.txt` | `a-z` |
| `uppercase.txt` | `A-Z` |
| `minimal.txt` | `/\!.*^_` |
| `symbols.txt` | Punctuation and symbols |

## The web version

The exact same engine, compiled to WebAssembly with wasm-pack. Live
demo — no install, drag & drop, sliders, terminal-style playback,
GIF/TXT downloads:

**https://himanshu121865.github.io/rustisay/**

The `web/pkg/` output is also a publishable npm package:

```bash
cd web/pkg
npm publish
```

## License

MIT