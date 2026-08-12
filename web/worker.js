import { ArtOptions, art_from_bytes, initSync } from "./pkg/rustisay.js";

const engine = (async () => {
  const res = await fetch(new URL("./pkg/rustisay_bg.wasm", import.meta.url));
  if (!res.ok) throw new Error(`failed to fetch rustisay_bg.wasm: HTTP ${res.status}`);
  initSync({ module: await res.arrayBuffer() });
})();

let latestId = 0;

self.onmessage = (e) => {
  const msg = e.data;
  latestId = msg.id;
  run(msg).catch((err) => {
    if (msg.id === latestId) self.postMessage({ id: msg.id, ok: false, error: String(err) });
  });
};

async function run(msg) {
  await engine;
  if (msg.id !== latestId) return;

  const opts = new ArtOptions();
  try {
    opts.width = msg.opts.width;
    opts.no_color = msg.opts.no_color;
    opts.invert = msg.opts.invert;
    opts.brightness = msg.opts.brightness;
    opts.contrast = msg.opts.contrast;
    opts.bg_color = msg.opts.bg_color;
    opts.repeat = msg.opts.repeat;
    opts.charset = msg.opts.charset;

    const art = art_from_bytes(msg.bytes, msg.filename, opts);
    if (msg.id !== latestId) return;

    self.postMessage({
      id: msg.id,
      ok: true,
      gif: art.gif,
      text_frames: art.text_frames,
      text_frames_color: art.text_frames_color,
      delays_ms: Array.from(art.delays_ms),
      frames: art.frames,
      width: art.width,
      height: art.height,
    });
    art.free();
  } finally {
    opts.free();
  }
}