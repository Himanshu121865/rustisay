const $ = (id) => document.getElementById(id);

let art = null;
let fileBytes = null;
let fileName = null;
let playing = false;
let timer = null;
let idx = 0;
let speed = 1;
let view = "text";
let previewUrl = null;
let reqId = 0;
let convertStarted = 0;

const BASE_URL = new URL(".", import.meta.url);

const worker = new Worker(new URL("./worker.js", import.meta.url), { type: "module" });
worker.onmessage = (e) => onConvertResult(e.data);
worker.onerror = (e) => {
  hideStage();
  showError("The conversion worker crashed: " + (e.message || e));
  setStatus("Conversion failed");
};

/* ---------- dropzone / file picker ---------- */

const dropzone = $("dropzone");
let dragDepth = 0;

dropzone.addEventListener("click", () => $("file-input").click());
dropzone.addEventListener("keydown", (e) => {
  if (e.key === "Enter" || e.key === " ") {
    e.preventDefault();
    $("file-input").click();
  }
});
$("file-input").addEventListener("change", (e) => {
  if (e.target.files[0]) loadFile(e.target.files[0]);
});

["dragenter", "dragover"].forEach((ev) =>
  window.addEventListener(ev, (e) => {
    e.preventDefault();
    dragDepth++;
    showDragOverlay();
  })
);
["dragleave"].forEach((ev) =>
  window.addEventListener(ev, (e) => {
    e.preventDefault();
    dragDepth = Math.max(0, dragDepth - 1);
    if (dragDepth === 0) hideDragOverlay();
  })
);
window.addEventListener("drop", (e) => {
  e.preventDefault();
  dragDepth = 0;
  hideDragOverlay();
  if (e.dataTransfer.files[0]) loadFile(e.dataTransfer.files[0]);
});
dropzone.addEventListener("drop", (e) => {
  if (e.dataTransfer.files[0]) loadFile(e.dataTransfer.files[0]);
});
["dragenter", "dragover", "dragleave"].forEach((ev) =>
  dropzone.addEventListener(ev, (e) => e.preventDefault())
);

$("btn-clear").addEventListener("click", (e) => {
  e.stopPropagation();
  clearFile();
});
$("btn-sample").addEventListener("click", (e) => {
  e.stopPropagation();
  fetch(`${new URL("sample.gif", BASE_URL)}?v=2`)
    .then((r) => {
      if (!r.ok) throw new Error(`sample.gif: HTTP ${r.status}`);
      return r.arrayBuffer();
    })
    .then((buf) => {
      loadFile({ name: "sample.gif", size: buf.byteLength, data: new Uint8Array(buf) });
    })
    .catch((err) => showError(String(err)));
});
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && fileBytes && e.target === document.body) clearFile();
});

function loadFile(file) {
  stopPlayback();
  hideError();
  hideStage();
  fileBytes = null;
  art = null;
  const label = file.name || "sample.gif";
  setStatus(`Reading ${label}…`);
  readFile(file).then((buf) => {
    fileName = label;
    fileBytes = buf;
    setPreview(file, buf);
    setStatus(`${label} loaded — converting…`);
    scheduleConvert();
  });
}

function readFile(file) {
  if (file.data) return Promise.resolve(file.data);
  return file.arrayBuffer().then((b) => new Uint8Array(b));
}

function setPreview(file, bytes) {
  const type = (file.type || "").toLowerCase();
  const ext = fileName.match(/\.([a-z0-9]+)$/i)?.[1]?.toLowerCase();
  const isImage =
    type.startsWith("image/") ||
    ["gif", "apng", "png", "jpg", "jpeg", "webp", "bmp", "ico", "avif"].includes(ext);

  dropzone.classList.add("has-file");
  $("dz-icon").hidden = true;

  if (isImage) {
    previewUrl = URL.createObjectURL(new Blob([bytes], { type: type || "image/" + ext }));
    const img = $("file-preview");
    img.onload = () => {
      $("file-meta").textContent =
        `${img.naturalWidth}×${img.naturalHeight} px · ${(bytes.length / 1024).toFixed(1)} KiB · ` +
        (ext ? ext.toUpperCase() : type.split("/")[1] || "image");
      $("file-meta").hidden = false;
    };
    img.src = previewUrl;
    img.hidden = false;
  } else {
    previewUrl = null;
    $("file-preview").hidden = true;
    $("file-meta").textContent = `${(bytes.length / 1024).toFixed(1)} KiB · ${ext || "unknown format"}`;
    $("file-meta").hidden = false;
  }

  $("dz-main").textContent = fileName;
  $("dz-sub").textContent = "click to change · Esc to clear";
  $("file-chip").hidden = true;
  $("btn-clear").hidden = false;
  $("dz-row").hidden = false;
  $("file-input").value = "";
}

function clearFile() {
  stopPlayback();
  reqId++;
  hideStage();
  dropzone.classList.remove("has-file");
  $("dz-icon").hidden = false;
  $("file-preview").hidden = true;
  $("file-meta").hidden = true;
  $("file-chip").hidden = true;
  $("btn-clear").hidden = true;
  $("dz-row").hidden = true;
  $("dz-main").textContent = "Drop an image, GIF, APNG or WebP here";
  $("dz-sub").textContent = "or click to browse";
  $("file-input").value = "";
  if (previewUrl) {
    URL.revokeObjectURL(previewUrl);
    previewUrl = null;
  }
  fileBytes = null;
  fileName = null;
  art = null;
  clearTimeout(convertTimer);
  $("btn-play").disabled = true;
  $("btn-dl-gif").disabled = true;
  $("btn-dl-txt").disabled = true;
  $("timeline").hidden = true;
  $("view-toggle").hidden = true;
  setStatus("Waiting for an image…");
}

function showDragOverlay() {
  $("drag-overlay").hidden = false;
}

function hideDragOverlay() {
  $("drag-overlay").hidden = true;
}

/* ---------- controls ---------- */

$("ctl-width").addEventListener("input", (e) => {
  const v = Number(e.target.value) || 0;
  $("out-width").textContent = v === 0 ? "auto" : String(v);
  scheduleConvert();
});
$("ctl-brightness").addEventListener("input", (e) => {
  $("out-brightness").textContent = Number(e.target.value).toFixed(2);
  scheduleConvert();
});
$("ctl-contrast").addEventListener("input", (e) => {
  $("out-contrast").textContent = Number(e.target.value).toFixed(2);
  scheduleConvert();
});

["ctl-charset", "ctl-invert", "ctl-bw", "ctl-repeat"].forEach((id) => {
  $(id).addEventListener("input", scheduleConvert);
});
["ctl-bg"].forEach((id) => {
  $(id).addEventListener("change", scheduleConvert);
});

let convertTimer = null;
function scheduleConvert() {
  clearTimeout(convertTimer);
  if (!fileBytes) return;
  convertTimer = setTimeout(() => {
    convert();
    convertTimer = null;
  }, 200);
}
$("ctl-speed").addEventListener("change", (e) => {
  speed = Number(e.target.value);
  if (playing) scheduleNext();
});
$("btn-play").addEventListener("click", togglePlay);
$("btn-dl-gif").addEventListener("click", () => download("gif"));
$("btn-dl-txt").addEventListener("click", () => download("txt"));
$("tab-text").addEventListener("click", () => setView("text"));
$("tab-gif").addEventListener("click", () => setView("gif"));
$("ctl-frame").addEventListener("input", (e) => {
  idx = Number(e.target.value);
  stopPlayback();
  showFrame(idx);
});

document.addEventListener("keydown", (e) => {
  if (e.code === "Space" && !["INPUT", "SELECT", "TEXTAREA"].includes(document.activeElement.tagName)) {
    e.preventDefault();
    if (art) togglePlay();
  }
});

window.addEventListener("unhandledrejection", (e) => {
  console.warn("rustisay:", e.reason);
});

function currentOptions() {
  return {
    width: Math.max(0, parseInt($("ctl-width").value, 10) || 0),
    no_color: $("ctl-bw").checked,
    invert: $("ctl-invert").checked,
    brightness: Number($("ctl-brightness").value),
    contrast: Number($("ctl-contrast").value),
    bg_color: $("ctl-bg").value.trim() || "black",
    repeat: Math.min(65535, Math.max(0, parseInt($("ctl-repeat").value, 10) || 0)),
    charset: $("ctl-charset").value,
  };
}

/* ---------- conversion ---------- */

function convert() {
  if (!fileBytes) return;
  stopPlayback();
  hideError();
  hideStage();
  setStatus("Converting…");
  const id = ++reqId;
  convertStarted = performance.now();
  worker.postMessage({ id, bytes: fileBytes, filename: fileName, opts: currentOptions() });
}

function onConvertResult(res) {
  if (res.id !== reqId) return;
  if (!res.ok) {
    hideStage();
    showError(String(res.error));
    setStatus("Conversion failed");
    return;
  }
  try {
    const ms = (performance.now() - convertStarted).toFixed(0);
    art = res;
    $("ctl-frame").max = art.frames - 1;
    $("ctl-frame").value = 0;
    $("timeline").hidden = art.frames < 2;
    $("view-toggle").hidden = false;
    $("btn-play").disabled = false;
    $("btn-dl-gif").disabled = false;
    $("btn-dl-txt").disabled = false;
    $("gif-preview").src = gifUrl();
    setStatus(
      `Converted ${fileName} — ${art.frames} frame${art.frames === 1 ? "" : "s"}, ` +
      `${art.width}×${art.height} chars, ${ms} ms`
    );
    idx = 0;
    setView("gif");
    $("frame-count").textContent = `${1} / ${art.frames}`;
  } catch (err) {
    hideStage();
    showError(String(err));
    setStatus("Conversion failed");
  }
}

/* ---------- playback ---------- */

function gifUrl() {
  return URL.createObjectURL(new Blob([art.gif], { type: "image/gif" }));
}

function setView(v) {
  view = v;
  $("tab-text").classList.toggle("active", v === "text");
  $("tab-gif").classList.toggle("active", v === "gif");
  const text = v === "text";
  $("pre").hidden = !text;
  $("gif-preview").hidden = text;
}

function showFrame(i) {
  const src = art.text_frames_color[i] || art.text_frames[i];
  $("pre").innerHTML = ansiToHtml(src);
  $("frame-count").textContent = `${i + 1} / ${art.frames}`;
}

function play() {
  if (!art || art.frames < 2 || playing) return;
  if (view !== "text") setView("text");
  playing = true;
  $("btn-play").textContent = "Pause";
  scheduleNext();
}

function scheduleNext() {
  if (!playing) return;
  showFrame(idx);
  idx = (idx + 1) % art.frames;
  $("ctl-frame").value = idx;
  timer = setTimeout(scheduleNext, Math.max(10, art.delays_ms[idx] / speed));
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

/* ---------- ANSI truecolor → HTML ---------- */

const ANSI_RE = /\x1b\[38;2;(\d+);(\d+);(\d+)m|\x1b\[0m/g;

function escapeHtml(s) {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function ansiToHtml(s) {
  let out = "";
  let last = 0;
  let open = false;
  let m;
  ANSI_RE.lastIndex = 0;
  while ((m = ANSI_RE.exec(s)) !== null) {
    const text = escapeHtml(s.slice(last, m.index));
    if (m[1] !== undefined) {
      out += (open ? "</span>" : "") +
        `<span style="color:rgb(${m[1]},${m[2]},${m[3]})">${text}`;
      open = true;
    } else {
      out += (open ? "</span>" : "") + text;
      open = false;
    }
    last = m.index + m[0].length;
  }
  out += (open ? "</span>" : "") + escapeHtml(s.slice(last));
  return out;
}

/* ---------- downloads / status ---------- */

function download(kind) {
  if (!art) return;
  const blob = kind === "gif"
    ? new Blob([art.gif], { type: "image/gif" })
    : new Blob([art.text_frames.join("\f")], { type: "text/plain" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = fileName.replace(/\.[^.]+$/, "") + "-ascii." + (kind === "gif" ? "gif" : "txt");
  a.click();
  URL.revokeObjectURL(url);
}

function setStatus(text) {
  $("status").hidden = false;
  $("stat-text").textContent = text;
}

function hideError() {
  $("error").hidden = true;
}

function showError(text) {
  $("error").textContent = text;
  $("error").hidden = false;
}

function hideStage() {
  $("pre").hidden = true;
  $("gif-preview").hidden = true;
}