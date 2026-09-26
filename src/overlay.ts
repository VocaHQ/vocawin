import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./overlay.css";

type Payload = { phase: string; title: string; detail: string };

const pill = document.querySelector<HTMLDivElement>("#pill")!;
const MIC = `<svg viewBox="0 0 16 16" aria-hidden="true"><rect x="5" y="1.5" width="6" height="9" rx="3" fill="#F2F6F2"/><path d="M3 8a5 5 0 0 0 10 0M8 13v2" stroke="#F2F6F2" stroke-width="1.6" fill="none" stroke-linecap="round"/></svg>`;
const WARN = `<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M8 2 15 14H1z" fill="none" stroke="#F2F6F2" stroke-width="1.6" stroke-linejoin="round"/><path d="M8 6.5v3.5M8 12v.5" stroke="#F2F6F2" stroke-width="1.6" stroke-linecap="round"/></svg>`;
const BARS = 9;
/** Transparent room either side of the pill for its shadow. */
const SHADOW_ROOM = 28;
let levelTimer: number | null = null;
let smoothed = 0;

const escape = (value: string) => value.replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]!));

function stopLevels() {
  if (levelTimer !== null) window.clearInterval(levelTimer);
  levelTimer = null;
  smoothed = 0;
}

/** Bars follow the live microphone level while listening. */
function startLevels() {
  stopLevels();
  const bars = Array.from(pill.querySelectorAll<HTMLElement>(".bars i"));
  levelTimer = window.setInterval(async () => {
    let level = 0;
    try { level = await invoke<number>("get_mic_level"); } catch { /* window closing */ }
    smoothed = Math.max(level, smoothed * 0.8);
    const loudness = Math.min(1, smoothed * 4);
    bars.forEach((bar, index) => {
      const centre = 1 - Math.abs(index - (BARS - 1) / 2) / BARS;
      const jitter = 0.65 + Math.random() * 0.35;
      bar.style.height = `${Math.round(4 + 18 * loudness * centre * jitter)}px`;
    });
  }, 60);
}

function render(payload: Payload) {
  stopLevels();
  pill.className = `pill ${payload.phase}`;
  switch (payload.phase) {
    case "hidden":
      pill.hidden = true;
      pill.innerHTML = "";
      return;
    case "listening":
      pill.innerHTML = `<span class="live-dot"></span><span class="bars">${"<i></i>".repeat(BARS)}</span>${payload.detail ? `<span class="hint">${escape(payload.detail)}</span>` : ""}`;
      break;
    case "processing":
      pill.innerHTML = `<span class="spinner"></span><strong>${escape(payload.title)}</strong>`;
      break;
    case "error":
      pill.innerHTML = `<span class="mark">${WARN}</span><strong>${escape(payload.title)}</strong>${payload.detail ? `<span class="detail">${escape(payload.detail)}</span>` : ""}`;
      break;
    default:
      pill.innerHTML = `<span class="mark">${MIC}</span><strong>${escape(payload.title)}</strong>${payload.detail ? `<span class="detail">· ${escape(payload.detail)}</span>` : ""}`;
  }
  pill.hidden = false;
  pill.setAttribute("aria-label", [payload.title, payload.detail].filter(Boolean).join(". "));
  if (payload.phase === "listening") startLevels();
  // Fit the window to the pill so it covers nothing else on screen.
  requestAnimationFrame(() => {
    const width = Math.ceil(pill.getBoundingClientRect().width) + SHADOW_ROOM;
    void invoke("set_overlay_width", { width }).catch(() => undefined);
  });
}

/** Live preview: the latest words replace the hint while listening. The
 *  span clips from the left, so the newest words stay in view. */
function showLive(text: string) {
  if (!pill.classList.contains("listening")) return;
  pill.querySelector(".hint")?.remove();
  let live = pill.querySelector<HTMLSpanElement>(".live");
  if (!live) {
    live = document.createElement("span");
    live.className = "live";
    pill.append(live);
  }
  live.textContent = text;
  pill.setAttribute("aria-label", `Listening. ${text}`);
  requestAnimationFrame(() => {
    const width = Math.ceil(pill.getBoundingClientRect().width) + SHADOW_ROOM;
    void invoke("set_overlay_width", { width }).catch(() => undefined);
  });
}

pill.addEventListener("click", () => {
  void invoke("dismiss_overlay").catch(() => undefined);
});

listen<Payload>("overlay-phase", event => render(event.payload)).catch(() => undefined);
listen<string>("overlay-live", event => showLive(event.payload)).catch(() => undefined);
invoke<Payload>("get_overlay_phase").then(render).catch(() => undefined);
