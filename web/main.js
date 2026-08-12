import init, { ArtOptions, art_from_bytes } from "./pkg/rustisay.js";

const $ = (id) => document.getElementById(id);

let art = null;
let fileBytes = null;
let fileName = null;
let playing = false;
let timer = null;
let slice = 0;

await init();

$("dropzone").addEventListener("click", () => $("file-input").click());
$("dropzone").addEventListener("keydown", (e) => {
  if (e.key === "Enter" || e.key === " ") $("file-input").click();
});
$("file-input").addEventListener("change", (e) => {
  if (e.target.files[0]) loadFile(e.target.files[0]);
});
["dragenter", "dragover"].forEach((ev) =>
  $("dropzone").addEventListener(ev, (e) => {
    e.preventDefault();
    $("dropzone").classList.add("dragover");
  })
);
["dragleave", "drop"].forEach((ev) =>
  $("dropzone").addEventListener(ev, (e) => {
    e.preventDefault();
    $("dropzone").classList.remove("dragover");
  })
);
$("dropzone").addEventListener("drop", (e) => {
  if (e.dataTransfer.files[0]) loadFile(e.dataTransfer.files[0]);
});

$("ctl-brightness").addEventListener("input", (e) => {
  $("out-brightness").textContent = Number(e.target.value).toFixed(2);
});
$("ctl-contrast").addEventListener("input", (e) => {
  $("out-contrast").textContent = Number(e.target.value).toFixed(2);
});

$("btn-convert").addEventListener("click", convert);
$("btn-play").addEventListener("click", togglePlay);
$("btn-dl-gif").addEventListener("click", () => downloadGif());
$("btn-dl-txt").addEventListener("click", () => downloadTxt());

function loadFile(file) {
  stopPlayback();
  $("error").hidden = true;
  $("pre").hidden = true;
  fileBytes = null;
  art = null;
  fileName = file.name;
  file.arrayBuffer().then((buf) => {
    fileBytes = new Uint8Array(buf);
    $("btn-convert").disabled = false;
    $("stat-text").textContent = `${fileName} (${(fileBytes.length / 1024).toFixed(1)} KiB) ready — click Convert`;
  });
}

function currentOptions() {
  const opts = new ArtOptions();
  opts.width = Math.max(0, parseInt($("ctl-width").value, 10) || 0);
  opts.no_color = $("ctl-bw").checked;
  opts.invert = $("ctl-invert").checked;
  opts.brightness = Number($("ctl-brightness").value);
  opts.contrast = Number($("ctl-contrast").value);
  opts.bg_color = $("ctl-bg").value.trim() || "black";
  opts.repeat = Math.min(65535, Math.max(0, parseInt($("ctl-repeat").value, 10) || 0));
  opts.charset = $("ctl-charset").value;
  return opts;
}

function convert() {
  if (!fileBytes) return;
  stopPlayback();
  $("error").hidden = true;
  $("pre").hidden = true;
  $("stat-text").textContent = "Converting…";
  try {
    const t0 = performance.now();
    art = art_from_bytes(fileBytes, fileName, currentOptions());
    const ms = (performance.now() - t0).toFixed(0);
    $("pre").textContent = art.text_frames[0];
    $("pre").hidden = false;
    $("btn-play").disabled = false;
    $("btn-dl-gif").disabled = false;
    $("btn-dl-txt").disabled = false;
    $("frame-count").textContent = `${art.frames} frame${art.frames === 1 ? "" : "s"} · ${art.width}×${art.height} chars · ${ms} ms`;
    $("stat-text").textContent = `Converted ${fileName}${art.frames > 1 ? " (animated)" : ""}`;
    if (art.frames > 1) play();
  } catch (err) {
    $("error").textContent = String(err);
    $("error").hidden = false;
    $("btn-convert").disabled = false;
    $("stat-text").textContent = "Conversion failed";
  }
}

function play() {
  if (!art || art.frames < 2 || playing) return;
  playing = true;
  $("btn-play").textContent = "Pause";
  slice = 0;
  scheduleNext();
}

function stopPlayback() {
  playing = false;
  clearTimeout(timer);
  $("btn-play").textContent = "Play";
}

function togglePlay() {
  if (playing) stopPlayback();
  else play();
}

function scheduleNext() {
  if (!playing) return;
  $("pre").textContent = art.text_frames[slice % art.frames];
  const delay = art.delays_ms[slice % art.frames];
  slice = (slice + 1) % art.frames;
  timer = setTimeout(scheduleNext, Math.max(10, delay));
}

function downloadGif() {
  if (!art) return;
  const blob = new Blob([art.gif], { type: "image/gif" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = fileName.replace(/\.[^.]+$/, "") + "-ascii.gif";
  a.click();
  URL.revokeObjectURL(url);
}

function downloadTxt() {
  if (!art) return;
  const blob = new Blob([art.text_frames.join("\f")], { type: "text/plain" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = fileName.replace(/\.[^.]+$/, "") + "-ascii.txt";
  a.click();
  URL.revokeObjectURL(url);
}