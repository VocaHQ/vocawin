import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./style.css";
import sidebarMark from "./assets/voca-logo.svg?raw";
import dictateIdle from "./assets/vocawin-dictate-idle.svg?raw";
import dictateListening from "./assets/vocawin-dictate-listening.svg?raw";
import familyLogo from "../web/assets/brand/voca-logo-512.png";
import hqMarkRaw from "../web/assets/brand/vocahq/voca-mark.svg?raw";
import linuxMarkRaw from "../web/assets/brand/promo/cards/platform/linux.svg?raw";
import appleMarkRaw from "../web/assets/brand/promo/cards/platform/apple.svg?raw";
import androidMarkRaw from "../web/assets/brand/promo/cards/platform/android.svg?raw";
import gatewayMarkRaw from "../web/assets/brand/vocagateway/vocagateway-1u.svg?raw";
import discordMark from "../web/assets/brand/vocahq/social/discord.svg?raw";
import githubMark from "../web/assets/brand/vocahq/social/github.svg?raw";
import mailMark from "../web/assets/brand/vocahq/social/mail.svg?raw";
import xMark from "../web/assets/brand/vocahq/social/x.svg?raw";

type Model = { id: string; name: string; engine: string; size: string; languages: string; acceleration: string; description: string };
type Settings = {
  hotkey: string;
  activationMode: string;
  language: string;
  silenceSeconds: number;
  maxRecordingSeconds: number;
  launchAtLogin: boolean;
  soundEffects: boolean;
  soundTheme: string;
  appendTrailingSpace: boolean;
  autoCapitalize: boolean;
  selectedModel: string;
  inputDevice: string;
  autoPauseEnabled: boolean;
  autoPauseApps: string;
  idleUnloadEnabled: boolean;
  idleUnloadSeconds: number;
  welcomeDismissed: boolean;
  historyEnabled: boolean;
  debugLogging: boolean;
  customVocabulary: string;
  copyToClipboard: boolean;
  cleanupLevel: string;
  numbersAsDigits: boolean;
  numberSymbols: boolean;
  spokenEmoji: boolean;
  replacements: Replacement[];
  snippets: Snippet[];
  escapeCancels: boolean;
  handsFreeHotkey: string;
  pasteLastHotkey: string;
  mouseButton: string;
  overlayStyle: string;
  overlayPosition: string;
  readyPill: boolean;
  skipSilence: boolean;
  muteOtherAudio: boolean;
  historyRetentionDays: number;
  historyKeepAudio: boolean;
  insertionMode: string;
  pasteApps: string;
};
type Replacement = { heard: string; replacement: string };
type Snippet = { trigger: string; expansion: string };
type View = "dictation" | "shortcuts" | "models" | "audio" | "formatting" | "dictionary" | "snippets" | "history" | "stats" | "settings" | "power" | "debug" | "about";
type HistoryEntry = {
  id: number;
  text: string;
  modelId: string;
  createdAtMs: number;
  audioFile?: string;
  durationMs?: number;
  status?: string;
  error?: string;
};
type StatsSummary = {
  dictations: number;
  words: number;
  characters: number;
  audioMinutes: number;
  wordsPerMinute: number;
  timeSavedMinutes: number;
  wordsToday: number;
  wordsThisWeek: number;
  currentStreak: number;
  longestStreak: number;
  activeDays: number;
  recent: Array<{ day: string; words: number }>;
};
/** Which shortcut the Record button is capturing. */
type CaptureTarget = "hotkey" | "handsFreeHotkey" | "pasteLastHotkey";
type ModelStatus = { installed: boolean; downloadable: boolean; downloading: boolean; progress: number; message?: string; bytesOnDisk?: number };
type HotkeyPreset = { id: string; label: string };
type GpuStatus = {
  available: boolean;
  name: string;
  backend: string;
  detail: string;
  deviceIndex: number;
  discrete: boolean;
  vramMb: number;
};
type InputDevice = { name: string; isDefault: boolean };
type ModelRecommendation = { modelId: string; modelName: string; reason: string; vramMb: number; gpu: GpuStatus };
type RuntimeStatus = {
  status: string;
  recording: boolean;
  paused: boolean;
  modelLoaded: boolean;
  parkKind: string;
  parkDetail: string;
  hotkey: string;
  inputDevice: string;
  gpuName: string;
  gpuBackend: string;
  gpuDetail?: string;
};
type LogLine = { level: string; text: string };
type DebugReport = {
  version: string;
  os: string;
  cpu: string;
  ram: string;
  gpu: GpuStatus;
  debugLogging: boolean;
  text: string;
};
type RunningApp = { name: string; label: string };
type EngineFilter = "all" | "whisper" | "onnx";
type LanguageFilter = "any" | "english" | "multilingual";

/** One searchable settings row. `page` is the sidebar page it lives on;
 *  `card` groups rows inside that page. */
type SettingsItem = {
  page: View;
  card: string;
  title: string;
  subtitle: string;
  keywords: string;
  html: string;
  /** Control sits under the text instead of beside it. */
  wide?: boolean;
  /** Extra markup under the row (the watched-app chips). */
  after?: string;
};

/** Sidebar sections, in VocaMac's order: Dictation, Writing, Activity, App. */
const NAV_SECTIONS: Array<{ label: string; items: Array<[View, string, string]> }> = [
  { label: "Dictation", items: [["dictation", "Dictation", "◉"], ["shortcuts", "Shortcuts", "⌨"], ["models", "Models", "◇"], ["audio", "Audio", "♪"]] },
  { label: "Writing", items: [["formatting", "Formatting", "¶"], ["dictionary", "Dictionary", "✎"], ["snippets", "Snippets", "⧉"]] },
  { label: "Activity", items: [["history", "History", "≡"], ["stats", "Stats", "▦"]] },
  { label: "App", items: [["settings", "General", "⚙"], ["power", "Power", "⏻"], ["debug", "Debug", "⌗"], ["about", "About", "ⓘ"]] },
];
const ALL_VIEWS: View[] = NAV_SECTIONS.flatMap(section => section.items.map(([id]) => id));

/** Shortcut choices for hands-free and paste-last. Lone modifiers are hold
 *  keys, so they are left to the main hotkey. */
const EXTRA_SHORTCUTS: Array<[string, string]> = [
  ["", "Off"],
  ["F7", "F7"],
  ["F8", "F8"],
  ["F9", "F9"],
  ["F10", "F10"],
  ["Ctrl+Alt+Space", "Ctrl+Alt+Space"],
  ["Ctrl+Shift+Space", "Ctrl+Shift+Space"],
  ["Ctrl+Alt+V", "Ctrl+Alt+V"],
  ["Ctrl+Shift+Alt+V", "Ctrl+Shift+Alt+V"],
];
const MOUSE_BUTTONS: Array<[string, string]> = [
  ["", "Off"],
  ["middle", "Middle button"],
  ["x1", "Back side button"],
  ["x2", "Forward side button"],
];
const RETENTION_CHOICES: Array<[number, string]> = [
  [1, "1 day"],
  [7, "7 days"],
  [30, "30 days"],
  [0, "Forever"],
];

const ENGINE_FILTERS: Array<[EngineFilter, string]> = [
  ["all", "All engines"],
  ["whisper", "Whisper"],
  ["onnx", "ONNX"],
];
const LANGUAGE_FILTERS: Array<[LanguageFilter, string]> = [
  ["any", "Any language"],
  ["english", "English"],
  ["multilingual", "Multilingual"],
];
const IDLE_PRESETS: Array<[number, string]> = [
  [0, "Never"],
  [300, "5 minutes"],
  [900, "15 minutes"],
  [1800, "30 minutes"],
  [3600, "1 hour"],
];

const SOUND_THEMES: Array<[string, string]> = [
  ["lift", "Lift"],
  ["flick", "Flick"],
  ["ember", "Ember"],
  ["step", "Step"],
  ["voca", "Voca"],
  ["soft", "Soft"],
  ["chirp", "Chirp"],
  ["scale", "Scale"],
  ["drop", "Drop"],
  ["glass", "Glass"],
  ["off", "Off"],
];

const LANGUAGE_CORE = [
  "Spanish",
  "French",
  "German",
  "Italian",
  "Portuguese",
  "Dutch",
  "Russian",
  "Japanese",
  "Chinese",
  "Korean",
  "Arabic",
  "Hindi",
  "Turkish",
  "Polish",
  "Ukrainian",
  "Swedish",
  "Norwegian",
  "Danish",
  "Finnish",
  "Czech",
  "Greek",
  "Hebrew",
  "Indonesian",
  "Vietnamese",
  "Thai",
  "Romanian",
  "Hungarian",
  "Catalan",
];

const LANGUAGE_CHOICES = ["Auto-detect", "English", ...[...LANGUAGE_CORE].sort((a, b) => a.localeCompare(b))];

const ICON_DOWNLOAD = `<svg viewBox="0 0 16 16" aria-hidden="true"><path fill="currentColor" d="M8 1.5a.75.75 0 0 1 .75.75v6.19l1.72-1.72a.75.75 0 1 1 1.06 1.06l-3 3a.75.75 0 0 1-1.06 0l-3-3a.75.75 0 0 1 1.06-1.06l1.72 1.72V2.25A.75.75 0 0 1 8 1.5Zm-4.5 9a.75.75 0 0 1 .75.75v1.5h7.5v-1.5a.75.75 0 0 1 1.5 0v2.25c0 .41-.34.75-.75.75h-9a.75.75 0 0 1-.75-.75V11.25A.75.75 0 0 1 3.5 10.5Z"/></svg>`;
const ICON_TRASH = `<svg viewBox="0 0 16 16" aria-hidden="true"><path fill="currentColor" d="M6.2 1.75A1.25 1.25 0 0 1 7.4.75h1.2c.55 0 1.03.36 1.2.88l.2.62h2.7a.75.75 0 0 1 0 1.5h-.3l-.55 8.08A1.75 1.75 0 0 1 10.11 13.5H5.89A1.75 1.75 0 0 1 4.15 11.83L3.6 3.75h-.35a.75.75 0 0 1 0-1.5h2.7l.2-.62c.17-.52.65-.88 1.2-.88Zm.55 1.5-.1.37h2.7l-.1-.37-.05-.13H6.8l-.05.13ZM5.1 3.75l.54 8.02a.25.25 0 0 0 .25.23h4.22a.25.25 0 0 0 .25-.23l.54-8.02H5.1Z"/></svg>`;
const ICON_CHECK = `<svg viewBox="0 0 16 16" aria-hidden="true"><path fill="currentColor" d="M6.4 10.3 3.85 7.74a.75.75 0 0 0-1.06 1.06l3.1 3.1a.75.75 0 0 0 1.08-.02l6.2-6.6A.75.75 0 0 0 12.1 4.2l-5.7 6.1Z"/></svg>`;
const ICON_PLAY = `<svg viewBox="0 0 16 16" aria-hidden="true"><path fill="#0F6B57" stroke="#0F6B57" stroke-width="1" stroke-linejoin="round" d="M4.2 2.4v11.2L13.6 8z"/></svg>`;
const ICON_STOP = `<svg viewBox="0 0 16 16" aria-hidden="true"><path fill="#0F6B57" stroke="#0F6B57" stroke-width="1" d="M4 4h8v8H4z"/></svg>`;

function themeBrandSvg(svg: string, opts: { dropPlate?: boolean } = {}) {
  let out = svg;
  if (opts.dropPlate) {
    out = out.replace(/<rect width="1024" height="1024"[^>]*\/?>\s*/i, "");
    out = out.replace(/viewBox="0 0 1024 1024"/i, 'viewBox="80 350 864 330"');
  }
  // Kit marks bake HQ ink (#0B1A15), plate green (#0F6B57), and paper (#F2F6F2).
  // Swap those fills so About follows the row's currentColor on light and dark.
  out = out
    .replace(/fill="#0[Bb]1[Aa]15"/g, 'fill="currentColor"')
    .replace(/stroke="#0[Bb]1[Aa]15"/g, 'stroke="currentColor"')
    .replace(/fill="#0[Ff]6[Bb]57"/g, 'fill="currentColor"')
    .replace(/stroke="#0[Ff]6[Bb]57"/g, 'stroke="currentColor"')
    .replace(/fill="#F2F6F2"/gi, 'fill="currentColor" fill-opacity="0.16"')
    .replace(/stroke="#F2F6F2"/gi, 'stroke="currentColor"');
  const openTagEnd = out.indexOf(">");
  const openTag = openTagEnd >= 0 ? out.slice(0, openTagEnd) : out;
  if (!/\sfill=/i.test(openTag)) {
    out = out.replace(/<svg\b/i, '<svg fill="currentColor"');
  }
  if (!/aria-hidden=/i.test(out)) {
    out = out.replace(/<svg\b/i, '<svg aria-hidden="true"');
  }
  return out;
}

const hqMark = themeBrandSvg(hqMarkRaw);
const linuxMark = themeBrandSvg(linuxMarkRaw);
const appleMark = themeBrandSvg(appleMarkRaw);
const androidMark = themeBrandSvg(androidMarkRaw);
const gatewayMark = themeBrandSvg(gatewayMarkRaw, { dropPlate: true });

const app = document.querySelector<HTMLDivElement>("#app")!;
let models: Model[] = [];
let statuses: Record<string, ModelStatus> = {};
let history: HistoryEntry[] = [];
let historyQuery = "";
let stats: StatsSummary | null = null;
let retrying = new Set<number>();
let playingId: number | null = null;
let tryText = "";
let tryResult = "";
/** Page to return to when the sidebar search is cleared. */
let pageBeforeSearch: View | null = null;
let onboardingStep = 0;
let onboardingModel = "";
let onboardingTryText = "";
let settings: Settings;
let presets: HotkeyPreset[] = [];
let gpu: GpuStatus = {
  available: false,
  name: "Checking…",
  backend: "CPU",
  detail: "",
  deviceIndex: -1,
  discrete: false,
  vramMb: 0,
};
let devices: InputDevice[] = [];
let recommendation: ModelRecommendation | null = null;
let runtime: RuntimeStatus = {
  status: "Ready",
  recording: false,
  paused: false,
  modelLoaded: false,
  parkKind: "",
  parkDetail: "",
  hotkey: "",
  inputDevice: "Default microphone",
  gpuName: "",
  gpuBackend: "",
};
let recording = false;
let recordingHotkey: CaptureTarget | null = null;
let testingDictation = false;
let testListening = false;
let testResult = "";
let micTesting = false;
let micLevel = 0;
let micMeterTimer: number | null = null;
let settingsQuery = "";
let modelQuery = "";
let engineFilter: EngineFilter = "all";
let languageFilter: LanguageFilter = "any";
let view: View = "dictation";
let toastText = "";
let toastTimer: number | null = null;
let logLines: LogLine[] = [];
let debugReport: DebugReport | null = null;
let previewStartNext = true;
let runningApps: RunningApp[] = [];
let focusRestore: { id: string; start: number; end: number } | null = null;
let paneScroll = 0;
let resetPaneScroll = false;
let micPeak = 0;

const escape = (value: string) => value.replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]!));
const selected = () => models.find(model => model.id === settings.selectedModel);
const modelInstalled = () => !!statuses[settings.selectedModel]?.installed;
const formatBytes = (bytes?: number) => {
  if (!bytes) return "";
  if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} KB on disk`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(bytes >= 100 * 1024 * 1024 ? 0 : 1)} MB on disk`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB on disk`;
};
const nav = (id: View, label: string, icon: string, count = 0) => `<button class="nav ${view === id ? "active" : ""}" data-view="${id}"><span class="nav-icon">${icon}</span>${label}${count ? `<span class="nav-count" aria-label="${count} matching settings">${count}</span>` : ""}</button>`;

function emptySpeechMessage() {
  return modelInstalled()
    ? "No speech was recognized."
    : "Install a speech model first (Models tab).";
}

function modelIsEnglishOnly(model: Model) {
  return model.languages.trim().toLowerCase() === "english";
}

function matchesEngine(model: Model, filter: EngineFilter) {
  if (filter === "all") return true;
  if (filter === "whisper") return model.engine === "whisper.cpp";
  return model.engine === "ONNX Runtime";
}

function matchesLanguage(model: Model, filter: LanguageFilter) {
  if (filter === "any") return true;
  if (filter === "english") return modelIsEnglishOnly(model);
  return !modelIsEnglishOnly(model);
}

function matchesModelQuery(model: Model, query: string) {
  if (!query) return true;
  const hay = `${model.name} ${model.id} ${model.engine} ${model.languages} ${model.description}`.toLowerCase();
  return hay.includes(query);
}

function filteredModels() {
  const query = modelQuery.trim().toLowerCase();
  return models.filter(model =>
    matchesEngine(model, engineFilter)
    && matchesLanguage(model, languageFilter)
    && matchesModelQuery(model, query)
  );
}

function paintToast(text: string) {
  toastText = text;
  const node = document.querySelector<HTMLElement>("#sidebar-toast");
  if (!node) return;
  if (!text) {
    node.hidden = true;
    node.textContent = "";
    return;
  }
  node.hidden = false;
  node.textContent = text;
}

function sidebarStatusLabel() {
  if (recording) return "Listening";
  if (testingDictation) return "Testing…";
  if (runtime.parkKind === "idle") return "Unloaded";
  if (runtime.paused || runtime.parkKind === "autopause") return "Paused";
  return "Ready";
}

function paintSidebarStatus() {
  const node = document.querySelector(".sidebar-status");
  if (!node) return;
  const parked = !!runtime.parkKind && !recording;
  node.classList.toggle("parked", parked);
  node.classList.toggle("live", recording);
  const strong = node.querySelector("strong");
  if (strong) strong.textContent = sidebarStatusLabel();
  const small = node.querySelector("small");
  if (small) {
    small.textContent = runtime.parkDetail && !recording
      ? runtime.parkDetail
      : runtime.inputDevice;
  }
  const test = document.querySelector<HTMLButtonElement>("#test-dictation");
  if (test) test.disabled = runtime.paused || micTesting;
}

function sessionChromeChanged(before: { recording: boolean; parked: boolean; status: string; parkKind: string; paused: boolean }) {
  const parked = !!runtime.parkKind && !recording;
  return before.recording !== recording
    || before.parked !== parked
    || before.status !== runtime.status
    || before.parkKind !== runtime.parkKind
    || before.paused !== runtime.paused;
}

function applyRuntimeStatus(payload: RuntimeStatus | string) {
  const before = {
    recording,
    parked: !!runtime.parkKind && !recording,
    status: runtime.status,
    parkKind: runtime.parkKind,
    paused: runtime.paused,
  };
  if (typeof payload === "string") {
    runtime = { ...runtime, status: payload, paused: payload === "Paused" };
  } else {
    runtime = { ...runtime, ...payload };
  }
  recording = !!runtime.recording;
  if (!runtime.recording) testListening = false;
  if (sessionChromeChanged(before)) {
    render();
    return;
  }
  paintSidebarStatus();
  paintToast(toastText);
}

function showToast(text: string) {
  if (toastTimer) window.clearTimeout(toastTimer);
  paintToast(text);
  toastTimer = window.setTimeout(() => {
    toastTimer = null;
    paintToast("");
  }, 1800);
}

function visibleLogs() {
  if (settings?.debugLogging) return logLines;
  return logLines.filter(line => line.level === "warn" || line.level === "error");
}

function watchedApps() {
  return settings.autoPauseApps
    .split(/[\n,;]+/)
    .map(part => part.trim())
    .filter(Boolean);
}

function iconButton(action: string, id: string, label: string, icon: string, extra = "") {
  return `<button type="button" class="icon-button ${extra}" data-${action}="${escape(id)}" title="${escape(label)}" aria-label="${escape(label)}">${icon}</button>`;
}

function modelAction(model: Model, status?: ModelStatus) {
  if (status?.downloading) {
    return `<span class="model-progress">Downloading ${status.progress}%</span>`;
  }
  if (status?.installed) {
    return iconButton("delete", model.id, "Remove model", ICON_TRASH, "danger");
  }
  return iconButton("download", model.id, "Download model", ICON_DOWNLOAD, "brand");
}

function modelCards() {
  const list = filteredModels();
  if (!list.length) {
    return `<div class="empty-history">No models match. Clear the search or a filter.</div>`;
  }
  return list.map(model => {
    const status = statuses[model.id];
    const failed = !status?.installed && !status?.downloading && !!status?.message
      && status.message !== "Installed";
    const state = status?.downloading
      ? `Downloading ${status.progress}%`
      : status?.installed
        ? status.bytesOnDisk
          ? `Downloaded · ${formatBytes(status.bytesOnDisk)}`
          : "Downloaded"
        : failed
          ? `Failed: ${status?.message}`
          : "Not on this PC";
    const isSelected = model.id === settings.selectedModel;
    const recommended = recommendation?.modelId === model.id;
    return `<article class="model-card compact ${isSelected ? "selected" : ""}" data-model="${model.id}">
      <label class="model-activate">
        <input type="checkbox" data-activate="${model.id}" ${isSelected ? "checked" : ""} />
        <span class="model-check" aria-hidden="true">${isSelected ? ICON_CHECK : ""}</span>
      </label>
      <div class="model-copy">
        <b>${escape(model.name)}</b>
        <span class="engine">${escape(model.engine)}${recommended ? " · Suggested start" : ""}</span>
        <ul class="model-meta">
          <li>${escape(model.size)}</li>
          <li>${escape(model.languages)}</li>
          <li class="install-state ${failed ? "failed" : ""}">${escape(state)}</li>
        </ul>
      </div>
      <div class="model-actions">${modelAction(model, status)}</div>
    </article>`;
  }).join("");
}

function hotkeyLabel(spec = settings.hotkey) {
  const preset = presets.find(entry => entry.id === spec);
  if (preset) return preset.label;
  return spec || "your hotkey";
}

function hotkeyOptions() {
  const known = new Set(presets.map(preset => preset.id));
  const options = presets.map(preset => `<option value="${escape(preset.id)}" ${settings.hotkey === preset.id ? "selected" : ""}>${escape(preset.label)}</option>`).join("");
  const custom = known.has(settings.hotkey) ? "" : `<option value="${escape(settings.hotkey)}" selected>Custom: ${escape(settings.hotkey)}</option>`;
  return options + custom;
}

function soundThemeOptions() {
  const selectedTheme = SOUND_THEMES.some(([id]) => id === settings.soundTheme)
    ? settings.soundTheme
    : "voca";
  return SOUND_THEMES.map(([id, label]) =>
    `<option value="${id}" ${id === selectedTheme ? "selected" : ""}>${label}</option>`
  ).join("");
}

function deviceOptions() {
  const options = [`<option value="" ${settings.inputDevice === "" ? "selected" : ""}>Default microphone</option>`];
  for (const device of devices) {
    options.push(`<option value="${escape(device.name)}" ${settings.inputDevice === device.name ? "selected" : ""}>${escape(device.name)}${device.isDefault ? " (default)" : ""}</option>`);
  }
  if (settings.inputDevice && !devices.some(device => device.name === settings.inputDevice)) {
    options.push(`<option value="${escape(settings.inputDevice)}" selected>${escape(settings.inputDevice)} (saved)</option>`);
  }
  return options.join("");
}

function languageOptions() {
  const selectedLanguage = settings.language || "Auto-detect";
  const extras = selectedLanguage && !LANGUAGE_CHOICES.includes(selectedLanguage)
    ? [[selectedLanguage, selectedLanguage] as [string, string]]
    : [];
  return extras.concat(LANGUAGE_CHOICES.map(language => [language, language]));
}

function languageControl() {
  const current = settings.language || "Auto-detect";
  return `<div class="language-picker"><input id="language" class="themed-select" type="search" list="language-list" placeholder="Search the list." autocomplete="off" value="${escape(current)}" />
    ${filterDatalist("language-list", languageOptions())}</div>`;
}

function dictateMark(kind: "idle" | "listening") {
  return kind === "listening" ? dictateListening : dictateIdle;
}

function dictationParkBanner() {
  if (recording) return "";
  if (!runtime.parkKind || !runtime.parkDetail) return "";
  const label = runtime.parkKind === "idle" ? "Model unloaded" : "Dictation paused";
  return `<div class="park-banner ${runtime.parkKind}"><strong>${label}</strong><p>${escape(runtime.parkDetail)}</p></div>`;
}

function dictationPage() {
  const model = selected();
  const installed = modelInstalled();
  const holdMode = settings.activationMode !== "toggle";
  const keycap = `<kbd>${escape(hotkeyLabel())}</kbd>`;
  const parked = !!runtime.parkKind;
  const heading = recording
    ? (testListening ? "Practice take" : "Listening…")
    : runtime.parkKind === "autopause"
      ? "Paused for a watched app"
      : runtime.parkKind === "idle"
        ? "Model unloaded to save RAM"
        : "Ready when you are";
  const statusHint = recording
    ? (testListening
      ? "This take stays in VocaWin. Stop it from the sidebar Test control."
      : `Speak now. Text lands at the caret when you finish.${settings.escapeCancels ? " Esc cancels." : ""}`)
    : "Text lands at the caret in the focused app.";
  const hotkeyLabelText = recording
    ? (testListening ? "PRACTICE" : "LISTENING")
    : (holdMode ? "HOLD TO TALK" : "TAP TO TOGGLE");
  const hotkeyHint = recording
    ? (testListening
      ? "Use Stop test in the sidebar. This take does not type into other apps."
      : holdMode
        ? "Release it when you finish speaking."
        : "Tap it again to stop and type.")
    : (holdMode
      ? "Hold it in any app."
      : "Tap it in any app to start, tap again to finish.");
  // Escape hatch for hotkey/window takes only. Sidebar Test uses its own Stop
  // and must not call inject_text via #record.
  const finishControl = recording && !testListening
    ? `<button type="button" class="quiet-button" id="record" ${runtime.paused ? "disabled" : ""}>Stop &amp; type</button>`
    : "";
  const modelState = installed
    ? `${escape(model?.engine ?? "")} · ${escape(model?.languages ?? "")} · On this PC`
    : "Not on this PC yet. Open Models to download it.";
  const modelAction = installed ? "Change model" : "Download this model";
  const stateLabel = recording ? "Listening" : parked ? (runtime.parkKind === "idle" ? "Unloaded" : "Paused") : "Ready";
  return `<header><div><p class="overline">VOICE DICTATION</p><h1>Speak naturally.<br><em>Keep it private.</em></h1><p class="lede">VocaWin turns your voice into text on your own computer, never in the cloud.</p></div><span class="state ${parked ? "parked" : ""}"><i></i>${stateLabel}</span></header>
  ${dictationParkBanner()}
  <div class="dictation-bento">
    <section class="record-panel" aria-live="polite">
      <div class="mic ${recording ? "listening" : parked ? "parked" : ""}">${dictateMark(recording ? "listening" : "idle")}</div>
      <div class="record-copy">
        <h2>${heading}</h2>
        <p class="record-hint">${statusHint}</p>
      </div>
    </section>
    <section class="hotkey-panel">
      <p class="card-label">${hotkeyLabelText}</p>
      <p class="hotkey-hero">${keycap}</p>
      <p class="record-hint">${hotkeyHint}</p>
      <p class="record-actions"><button type="button" class="text-button" data-go="settings">Change shortcut</button>${finishControl ? ` · ${finishControl}` : ""}</p>
      <small>Practice via sidebar Test. That take stays in VocaWin.</small>
    </section>
  </div>
  <section class="overview"><button class="info-card model-cta ${installed ? "" : "needs-download"}" data-go="models" type="button"><p class="card-label">ACTIVE MODEL</p><strong>${escape(model?.name ?? "Choose a model")}</strong><span>${modelState}</span><span class="text-button">${modelAction}</span></button></section>`;
}

function filterLabel<T extends string>(options: Array<[T, string]>, value: T) {
  return options.find(([id]) => id === value)?.[1] ?? options[0][1];
}

function filterDatalist(id: string, options: Array<[string, string]>) {
  return `<datalist id="${id}">${options.map(([, label]) => `<option value="${escape(label)}"></option>`).join("")}</datalist>`;
}

function modelsPage() {
  const tip = recommendation
    ? `<p class="hw-tip"><strong>Starting size:</strong> ${escape(recommendation.modelName)}. ${escape(recommendation.reason)}</p>`
    : "";
  const header = `<header><div><p class="overline">ON-DEVICE MODELS</p><h1>Choose your <em>engine.</em></h1><p class="lede">Models stay on your PC. Pick the trade-off between speed, accuracy, and language coverage.</p></div></header>`;
  if (searchQuery()) return `${header}${settingsCards("models")}`;
  return `${header}
  ${tip}
  ${settingsCards("models")}
  <div class="model-filters">
    <input id="model-search" type="search" placeholder="Search models" value="${escape(modelQuery)}" />
    <label class="filter-combo"><span class="vh">Engine</span>
      <input id="engine-filter" type="search" list="engine-filter-list" placeholder="All engines" autocomplete="off" value="${escape(filterLabel(ENGINE_FILTERS, engineFilter))}" />
      ${filterDatalist("engine-filter-list", ENGINE_FILTERS)}
    </label>
    <label class="filter-combo"><span class="vh">Language</span>
      <input id="language-filter" type="search" list="language-filter-list" placeholder="Any language" autocomplete="off" value="${escape(filterLabel(LANGUAGE_FILTERS, languageFilter))}" />
      ${filterDatalist("language-filter-list", LANGUAGE_FILTERS)}
    </label>
  </div>
  <div class="model-grid compact">${modelCards()}</div>`;
}

function chipLabel(name: string) {
  return runningApps.find(app => app.name.toLowerCase() === name.toLowerCase())?.label
    ?? name.replace(/\.exe$/i, "");
}

function appPickerOptions(chosen: string[]) {
  const taken = new Set(chosen.map(name => name.toLowerCase()));
  const options = runningApps
    .filter(app => !taken.has(app.name.toLowerCase()))
    .map(app => `<option value="${escape(app.name)}">${escape(app.label)}</option>`);
  if (!options.length) {
    return `<option value="">No other running apps right now</option>`;
  }
  return `<option value="">Add a running app</option>${options.join("")}`;
}

function runningAppOptions() {
  return appPickerOptions(watchedApps());
}

function appChips(apps: string[], action: string) {
  if (!apps.length) return "";
  return `<ul class="app-chips">${apps.map(name => `<li class="app-chip"><span>${escape(chipLabel(name))}</span><button type="button" data-${action}="${escape(name)}" title="Remove ${escape(chipLabel(name))}" aria-label="Remove ${escape(chipLabel(name))}">×</button></li>`).join("")}</ul>`;
}

function watchedAppChips() {
  return appChips(watchedApps(), "unwatch");
}

function pasteApps() {
  return (settings.pasteApps ?? "")
    .split(/[\n,;]+/)
    .map(part => part.trim())
    .filter(Boolean);
}

function idleUnloadValue() {
  if (!settings.idleUnloadEnabled) return 0;
  const seconds = settings.idleUnloadSeconds;
  return [300, 900, 1800, 3600].reduce((best, value) =>
    Math.abs(value - seconds) < Math.abs(best - seconds) ? value : best, 300);
}

function idleUnloadOptions() {
  const current = idleUnloadValue();
  return IDLE_PRESETS.map(([value, label]) =>
    `<option value="${value}" ${value === current ? "selected" : ""}>${label}</option>`
  ).join("");
}

function previewSoundControl() {
  const label = previewStartNext ? "Preview start" : "Preview end";
  const icon = previewStartNext ? ICON_PLAY : ICON_STOP;
  const off = settings.soundTheme === "off";
  return `<button type="button" class="quiet-button preview-sound" id="preview-sound" ${off ? "disabled " : ""}aria-label="${label}" title="${label}">${icon}</button>`;
}

function switchControl(id: string, checked: boolean, disabled = false) {
  return `<label class="switch"><input id="${id}" type="checkbox" ${checked ? "checked" : ""} ${disabled ? "disabled" : ""}/><span></span></label>`;
}

function selectControl(id: string, options: Array<[string, string]>, current: string) {
  return `<select id="${id}" class="themed-select">${options.map(([value, label]) => `<option value="${escape(value)}" ${value === current ? "selected" : ""}>${escape(label)}</option>`).join("")}</select>`;
}

/** Hands-free and paste-last: presets, plus Record for any combo. */
function extraShortcutControl(target: CaptureTarget) {
  const current = settings[target] as string;
  const options = EXTRA_SHORTCUTS.some(([value]) => value === current)
    ? EXTRA_SHORTCUTS
    : [...EXTRA_SHORTCUTS, [current, `Custom: ${current}`] as [string, string]];
  const id = target === "handsFreeHotkey" ? "hands-free-hotkey" : "paste-last-hotkey";
  return `<div class="hotkey-controls">${selectControl(id, options, current)}
    <button type="button" class="quiet-button" data-capture="${target}">${recordingHotkey === target ? "Cancel" : "Record"}</button></div>`;
}

function replacementsEditor() {
  const rows = settings.replacements.map((entry, index) => `<li class="pair-row">
      <span class="pair-from">${escape(entry.heard)}</span><span class="pair-arrow" aria-hidden="true">→</span><span class="pair-to">${escape(entry.replacement)}</span>
      <button type="button" class="pair-remove" data-remove-replacement="${index}" title="Remove" aria-label="Remove ${escape(entry.heard)}">×</button>
    </li>`).join("");
  return `<div class="pair-editor">
    ${rows ? `<ul class="pair-list">${rows}</ul>` : `<p class="pair-empty">No replacements yet.</p>`}
    <div class="pair-form">
      <input id="replacement-heard" class="draft" type="text" placeholder="Heard, e.g. get hub" autocomplete="off" />
      <input id="replacement-to" class="draft" type="text" placeholder="Type instead, e.g. GitHub" autocomplete="off" />
      <button type="button" class="quiet-button" id="add-replacement">Add</button>
    </div>
  </div>`;
}

function snippetsEditor() {
  const rows = settings.snippets.map((entry, index) => `<li class="pair-row snippet">
      <span class="pair-from">${escape(entry.trigger)}</span><span class="pair-arrow" aria-hidden="true">→</span><span class="pair-to">${escape(entry.expansion)}</span>
      <button type="button" class="pair-remove" data-remove-snippet="${index}" title="Remove" aria-label="Remove ${escape(entry.trigger)}">×</button>
    </li>`).join("");
  return `<div class="pair-editor">
    ${rows ? `<ul class="pair-list">${rows}</ul>` : `<p class="pair-empty">No snippets yet.</p>`}
    <div class="pair-form snippet">
      <input id="snippet-trigger" class="draft" type="text" placeholder="Say, e.g. my address" autocomplete="off" />
      <textarea id="snippet-expansion" class="draft" rows="3" placeholder="Type, e.g. 221B Baker Street, London"></textarea>
      <button type="button" class="quiet-button" id="add-snippet">Add</button>
    </div>
  </div>`;
}

function settingsItems(): SettingsItem[] {
  const levelPct = Math.min(100, Math.round(micLevel * 140));
  return [
    {
      page: "shortcuts",
      card: "Dictation key",
      title: "Activation hotkey",
      subtitle: "Pick a preset or press Record. New installs default to Right Alt (Option), the same hold-default as VocaLinux. AltGr (Ctrl+Right Alt) is not consumed, so layout characters still type. The live listener pauses while recording.",
      keywords: "hotkey shortcut keyboard record preset right alt altright",
      html: `<div class="hotkey-controls"><select id="hotkey-preset" class="themed-select">${hotkeyOptions()}</select>
    <button type="button" class="quiet-button" data-capture="hotkey">${recordingHotkey === "hotkey" ? "Cancel" : "Record"}</button></div>`,
    },
    {
      page: "shortcuts",
      card: "Dictation key",
      title: "Activation style",
      subtitle: "Hold to talk, or tap to start and tap again to stop. Toggle also stops after the silence set on the Audio page.",
      keywords: "push to talk toggle mode tap",
      html: `<select id="activation" class="themed-select"><option value="pushToTalk">Push to talk</option><option value="toggle">Toggle</option></select>`,
    },
    {
      page: "shortcuts",
      card: "More ways to dictate",
      title: "Hands-free shortcut",
      subtitle: "Press once to start and again to stop, without holding a key. Silence does not end it; Max recording still does.",
      keywords: "hands free handsfree toggle shortcut start stop long dictation",
      html: extraShortcutControl("handsFreeHotkey"),
    },
    {
      page: "shortcuts",
      card: "More ways to dictate",
      title: "Mouse button",
      subtitle: "Dictate with the middle or a side mouse button, held or tapped like the hotkey. That button's normal click goes to VocaWin while this is on.",
      keywords: "mouse button middle side back forward xbutton",
      html: selectControl("mouse-button", MOUSE_BUTTONS, settings.mouseButton),
    },
    {
      page: "shortcuts",
      card: "More ways to dictate",
      title: "Escape cancels dictation",
      subtitle: "Escape throws a take away while it records, or keeps a transcription from being typed. At other times Escape reaches your apps as usual.",
      keywords: "escape esc cancel discard throw away",
      html: switchControl("escape-cancels", settings.escapeCancels),
    },
    {
      page: "shortcuts",
      card: "More ways to dictate",
      title: "Paste last dictation",
      subtitle: "Types your last dictation again at the caret, even with history off. Ctrl+Alt shortcuts can clash with AltGr keyboard layouts.",
      keywords: "paste last again repeat retype shortcut",
      html: extraShortcutControl("pasteLastHotkey"),
    },
    {
      page: "models",
      card: "Language",
      title: "Dictation language",
      subtitle: "One list. Auto-detect is first, then English, then A to Z.",
      keywords: "language locale english auto detect",
      html: languageControl(),
    },
    {
      page: "audio",
      card: "Microphone",
      title: "Microphone",
      subtitle: "WASAPI capture device used for dictation.",
      keywords: "mic microphone device wasapi input",
      html: `<select id="input-device" class="themed-select">${deviceOptions()}</select>`,
    },
    {
      page: "audio",
      card: "Microphone",
      title: "Mic Test",
      subtitle: "Level meter only. Does not recognize or inject text.",
      keywords: "mic test level meter volume",
      html: `<div class="mic-test"><button type="button" class="quiet-button" id="mic-test" ${recording || testListening ? "disabled" : ""}>${micTesting ? "Stop Mic Test" : "Mic Test"}</button>
      <div class="level-meter" aria-hidden="true"><span style="width:${levelPct}%"></span></div></div>`,
    },
    {
      page: "audio",
      card: "Recording",
      title: "Silence auto-stop",
      subtitle: "Seconds of quiet before a toggled take ends. Push-to-talk and the hands-free shortcut ignore this.",
      keywords: "vad silence timeout toggle",
      html: `<input id="silence" type="number" min="0.3" max="10" step="0.1" value="${settings.silenceSeconds}" />`,
    },
    {
      page: "audio",
      card: "Recording",
      title: "Max recording",
      subtitle: "Hard stop so a stuck session cannot run forever.",
      keywords: "duration limit max",
      html: `<input id="max-recording" type="number" min="3" max="300" step="1" value="${settings.maxRecordingSeconds}" />`,
    },
    {
      page: "audio",
      card: "Recording",
      title: "Skip silence before transcribing",
      subtitle: "Cuts quiet stretches so the model only hears speech. Faster, and Whisper has no silence to invent words over.",
      keywords: "silence skip trim vad quiet hallucination",
      html: switchControl("skip-silence", settings.skipSilence),
    },
    {
      page: "audio",
      card: "Recording",
      title: "Mute other audio while dictating",
      subtitle: "Mutes apps that are playing sound while you record, then unmutes exactly those. Apps you muted yourself stay muted.",
      keywords: "mute duck music video audio other apps sound",
      html: switchControl("mute-other-audio", settings.muteOtherAudio),
    },
    {
      page: "audio",
      card: "Sounds",
      title: "Dictation sounds",
      subtitle: "These play when listening starts and stops. Preview is two clicks: start tone, then end tone.",
      keywords: "sound beep audio cue",
      html: `<div class="sound-theme-controls"><select id="sound-theme" class="themed-select">${soundThemeOptions()}</select>
      ${previewSoundControl()}</div>`,
    },
    {
      page: "formatting",
      card: "Cleanup",
      title: "Cleanup",
      subtitle: "Medium removes “um” and “uh”, a letter said three times (“I I I”), and keeps only the fix when you correct a day, month, number, or time (“tomorrow, no, Wednesday” → “Wednesday”). None types every word as heard.",
      keywords: "cleanup filler um uh hesitation stutter correction",
      html: selectControl("cleanup-level", [["medium", "Medium"], ["none", "None"]], settings.cleanupLevel),
    },
    {
      page: "formatting",
      card: "Formatting",
      title: "Auto-capitalize",
      subtitle: "Capitalize the start of sentences.",
      keywords: "capitalize formatting output",
      html: switchControl("auto-cap", settings.autoCapitalize),
    },
    {
      page: "formatting",
      card: "Formatting",
      title: "Trailing space",
      subtitle: "Append a space after each utterance.",
      keywords: "space formatting output",
      html: switchControl("trailing-space", settings.appendTrailingSpace),
    },
    {
      page: "formatting",
      card: "Spoken forms",
      title: "Write numbers as digits",
      subtitle: "“twenty three” becomes “23”, “seven thirty pm” becomes “7:30 pm”, “my number is nine eight seven…” becomes digits. English only. “High five” and “no one” keep their words.",
      keywords: "numbers digits spoken number phone time year",
      html: switchControl("numbers-as-digits", settings.numbersAsDigits),
    },
    {
      page: "formatting",
      card: "Spoken forms",
      title: "Use symbols and ordinals",
      subtitle: "With digits on: “fifty percent” → “50%”, “five dollars and fifty cents” → “$5.50”, “the twenty first” → “21st”, “June twenty second” → “June 22”.",
      keywords: "percent dollar currency ordinal date symbols",
      html: switchControl("number-symbols", settings.numberSymbols, !settings.numbersAsDigits),
    },
    {
      page: "formatting",
      card: "Spoken forms",
      title: "Spoken emoji",
      subtitle: "Say “party emoji” for 🎉 or “three fire emojis” for 🔥🔥🔥. Talking about one (“send a fire emoji”) keeps the words.",
      keywords: "emoji emoticon smiley",
      html: switchControl("spoken-emoji", settings.spokenEmoji),
    },
    {
      page: "formatting",
      card: "Output",
      title: "How text goes in",
      subtitle: "Typing works in almost every app and leaves your clipboard alone. Paste is faster for long text and suits apps that drop typed characters; your clipboard is put back afterwards.",
      keywords: "type paste insert injection sendinput clipboard method",
      html: selectControl("insertion-mode", [["type", "Type at the caret"], ["paste", "Paste"]], settings.insertionMode),
    },
    {
      page: "formatting",
      card: "Output",
      title: "Always paste in these apps",
      subtitle: "For an app where typed text comes out wrong or incomplete. Notepad and WordPad always paste.",
      keywords: "paste apps dropped characters remote desktop terminal electron",
      html: `<select id="paste-app" class="themed-select power-combo">${appPickerOptions(pasteApps())}</select>`,
      after: `<div class="power-chips"><div id="paste-app-chips">${appChips(pasteApps(), "unpaste")}</div><p class="power-note">Empty list means type everywhere (unless Paste is chosen above).</p></div>`,
    },
    {
      page: "formatting",
      card: "Output",
      title: "Copy to clipboard",
      subtitle: "Leave recognized text on the clipboard after each take. Off by default so dictation does not replace what you already copied.",
      keywords: "clipboard paste preserve copy output",
      html: switchControl("copy-to-clipboard", settings.copyToClipboard),
    },
    {
      page: "dictionary",
      card: "Vocabulary",
      wide: true,
      title: "Vocabulary",
      subtitle: "Names and jargon spelled your way with every model: “voca win” becomes “VocaWin”. Whisper also gets them as a recognition hint. One per line, or comma-separated.",
      keywords: "custom vocabulary dictionary glossary names jargon initial prompt whisper spelling",
      html: `<textarea id="custom-vocabulary" rows="5" placeholder="VocaWin, kubectl, PostgreSQL, Grafana">${escape(settings.customVocabulary)}</textarea>`,
    },
    {
      page: "dictionary",
      card: "Replacements",
      wide: true,
      title: "Replacements",
      subtitle: "Type something else when a model gets a word wrong: “get hub” → “GitHub”. Separate several spoken forms with commas. Matching ignores case.",
      keywords: "replacements replace correct misheard dictionary",
      html: replacementsEditor(),
    },
    {
      page: "snippets",
      card: "Snippets",
      wide: true,
      title: "Custom snippets",
      subtitle: "Say a trigger and VocaWin types your saved text exactly as written, never re-cased: “my address” → your full address.",
      keywords: "snippets trigger expansion text shortcut template",
      html: snippetsEditor(),
    },
    {
      page: "history",
      card: "History settings",
      title: "Keep dictation history",
      subtitle: "When this is off, new takes are not added to History. Older entries stay until they expire or you clear them.",
      keywords: "history transcript save local",
      html: switchControl("history-enabled", settings.historyEnabled),
    },
    {
      page: "history",
      card: "History settings",
      title: "Keep history for",
      subtitle: "Older dictations and their audio are deleted from this PC.",
      keywords: "history retention delete days forever expire",
      html: selectControl("history-retention", RETENTION_CHOICES.map(([value, label]) => [String(value), label]), String(settings.historyRetentionDays)),
    },
    {
      page: "history",
      card: "History settings",
      title: "Keep audio for replay and retry",
      subtitle: "The last 50 takes keep their audio on this PC so you can replay them or retry with another model. It is saved before transcribing, so a crash never loses what you said.",
      keywords: "history audio replay retry recording save crash",
      html: switchControl("history-keep-audio", settings.historyKeepAudio),
    },
    {
      page: "settings",
      card: "Startup",
      title: "Launch at login",
      subtitle: "Start VocaWin with Windows for this user (starts minimized).",
      keywords: "startup autostart login",
      html: switchControl("launch-login", settings.launchAtLogin),
    },
    {
      page: "settings",
      card: "Startup",
      title: "Ready pill at launch",
      subtitle: "Shows “VocaWin is ready” on screen for a few seconds after it starts, since Windows often hides new tray icons.",
      keywords: "ready pill startup notification started running launch indicator",
      html: switchControl("ready-pill", settings.readyPill),
    },
    {
      page: "settings",
      card: "Recording overlay",
      title: "Recording overlay",
      subtitle: "A small pill with a live level while you speak and a spinner while it transcribes. It never takes focus from the app you are typing into.",
      keywords: "overlay pill indicator hud waveform recording",
      html: selectControl("overlay-style", [["minimal", "Minimal pill"], ["off", "Off"]], settings.overlayStyle),
    },
    {
      page: "settings",
      card: "Recording overlay",
      title: "Overlay position",
      subtitle: "Centered on the screen your mouse is on.",
      keywords: "overlay position top bottom screen",
      html: selectControl("overlay-position", [["bottom", "Bottom of screen"], ["top", "Top of screen"]], settings.overlayPosition),
    },
    {
      page: "settings",
      card: "Backup",
      title: "Settings backup",
      subtitle: "Export writes your preferences, dictionary, and snippets to a JSON file in Downloads. Import reads one back. No audio, history, or models.",
      keywords: "export import backup restore settings move",
      html: `<div class="button-pair"><button type="button" class="quiet-button" id="export-settings">Export</button><button type="button" class="quiet-button" id="import-settings">Import</button><input id="import-file" class="draft" type="file" accept=".json,application/json" hidden /></div>`,
    },
    {
      page: "settings",
      card: "Setup",
      title: "Setup guide",
      subtitle: "Walk through language, model, and a first dictation again.",
      keywords: "setup onboarding welcome wizard guide",
      html: `<button type="button" class="quiet-button" id="run-setup">Run setup</button>`,
    },
    {
      page: "power",
      card: "Power",
      title: "Pause while these apps are running",
      subtitle: "Voca stays quiet so they can use the mic.",
      keywords: "power pause while apps running autopause game mic",
      html: `<select id="auto-pause-app" class="themed-select power-combo">${runningAppOptions()}</select>`,
      after: `<div class="power-chips"><div id="watched-app-chips">${watchedAppChips()}</div><p class="power-note">Empty list means off. Each chip removes that app.</p></div>`,
    },
    {
      page: "power",
      card: "Power",
      title: "Unload the model after idle",
      subtitle: "Frees RAM. Next dictation loads it again.",
      keywords: "power unload model idle ram memory minutes hour",
      html: `<select id="idle-unload" class="themed-select power-combo">${idleUnloadOptions()}</select>`,
    },
  ];
}

function itemMatches(item: SettingsItem, query: string) {
  return matchScore(item, query) > 0;
}

/** 3 when every word is in the title, 2 in the title or keywords, 1 anywhere
 *  (the subtitle included), 0 for no match. Search jumps to the best page. */
function matchScore(item: SettingsItem, query: string) {
  const parts = query.split(/\s+/).filter(Boolean);
  if (!parts.length) return 1;
  const within = (text: string) => parts.every(part => text.toLowerCase().includes(part));
  if (within(item.title)) return 3;
  if (within(`${item.title} ${item.keywords}`)) return 2;
  return within(`${item.card} ${item.title} ${item.subtitle} ${item.keywords}`) ? 1 : 0;
}

function searchQuery() {
  return settingsQuery.trim().toLowerCase();
}

/** Matching settings per page while the sidebar search has text. */
function pageMatchCounts() {
  const counts = new Map<View, number>();
  const query = searchQuery();
  if (!query) return counts;
  for (const item of settingsItems()) {
    if (itemMatches(item, query)) counts.set(item.page, (counts.get(item.page) ?? 0) + 1);
  }
  return counts;
}

function settingRow(item: SettingsItem) {
  const text = `<div><strong>${escape(item.title)}</strong><p>${escape(item.subtitle)}</p></div>`;
  return `<div class="setting-row ${item.wide ? "wide" : ""}">${text}${item.html}</div>${item.after ?? ""}`;
}

/** The page's settings, grouped into cards, filtered by the sidebar search. */
function settingsCards(page: View) {
  const query = searchQuery();
  const items = settingsItems().filter(item => item.page === page && itemMatches(item, query));
  const cards: string[] = [];
  for (const card of [...new Set(items.map(item => item.card))]) {
    const rows = items.filter(item => item.card === card);
    cards.push(`<section class="settings-card" data-settings-group="${escape(card)}"><p class="settings-group">${escape(card)}</p>${rows.map(settingRow).join("")}</section>`);
  }
  return cards.join("");
}

function pageHeader(overline: string, title: string, lede: string, extra = "") {
  return `<header><div><p class="overline">${overline}</p><h1>${title}</h1><p class="lede">${lede}</p></div>${extra}</header>`;
}

function captureHint() {
  return recordingHotkey ? `<p class="recording-hint">Press a key combo, or Escape to cancel.</p>` : "";
}

function shortcutsPage() {
  return `${pageHeader("SHORTCUTS", "Dictate <em>your way.</em>", "Hold a key, tap a shortcut, or use a mouse button. Escape throws a take away.")}
  ${settingsCards("shortcuts")}${captureHint()}`;
}

function audioPage() {
  return `${pageHeader("AUDIO", "Hear you <em>clearly.</em>", "Microphone, when a take ends, and what happens to other sound while you speak.")}
  ${settingsCards("audio")}`;
}

function formattingPage() {
  const tryCard = searchQuery() ? "" : `<section class="settings-card try-card"><p class="settings-group">Try it</p>
    <div class="try-box">
      <textarea id="try-input" class="draft" rows="3" placeholder="Type what a model might hear, e.g. um meet me at seven thirty pm, oh no, eight pm party emoji">${escape(tryText)}</textarea>
      <button type="button" class="quiet-button" id="try-run">Try</button>
      <p class="try-result ${tryResult ? "" : "muted"}">${tryResult ? escape(tryResult) : "The result appears here, formatted with your current settings. Nothing is typed."}</p>
    </div></section>`;
  return `${pageHeader("FORMATTING", "Text that <em>reads right.</em>", "Rules that run on this PC after every take, with every model. No language model is involved.")}
  ${settingsCards("formatting")}${tryCard}`;
}

function dictionaryPage() {
  return `${pageHeader("DICTIONARY", "Words <em>your way.</em>", "Your spellings and fixes apply to every speech model, after transcription, on this PC.")}
  ${settingsCards("dictionary")}`;
}

function snippetsPage() {
  return `${pageHeader("SNIPPETS", "Say less, <em>type more.</em>", "Triggers expand into text you saved. They are matched before any other formatting.")}
  ${settingsCards("snippets")}`;
}

function historyStatus(entry: HistoryEntry) {
  switch (entry.status) {
    case "pending": return "Unfinished";
    case "failed": return "Failed";
    case "cancelled": return "Cancelled, not typed";
    default: return "";
  }
}

function historyEntryMarkup(entry: HistoryEntry) {
  const model = models.find(item => item.id === entry.modelId)?.name ?? entry.modelId;
  const seconds = entry.durationMs ? ` · ${Math.max(1, Math.round(entry.durationMs / 1000))}s` : "";
  const status = historyStatus(entry);
  const body = entry.text
    ? `<p>${escape(entry.text)}</p>`
    : `<p class="history-missing">${escape(entry.error || (entry.status === "pending" ? "This take did not finish." : "No text."))}</p>`;
  const id = String(entry.id);
  const busy = retrying.has(entry.id);
  const actions = [
    entry.text ? `<button type="button" class="text-button" data-copy-history="${id}">Copy</button>` : "",
    entry.audioFile ? `<button type="button" class="text-button" data-play-history="${id}">${playingId === entry.id ? "Stop" : "Play"}</button>` : "",
    entry.audioFile ? `<button type="button" class="text-button" data-retry-history="${id}" ${busy ? "disabled" : ""}>${busy ? "Retrying…" : "Retry"}</button>` : "",
    `<button type="button" class="text-button danger" data-delete-history="${id}">Delete</button>`,
  ].filter(Boolean).join("");
  return `<article class="history-entry ${entry.status && entry.status !== "ok" ? `status-${escape(entry.status)}` : ""}">${body}
    <footer><span>${escape(model)} · ${new Date(entry.createdAtMs).toLocaleString()}${seconds}${status ? ` · <b>${escape(status)}</b>` : ""}</span><span class="history-actions">${actions}</span></footer></article>`;
}

function historyPage() {
  const cards = settingsCards("history");
  if (searchQuery()) {
    return `${pageHeader("LOCAL HISTORY", "Your recent <em>dictation.</em>", "History settings that match your search.")}${cards}`;
  }
  const query = historyQuery.trim().toLowerCase();
  const list = query ? history.filter(entry => entry.text.toLowerCase().includes(query)) : history;
  const entries = list.length
    ? list.map(historyEntryMarkup).join("")
    : `<div class="empty-history">${history.length
      ? `No dictation matches “${escape(historyQuery)}”.`
      : settings.historyEnabled ? "Your local transcription history will appear here." : "Nothing is saved yet. Turn history back on below if you want new takes kept on this PC."}</div>`;
  const lede = settings.historyEnabled
    ? "Stored only on this computer. Search, copy, replay, or retry a take with another model."
    : "New takes are not being saved. Older entries stay on this PC until they expire or you clear them.";
  const clear = history.length ? `<button class="quiet-button" id="clear-history">Clear history</button>` : "";
  return `${pageHeader("LOCAL HISTORY", "Your recent <em>dictation.</em>", lede, clear)}
    ${history.length ? `<div class="history-search"><input id="history-search" class="draft" type="search" placeholder="Search history" value="${escape(historyQuery)}" /></div>` : ""}
    <section class="history-list">${entries}</section>
    ${cards}`;
}

function formatMinutes(minutes: number) {
  if (minutes < 1) return "under a minute";
  if (minutes < 60) return `${Math.round(minutes)} min`;
  const hours = Math.floor(minutes / 60);
  const rest = Math.round(minutes % 60);
  return rest ? `${hours} h ${rest} min` : `${hours} h`;
}

function statTile(label: string, value: string, note = "") {
  return `<div class="stat-tile"><p class="card-label">${escape(label)}</p><strong>${escape(value)}</strong>${note ? `<span>${escape(note)}</span>` : ""}</div>`;
}

function statsPage() {
  const summary = stats;
  const header = pageHeader("STATS", "Your voice, <em>counted.</em>", "Kept only on this PC. Counted when a dictation is typed into another app; tests and retries do not count.",
    summary && summary.dictations ? `<button class="quiet-button" id="reset-stats">Reset stats</button>` : "");
  if (!summary || !summary.dictations) {
    return `${header}<div class="empty-history">Dictate into any app and your totals, pace, and streak show up here.</div>`;
  }
  const max = Math.max(1, ...summary.recent.map(point => point.words));
  const bars = summary.recent.map(point => {
    const date = new Date(`${point.day}T12:00:00`);
    const label = date.toLocaleDateString(undefined, { weekday: "narrow" });
    const height = Math.round((point.words / max) * 100);
    return `<div class="stat-bar" title="${escape(date.toLocaleDateString())}: ${point.words} words"><span style="height:${Math.max(point.words ? 4 : 0, height)}%"></span><small>${escape(label)}</small></div>`;
  }).join("");
  const days = (count: number) => `${count} day${count === 1 ? "" : "s"}`;
  return `${header}
    <section class="stat-grid">
      ${statTile("Words dictated", summary.words.toLocaleString(), `${summary.dictations.toLocaleString()} dictations`)}
      ${statTile("Time saved", formatMinutes(summary.timeSavedMinutes), "versus typing at 40 wpm")}
      ${statTile("Speaking pace", summary.wordsPerMinute ? `${summary.wordsPerMinute} wpm` : "—", `${formatMinutes(summary.audioMinutes)} of speech`)}
      ${statTile("Current streak", days(summary.currentStreak), `Longest ${days(summary.longestStreak)}`)}
      ${statTile("This week", summary.wordsThisWeek.toLocaleString(), `${summary.wordsToday.toLocaleString()} today`)}
      ${statTile("Active days", summary.activeDays.toLocaleString())}
    </section>
    <section class="settings-card stat-chart-card"><p class="settings-group">Last 14 days</p><div class="stat-chart" role="img" aria-label="Words dictated per day over the last 14 days">${bars}</div></section>`;
}

function generalPage() {
  return `${pageHeader("GENERAL", "Make it <em>yours.</em>", "VocaWin only stores these choices locally on this PC. Each change is saved as you make it.")}
  ${settingsCards("settings")}`;
}

function powerPage() {
  return `${pageHeader("POWER", "Stay out of <em>the way.</em>", "Pause for apps that need the microphone, and give memory back when you are not dictating.")}
  ${settingsCards("power")}`;
}

function searchEmptyState() {
  return `${pageHeader("SEARCH", "Nothing <em>found.</em>", `No settings match “${escape(settingsQuery)}”. Try another word, or clear the search.`)}`;
}

function debugFact(label: string, value: string) {
  return `<div><dt>${escape(label)}</dt><dd>${escape(value)}</dd></div>`;
}

function debugPage() {
  const lines = visibleLogs();
  const body = lines.length
    ? lines.map(line => `<div class="log-line level-${escape(line.level)}"><span class="log-level">${escape(line.level)}</span>${escape(line.text)}</div>`).join("")
    : `<div class="empty-history">No warning or error lines yet.${settings.debugLogging ? "" : " Turn on debug logging to see the quieter chatter."}</div>`;
  const report = debugReport;
  const gpuName = report?.gpu.name ?? gpu.name;
  const gpuBackend = report?.gpu.backend ?? gpu.backend;
  const gpuDetail = report?.gpu.detail ?? gpu.detail;
  const gpuDiscrete = report?.gpu.discrete ?? gpu.discrete;
  const gpuVram = report?.gpu.vramMb ?? gpu.vramMb;
  const gpuValue = [
    gpuName,
    gpuDiscrete ? "discrete" : "",
    gpuVram ? `~${gpuVram} MB` : "",
    gpuBackend,
  ].filter(Boolean).join(" · ");
  return `<header><div><p class="overline">DEBUG</p><h1>This PC and <em>logs.</em></h1><p class="lede">This is for testers. Debug logging stays off unless you turn it on. Copy report gathers version, OS, CPU, RAM, GPU, the debug flag, then the in-memory log. It leaves the PC name out. Clear only wipes that buffer, not files on disk.</p></div></header>
    <section class="settings-card"><p class="settings-group">This PC</p>
      <dl class="machine-facts">
        ${debugFact("Version", `VocaWin ${report?.version ?? "0.1.0"} beta`)}
        ${debugFact("OS", report?.os ?? "…")}
        ${debugFact("CPU", report?.cpu ?? "…")}
        ${debugFact("RAM", report?.ram ?? "…")}
        ${debugFact("GPU", gpuValue)}
      </dl>
      <p class="machine-gpu-detail">${escape(gpuDetail || gpuBackend)}</p>
    </section>
    <section class="settings-card"><p class="settings-group">Logs</p>
      <div class="setting-row"><div><strong>Debug logging</strong><p>Off shows warning and error. On also shows debug and info.</p></div>
        <label class="switch"><input id="debug-logging" type="checkbox" ${settings.debugLogging ? "checked" : ""}/><span></span></label>
      </div>
      <div class="log-toolbar">
        <button type="button" class="quiet-button" id="copy-logs">Copy report</button>
        <button type="button" class="quiet-button" id="clear-logs">Clear</button>
      </div>
      <section class="log-panel">${body}</section>
    </section>`;
}

function familyRow(href: string, markClass: string, mark: string, name: string, host: string) {
  return `<li><button type="button" class="about-family-btn" data-open="${href}">
      <span class="about-family-mark ${markClass}">${mark}</span>
      <span class="about-family-copy"><strong>${name}</strong><span>${host}</span></span>
    </button></li>`;
}

function aboutPage() {
  return `<header><div><p class="overline">ABOUT</p><h1>VocaWin <em>beta.</em></h1></div></header>
    <section class="about-hero">
      <img class="about-logo" src="${familyLogo}" width="96" height="96" alt="Voca" />
      <h2>VocaWin</h2>
      <p class="about-tagline">Voice-to-text for Windows, kept on this PC.</p>
      <button type="button" class="text-button" data-open="https://vocawin.com">vocawin.com</button>
    </section>
    <section class="settings-card">
      <p class="settings-group">This app</p>
      <div class="about-copy">
        <p>Voice-to-text for Windows. After a Whisper or ONNX model is on disk, recording and speech-to-text stay on this PC. No Voca account, and no hosted speech API for dictation.</p>
        <p>Hold the hotkey (Right Alt by default) to dictate into any app. Download models on the Models page. Debug has machine details and a copyable support report. This is still an unsigned beta. File bugs from Talk to us below.</p>
      </div>
    </section>
    <section class="settings-card">
      <p class="settings-group">Part of VocaHQ</p>
      <div class="about-copy">
        <p>VocaWin is one of the VocaHQ apps. The same private dictation already runs on Linux as VocaLinux, on macOS as VocaMac, and on phones as VocaPhone. VocaGateway is optional self-hosted compute for other Voca clients.</p>
        <ul class="about-family" role="list">
          ${familyRow("https://vocahq.com", "about-mark-hq", hqMark, "VocaHQ", "vocahq.com")}
          ${familyRow("https://vocalinux.com", "about-mark-platform", linuxMark, "VocaLinux", "vocalinux.com")}
          ${familyRow("https://vocamac.com", "about-mark-platform", appleMark, "VocaMac", "vocamac.com")}
          ${familyRow("https://vocaphone.vocahq.com", "about-mark-phone", `${androidMark}${appleMark}`, "VocaPhone", "vocaphone.vocahq.com")}
          ${familyRow("https://vocagateway.vocahq.com", "about-mark-gateway", gatewayMark, "VocaGateway", "vocagateway.vocahq.com")}
        </ul>
      </div>
    </section>
    <section class="settings-card">
      <p class="settings-group">Talk to us</p>
      <div class="about-copy">
        <p>Bugs, feedback, and feature ideas open a new GitHub issue. You pick the template on the next screen.</p>
        <ul class="about-talk" role="list">
          <li><button type="button" class="primary about-report" data-open="https://github.com/VocaHQ/vocawin/issues/new/choose">${githubMark}<span>Report a bug or idea</span></button></li>
          <li><button type="button" class="about-talk-btn" data-open="https://discord.gg/t6muquAJbm">${discordMark}<span>Discord</span></button></li>
          <li><button type="button" class="about-talk-btn" data-open="https://x.com/vocahq">${xMark}<span>X</span></button></li>
          <li><button type="button" class="about-talk-btn" data-open="mailto:hello@vocahq.com">${mailMark}<span>Email</span></button></li>
        </ul>
      </div>
    </section>`;
}

/** Languages Parakeet TDT v3 transcribes, among the ones VocaWin lists. */
const PARAKEET_LANGUAGES = ["English", "Spanish", "French", "German", "Italian", "Portuguese", "Dutch", "Russian", "Polish", "Ukrainian", "Swedish", "Danish", "Finnish", "Czech", "Greek", "Romanian", "Hungarian"];

/** Whether a catalog model can transcribe `language` ("Auto-detect" means
 *  several languages, so it needs a multilingual model). */
function modelSpeaks(model: Model, language: string) {
  const multilingual = (list: string[]) => language === "Auto-detect" ? list.length > 1 : list.includes(language);
  if (model.id === "voca-hinglish") return multilingual(["Hindi", "English"]);
  if (model.engine === "whisper.cpp") return modelIsEnglishOnly(model) ? language === "English" : true;
  switch (model.id) {
    case "parakeet-tdt-0.6b-v3": return multilingual(PARAKEET_LANGUAGES);
    case "sensevoice-small": return multilingual(["Chinese", "Japanese", "Korean", "English"]);
    case "canary-180m": return multilingual(["English", "Spanish", "German", "French"]);
    case "gigaam-v3": return language === "Russian";
    default: return language === "English" && modelIsEnglishOnly(model);
  }
}

/** Up to three models for the setup guide: this PC's suggested size first
 *  when it fits, then specialists for the language, then the rest. */
function suggestedModels(language: string) {
  const fits = models.filter(model => modelSpeaks(model, language));
  const preferred: string[] = [];
  if (recommendation && fits.some(model => model.id === recommendation!.modelId)) preferred.push(recommendation.modelId);
  if (language === "English") preferred.push("parakeet-tdt-0.6b-v3", "moonshine-base");
  else if (language === "Russian") preferred.push("gigaam-v3", "parakeet-tdt-0.6b-v3");
  else if (language === "Hindi") preferred.push("voca-hinglish");
  else if (["Chinese", "Japanese", "Korean"].includes(language)) preferred.push("sensevoice-small");
  else if (PARAKEET_LANGUAGES.includes(language)) preferred.push("parakeet-tdt-0.6b-v3");
  preferred.push("whisper-small", "whisper-base");
  const ordered = [...preferred.map(id => fits.find(model => model.id === id)), ...fits]
    .filter((model): model is Model => !!model);
  return [...new Map(ordered.map(model => [model.id, model])).values()].slice(0, 3);
}

const ONBOARDING_STEPS = ["Welcome", "Language", "Model", "Try it", "Done"];

function onboardingOverlay() {
  if (settings.welcomeDismissed) return "";
  const dots = ONBOARDING_STEPS.map((label, index) =>
    `<li class="${index === onboardingStep ? "current" : index < onboardingStep ? "done" : ""}"><span>${escape(label)}</span></li>`).join("");
  const key = `<kbd>${escape(hotkeyLabel())}</kbd>`;
  const hold = settings.activationMode === "toggle" ? `Tap ${key}, speak, then tap it again.` : `Hold ${key}, speak, then let go.`;
  let body = "";
  let actions = "";
  switch (onboardingStep) {
    case 0:
      body = `<h2 id="welcome-title">Private voice typing for Windows</h2>
        <ul class="onboarding-points">
          <li><strong>Talk anywhere.</strong> ${hold} Your words appear at the caret in the app you are using.</li>
          <li><strong>Stays on this PC.</strong> After a model downloads, audio and text never leave this computer.</li>
          <li><strong>Lives in the tray.</strong> Closing this window keeps VocaWin running. A small pill shows when it is listening.</li>
        </ul>`;
      actions = `<button type="button" class="text-button" id="onboarding-skip">Skip setup</button><button type="button" class="primary" data-onboarding-next>Get started</button>`;
      break;
    case 1:
      body = `<h2 id="welcome-title">What do you speak?</h2>
        <p>VocaWin suggests models that know your language. Pick Auto-detect if you switch between languages.</p>
        <select id="onboarding-language" class="themed-select">${LANGUAGE_CHOICES.map(language => `<option value="${escape(language)}" ${language === (settings.language || "Auto-detect") ? "selected" : ""}>${escape(language === "Auto-detect" ? "Auto-detect (several languages)" : language)}</option>`).join("")}</select>`;
      actions = `<button type="button" class="text-button" data-onboarding-back>Back</button><button type="button" class="primary" data-onboarding-next>Next</button>`;
      break;
    case 2: {
      const suggestions = suggestedModels(settings.language || "Auto-detect");
      if (!onboardingModel || !suggestions.some(model => model.id === onboardingModel)) {
        onboardingModel = suggestions.find(model => statuses[model.id]?.installed)?.id ?? suggestions[0]?.id ?? settings.selectedModel;
      }
      const chosen = statuses[onboardingModel];
      const rows = suggestions.map(model => {
        const status = statuses[model.id];
        const state = status?.downloading ? `Downloading ${status.progress}%` : status?.installed ? "On this PC" : model.size;
        const note = recommendation?.modelId === model.id ? " · Suggested for this PC" : "";
        return `<label class="onboarding-model ${model.id === onboardingModel ? "selected" : ""}">
          <input type="radio" name="onboarding-model" value="${escape(model.id)}" ${model.id === onboardingModel ? "checked" : ""} />
          <span><strong>${escape(model.name)}</strong><small>${escape(model.description)}${escape(note)}</small></span>
          <em>${escape(state)}</em></label>`;
      }).join("");
      body = `<h2 id="welcome-title">Pick a speech model</h2>
        <p>Models run on this PC. The download happens once; after that VocaWin works offline.</p>
        <div class="onboarding-models">${rows || `<p>No model in the catalog covers that language yet. Go back and pick Auto-detect.</p>`}</div>`;
      const label = chosen?.downloading ? `Downloading ${chosen.progress}%` : chosen?.installed ? "Use this model" : "Download and use";
      actions = `<button type="button" class="text-button" data-onboarding-back>Back</button><button type="button" class="primary" id="onboarding-model-go" ${chosen?.downloading || !rows ? "disabled" : ""}>${label}</button>`;
      break;
    }
    case 3:
      body = `<h2 id="welcome-title">Try your first dictation</h2>
        <p>Click in the box, then ${hold.charAt(0).toLowerCase()}${hold.slice(1)} Press Escape while speaking to throw a take away.</p>
        <textarea id="onboarding-try" class="draft" rows="4" placeholder="Your words appear here.">${escape(onboardingTryText)}</textarea>
        <p class="onboarding-note">${recording ? "Listening…" : "It works the same in any app: the text goes where the caret is."}</p>`;
      actions = `<button type="button" class="text-button" data-onboarding-back>Back</button><button type="button" class="primary" data-onboarding-next>${onboardingTryText.trim() ? "Next" : "Skip for now"}</button>`;
      break;
    default:
      body = `<h2 id="welcome-title">You are set</h2>
        <ul class="onboarding-points">
          <li><strong>${escape(hotkeyLabel())}</strong> dictates in any app. Change it, or add a hands-free shortcut or mouse button, on the Shortcuts page.</li>
          <li><strong>Formatting</strong> has spoken numbers, emoji, and cleanup. <strong>Dictionary</strong> fixes names a model gets wrong.</li>
          <li>VocaWin lives in the tray. Closing this window keeps it running.</li>
        </ul>
        <label class="onboarding-login">${switchControl("onboarding-login", settings.launchAtLogin)}<span>Start VocaWin when I sign in to Windows</span></label>`;
      actions = `<button type="button" class="text-button" data-onboarding-back>Back</button><button type="button" class="primary" id="welcome-dismiss">Finish</button>`;
  }
  return `<div class="welcome-overlay" role="dialog" aria-modal="true" aria-labelledby="welcome-title">
    <div class="welcome-card onboarding">
      <ol class="onboarding-steps" aria-label="Setup steps">${dots}</ol>
      ${body}
      <div class="onboarding-actions">${actions}</div>
    </div>
  </div>`;
}

function onboardingTrying() {
  return !settings.welcomeDismissed && onboardingStep === 3;
}

function sidebarFooter() {
  const parked = !!runtime.parkKind && !recording;
  const statusLabel = sidebarStatusLabel();
  const result = testResult
    ? `<p class="sidebar-result">${escape(testResult)}</p>`
    : `<p class="sidebar-result muted">Practice here. This take does not type into other apps.</p>`;
  return `<div class="sidebar-footer">
    <div class="sidebar-status ${parked ? "parked" : recording ? "live" : ""}"><i></i><div><strong>${statusLabel}</strong>${runtime.parkDetail && !recording ? `<small>${escape(runtime.parkDetail)}</small>` : `<small>${escape(runtime.inputDevice)}</small>`}</div></div>
    ${toastMarkup()}
    <button type="button" class="quiet-button sidebar-test" id="test-dictation" ${runtime.paused || micTesting ? "disabled" : ""} aria-label="${testListening ? "Stop test dictation" : "Test dictation inside VocaWin"}">${testListening ? "Stop test" : testingDictation ? "Testing…" : "Test dictation"}</button>
    ${result}
  </div>`;
}

function toastMarkup() {
  return `<p class="toast" id="sidebar-toast" role="status"${toastText ? "" : " hidden"}>${escape(toastText)}</p>`;
}

function captureChrome() {
  const main = document.querySelector("main");
  if (main && !resetPaneScroll) paneScroll = main.scrollTop;
  const active = document.activeElement;
  if (active instanceof HTMLElement && active.id && !focusRestore) {
    if (active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement) {
      focusRestore = { id: active.id, start: active.selectionStart ?? 0, end: active.selectionEnd ?? 0 };
    } else {
      focusRestore = { id: active.id, start: 0, end: 0 };
    }
  }
}

function restorePaneScroll() {
  const main = document.querySelector("main");
  if (!main) return;
  main.scrollTop = resetPaneScroll ? 0 : paneScroll;
  resetPaneScroll = false;
}

function sidebarNav() {
  const counts = pageMatchCounts();
  const searching = !!searchQuery();
  return NAV_SECTIONS.map(section => {
    const items = section.items
      .filter(([id]) => !searching || counts.has(id))
      .map(([id, label, icon]) => nav(id, label, icon, searching ? counts.get(id) ?? 0 : 0))
      .join("");
    return items ? `<p class="nav-section">${escape(section.label)}</p>${items}` : "";
  }).join("") || `<p class="nav-empty">No settings match.</p>`;
}

/** VocaMac's search contract: filter pages, badge counts, jump to the first
 *  page with matches, and restore the previous page when the search clears. */
function applySettingsSearch(value: string) {
  const was = searchQuery();
  settingsQuery = value;
  const query = searchQuery();
  if (query && !was) pageBeforeSearch = view;
  if (!query) {
    if (pageBeforeSearch) view = pageBeforeSearch;
    pageBeforeSearch = null;
    return;
  }
  // Land on the page whose setting best matches, not merely the first
  // page that mentions the word somewhere.
  const best = new Map<View, number>();
  for (const item of settingsItems()) {
    const score = matchScore(item, query);
    if (score > (best.get(item.page) ?? 0)) best.set(item.page, score);
  }
  const top = Math.max(0, ...best.values());
  if ((best.get(view) ?? 0) < top) {
    const first = ALL_VIEWS.find(id => best.get(id) === top);
    if (first) view = first;
  }
}

function render() {
  captureChrome();
  const pages: Record<View, () => string> = {
    dictation: dictationPage,
    shortcuts: shortcutsPage,
    models: modelsPage,
    audio: audioPage,
    formatting: formattingPage,
    dictionary: dictionaryPage,
    snippets: snippetsPage,
    history: historyPage,
    stats: statsPage,
    settings: generalPage,
    power: powerPage,
    debug: debugPage,
    about: aboutPage,
  };
  const noMatches = !!searchQuery() && pageMatchCounts().size === 0;
  app.innerHTML = `<aside>
    <div class="brand"><span class="mark">${sidebarMark}</span><span>VocaWin</span><span class="brand-tag" title="Unsigned tester build">Beta</span></div>
    <div class="sidebar-search"><input id="settings-search" type="search" placeholder="Search settings" aria-label="Search settings" value="${escape(settingsQuery)}" /></div>
    <nav>${sidebarNav()}</nav>
    ${sidebarFooter()}
  </aside>
  <main>
    ${noMatches ? searchEmptyState() : pages[view]()}
    ${onboardingOverlay()}
  </main>`;
  bindChrome();
  restoreFocusedField();
  restorePaneScroll();
}

function openView(next: View) {
  view = next;
  resetPaneScroll = true;
  if (searchQuery() && !pageMatchCounts().has(next)) {
    settingsQuery = "";
    pageBeforeSearch = null;
  }
  if (next === "power" || next === "formatting") {
    void Promise.all([refreshRunningApps(), refreshRuntime()]).then(render);
    return;
  }
  if (next === "debug") {
    void Promise.all([refreshLogs(), refreshDebugReport()]).then(render);
    return;
  }
  if (next === "stats") {
    void refreshStats().then(render);
    return;
  }
  if (next === "history") {
    void refreshHistory().then(render).catch(() => render());
    return;
  }
  render();
}

function bindChrome() {
  document.querySelectorAll<HTMLButtonElement>("[data-view]").forEach(button => button.addEventListener("click", () => {
    openView(button.dataset.view as View);
  }));
  document.querySelectorAll<HTMLButtonElement>("[data-go]").forEach(button => button.addEventListener("click", () => {
    openView(button.dataset.go as View);
  }));
  document.querySelectorAll<HTMLButtonElement>("[data-open]").forEach(button => button.addEventListener("click", () => {
    void openExternal(button.dataset.open!);
  }));
  document.querySelector("#record")?.addEventListener("click", toggleRecording);
  document.querySelector("#clear-history")?.addEventListener("click", clearHistory);
  document.querySelector("#test-dictation")?.addEventListener("click", testDictation);
  document.querySelector("#mic-test")?.addEventListener("click", toggleMicTest);
  document.querySelector("#welcome-dismiss")?.addEventListener("click", dismissWelcome);
  document.querySelector("#copy-logs")?.addEventListener("click", copyLogs);
  document.querySelector("#clear-logs")?.addEventListener("click", clearLogs);
  document.querySelector("#auto-pause-app")?.addEventListener("change", () => {
    void addWatchedApp();
  });
  bindWatchedAppChips();
  bindFilterCombo("#engine-filter", ENGINE_FILTERS, value => { engineFilter = value as EngineFilter; });
  bindFilterCombo("#language-filter", LANGUAGE_FILTERS, value => { languageFilter = value as LanguageFilter; });
  bindLiveSearch("#settings-search", applySettingsSearch);
  bindLiveSearch("#model-search", value => { modelQuery = value; });
  bindLiveSearch("#history-search", value => { historyQuery = value; });
  document.querySelectorAll<HTMLButtonElement>("[data-capture]").forEach(button => button.addEventListener("click", () => {
    void toggleHotkeyRecording(button.dataset.capture as CaptureTarget);
  }));
  bindPageActions();
  bindOnboarding();
  document.querySelector<HTMLSelectElement>("#paste-app")?.addEventListener("change", event => {
    const name = (event.target as HTMLSelectElement).value.trim();
    if (!name || pasteApps().some(app => app.toLowerCase() === name.toLowerCase())) return;
    settings.pasteApps = [...pasteApps(), name].join("\n");
    void persistSettings(true, true).then(render);
  });
  document.querySelectorAll<HTMLButtonElement>("[data-unpaste]").forEach(button => button.addEventListener("click", () => {
    settings.pasteApps = pasteApps().filter(app => app !== button.dataset.unpaste).join("\n");
    void persistSettings(true, true).then(render);
  }));
  const soundTheme = document.querySelector<HTMLSelectElement>("#sound-theme");
  const previewSound = document.querySelector<HTMLButtonElement>("#preview-sound");
  previewSound?.addEventListener("click", async () => {
    const theme = soundTheme?.value ?? settings.soundTheme;
    if (theme === "off") return;
    try {
      await invoke("preview_sound", { theme, start: previewStartNext });
      previewStartNext = !previewStartNext;
      if (previewSound) {
        const label = previewStartNext ? "Preview start" : "Preview end";
        previewSound.innerHTML = previewStartNext ? ICON_PLAY : ICON_STOP;
        previewSound.setAttribute("aria-label", label);
        previewSound.title = label;
      }
    } catch (error) {
      showToast(String(error));
      render();
    }
  });
  const language = document.querySelector<HTMLInputElement>("#language");
  if (language) language.value = settings.language;
  const activation = document.querySelector<HTMLSelectElement>("#activation");
  if (activation) activation.value = settings.activationMode;
  document.querySelectorAll<HTMLInputElement>("[data-activate]").forEach(box => {
    box.addEventListener("click", event => event.stopPropagation());
    box.addEventListener("change", () => {
      if (box.checked) void selectModel(box.dataset.activate!);
      else {
        box.checked = true;
      }
    });
  });
  document.querySelectorAll<HTMLButtonElement>("[data-download]").forEach(button => button.addEventListener("click", event => {
    event.stopPropagation();
    downloadModel(button.dataset.download!);
  }));
  document.querySelectorAll<HTMLButtonElement>("[data-delete]").forEach(button => button.addEventListener("click", event => {
    event.stopPropagation();
    deleteModel(button.dataset.delete!);
  }));
  bindAutosave();
}

function bindLiveSearch(selector: string, assign: (value: string) => void) {
  document.querySelector(selector)?.addEventListener("input", event => {
    const input = event.target as HTMLInputElement;
    assign(input.value);
    focusRestore = { id: input.id, start: input.selectionStart ?? input.value.length, end: input.selectionEnd ?? input.value.length };
    render();
  });
}

function restoreFocusedField() {
  if (!focusRestore) return;
  const field = document.querySelector<HTMLElement>(`#${focusRestore.id}`);
  if (field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement) {
    field.focus();
    try { field.setSelectionRange(focusRestore.start, focusRestore.end); } catch { /* ignore */ }
  } else if (field instanceof HTMLSelectElement) {
    field.focus();
  }
  focusRestore = null;
}

function bindFilterCombo(selector: string, options: Array<[string, string]>, assign: (value: string) => void) {
  const input = document.querySelector<HTMLInputElement>(selector);
  if (!input) return;
  const apply = () => {
    const match = options.find(([, label]) => label.toLowerCase() === input.value.trim().toLowerCase());
    if (!match) return;
    assign(match[0]);
    focusRestore = { id: input.id, start: input.value.length, end: input.value.length };
    render();
  };
  input.addEventListener("change", apply);
  input.addEventListener("input", apply);
}

function bindAutosave() {
  const persistFromEvent = (event: Event) => {
    const target = event.target as HTMLElement;
    if (target.id === "settings-search" || target.id === "model-search" || target.id === "engine-filter" || target.id === "language-filter" || target.id === "auto-pause-app" || target.id === "paste-app") return;
    if (target.classList.contains("draft")) return;
    void persistSettings();
  };
  document.querySelectorAll<HTMLElement>(".setting-row input, .setting-row select, .setting-row textarea, #debug-logging, #idle-unload").forEach(node => {
    if (node.id === "custom-vocabulary" || node.id === "language") {
      node.addEventListener("change", persistFromEvent);
      node.addEventListener("blur", persistFromEvent);
      return;
    }
    node.addEventListener("change", persistFromEvent);
    if (node instanceof HTMLInputElement && (node.type === "number" || node.type === "search")) {
      node.addEventListener("blur", persistFromEvent);
    }
  });
}

function collectSettingsFromDom() {
  const preset = document.querySelector<HTMLSelectElement>("#hotkey-preset");
  if (preset) settings.hotkey = preset.value;
  const language = document.querySelector<HTMLInputElement>("#language");
  if (language) {
    const typed = language.value.trim();
    const match = LANGUAGE_CHOICES.find(item => item.toLowerCase() === typed.toLowerCase())
      ?? (typed && !LANGUAGE_CHOICES.includes(typed) ? typed : "");
    if (match) settings.language = match;
  }
  const activation = document.querySelector<HTMLSelectElement>("#activation");
  if (activation) settings.activationMode = activation.value;
  const silence = document.querySelector<HTMLInputElement>("#silence");
  if (silence) settings.silenceSeconds = Number(silence.value) || 1.5;
  const maxRecording = document.querySelector<HTMLInputElement>("#max-recording");
  if (maxRecording) settings.maxRecordingSeconds = Number(maxRecording.value) || 60;
  const soundTheme = document.querySelector<HTMLSelectElement>("#sound-theme");
  if (soundTheme) {
    settings.soundTheme = soundTheme.value;
    settings.soundEffects = soundTheme.value !== "off";
    previewStartNext = true;
  }
  const autoCap = document.querySelector<HTMLInputElement>("#auto-cap");
  if (autoCap) settings.autoCapitalize = autoCap.checked;
  const trailing = document.querySelector<HTMLInputElement>("#trailing-space");
  if (trailing) settings.appendTrailingSpace = trailing.checked;
  const copyToClipboard = document.querySelector<HTMLInputElement>("#copy-to-clipboard");
  if (copyToClipboard) settings.copyToClipboard = copyToClipboard.checked;
  const launch = document.querySelector<HTMLInputElement>("#launch-login");
  if (launch) settings.launchAtLogin = launch.checked;
  const inputDevice = document.querySelector<HTMLSelectElement>("#input-device");
  if (inputDevice) settings.inputDevice = inputDevice.value;
  const idleUnload = document.querySelector<HTMLSelectElement>("#idle-unload");
  if (idleUnload) {
    const seconds = Number(idleUnload.value);
    settings.idleUnloadEnabled = seconds > 0;
    if (seconds > 0) settings.idleUnloadSeconds = seconds;
  }
  const historyEnabled = document.querySelector<HTMLInputElement>("#history-enabled");
  if (historyEnabled) settings.historyEnabled = historyEnabled.checked;
  const debugLogging = document.querySelector<HTMLInputElement>("#debug-logging");
  if (debugLogging) settings.debugLogging = debugLogging.checked;
  const customVocabulary = document.querySelector<HTMLTextAreaElement>("#custom-vocabulary");
  if (customVocabulary) settings.customVocabulary = customVocabulary.value;
  const pick = (id: string) => document.querySelector<HTMLSelectElement>(`#${id}`)?.value;
  settings.insertionMode = pick("insertion-mode") ?? settings.insertionMode;
  const checked = (id: string) => document.querySelector<HTMLInputElement>(`#${id}`)?.checked;
  settings.handsFreeHotkey = pick("hands-free-hotkey") ?? settings.handsFreeHotkey;
  settings.pasteLastHotkey = pick("paste-last-hotkey") ?? settings.pasteLastHotkey;
  settings.mouseButton = pick("mouse-button") ?? settings.mouseButton;
  settings.cleanupLevel = pick("cleanup-level") ?? settings.cleanupLevel;
  settings.overlayStyle = pick("overlay-style") ?? settings.overlayStyle;
  settings.overlayPosition = pick("overlay-position") ?? settings.overlayPosition;
  const retention = pick("history-retention");
  if (retention !== undefined) settings.historyRetentionDays = Number(retention);
  settings.escapeCancels = checked("escape-cancels") ?? settings.escapeCancels;
  settings.skipSilence = checked("skip-silence") ?? settings.skipSilence;
  settings.muteOtherAudio = checked("mute-other-audio") ?? settings.muteOtherAudio;
  settings.numbersAsDigits = checked("numbers-as-digits") ?? settings.numbersAsDigits;
  settings.numberSymbols = checked("number-symbols") ?? settings.numberSymbols;
  settings.spokenEmoji = checked("spoken-emoji") ?? settings.spokenEmoji;
  settings.historyKeepAudio = checked("history-keep-audio") ?? settings.historyKeepAudio;
  settings.readyPill = checked("ready-pill") ?? settings.readyPill;
}

async function persistSettings(silent = false, skipCollect = false) {
  if (!skipCollect) collectSettingsFromDom();
  try {
    await invoke("save_settings", { settings });
    settings = await invoke<Settings>("get_settings");
    await refreshRuntime();
    if (view === "debug") {
      await Promise.all([refreshLogs(), refreshDebugReport()]);
      render();
      if (!silent) showToast("Settings saved");
      return;
    }
    syncSettingsControls();
    // Some rows enable others (symbols need digits); redraw those pages.
    if (view === "formatting" || view === "shortcuts") render();
    if (!silent) showToast("Settings saved");
  } catch (error) {
    // Keep the window honest: show what was actually saved.
    try { settings = await invoke<Settings>("get_settings"); } catch { /* keep local copy */ }
    render();
    showToast(String(error));
  }
}

function syncSettingsControls() {
  const preset = document.querySelector<HTMLSelectElement>("#hotkey-preset");
  if (preset && document.activeElement !== preset) preset.value = settings.hotkey;
  const language = document.querySelector<HTMLInputElement>("#language");
  if (language && document.activeElement !== language) language.value = settings.language;
  const activation = document.querySelector<HTMLSelectElement>("#activation");
  if (activation && document.activeElement !== activation) activation.value = settings.activationMode;
  const soundTheme = document.querySelector<HTMLSelectElement>("#sound-theme");
  if (soundTheme && document.activeElement !== soundTheme) soundTheme.value = settings.soundTheme;
  const inputDevice = document.querySelector<HTMLSelectElement>("#input-device");
  if (inputDevice && document.activeElement !== inputDevice) inputDevice.value = settings.inputDevice;
  const idleUnload = document.querySelector<HTMLSelectElement>("#idle-unload");
  if (idleUnload && document.activeElement !== idleUnload) idleUnload.value = String(idleUnloadValue());
}

function bindWatchedAppChips() {
  document.querySelectorAll<HTMLButtonElement>("[data-unwatch]").forEach(button => {
    button.addEventListener("click", () => void removeWatchedApp(button.dataset.unwatch!));
  });
}

function paintPowerApps() {
  const select = document.querySelector<HTMLSelectElement>("#auto-pause-app");
  if (select) {
    select.innerHTML = runningAppOptions();
    select.value = "";
  }
  const chips = document.querySelector("#watched-app-chips");
  if (chips) {
    chips.innerHTML = watchedAppChips();
    bindWatchedAppChips();
  }
}

async function addWatchedApp() {
  const select = document.querySelector<HTMLSelectElement>("#auto-pause-app");
  const name = (select?.value || "").trim();
  if (!name) return;
  const current = watchedApps();
  if (current.some(entry => entry.toLowerCase() === name.toLowerCase())) {
    if (select) select.value = "";
    return;
  }
  current.push(name);
  settings.autoPauseApps = current.join("\n");
  settings.autoPauseEnabled = true;
  await persistSettings();
  paintPowerApps();
}

async function removeWatchedApp(name: string) {
  const remaining = watchedApps().filter(entry => entry !== name);
  settings.autoPauseApps = remaining.join("\n");
  settings.autoPauseEnabled = remaining.length > 0;
  await persistSettings();
  paintPowerApps();
}

async function openExternal(url: string) {
  try {
    await invoke("open_external", { url });
  } catch (error) {
    showToast(String(error));
    render();
  }
}

function codeToHotkeyPart(code: string, key: string): string | null {
  const map: Record<string, string> = {
    Space: "Space",
    F7: "F7",
    F8: "F8",
    F9: "F9",
    F10: "F10",
    ControlRight: "ControlRight",
    ControlLeft: "ControlLeft",
    AltRight: "AltRight",
    AltLeft: "AltLeft",
    ShiftRight: "ShiftRight",
    ShiftLeft: "ShiftLeft",
  };
  if (map[code]) return map[code];
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (key.length === 1) return key.toUpperCase();
  return null;
}

async function toggleHotkeyRecording(target: CaptureTarget) {
  if (recordingHotkey) {
    const same = recordingHotkey === target;
    recordingHotkey = null;
    try { await invoke("resume_hotkey_listener"); } catch { /* ignore */ }
    if (same) {
      showToast("Shortcut recording cancelled.");
      render();
      return;
    }
  }
  try { await invoke("pause_hotkey_listener"); } catch { /* ignore */ }
  recordingHotkey = target;
  render();
}

function finishHotkeyCapture(spec: string, label: string) {
  const target = recordingHotkey ?? "hotkey";
  settings[target] = spec;
  recordingHotkey = null;
  void invoke("resume_hotkey_listener").catch(() => undefined);
  void persistSettings(true, true).then(() => {
    syncSettingsControls();
    render();
    if (settings[target] === spec) showToast(`Shortcut set to ${label}.`);
  });
}

function onGlobalKeyDown(event: KeyboardEvent) {
  if (!recordingHotkey) return;
  event.preventDefault();
  event.stopPropagation();
  if (event.key === "Escape") {
    recordingHotkey = null;
    showToast("Shortcut recording cancelled.");
    void invoke("resume_hotkey_listener").catch(() => undefined);
    render();
    return;
  }
  if (event.key === "Meta" || event.code.startsWith("Meta") || event.code === "OSLeft" || event.code === "OSRight") {
    showToast("Win/Super is reserved on Windows. Pick another key.");
    render();
    return;
  }
  const isModifier = ["Control", "Alt", "Shift", "Meta"].includes(event.key);
  if (isModifier) return;
  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  const keyPart = codeToHotkeyPart(event.code, event.key);
  if (!keyPart) return;
  if (keyPart === "ControlRight" || keyPart === "AltRight" || keyPart === "ShiftRight"
    || keyPart === "ControlLeft" || keyPart === "AltLeft" || keyPart === "ShiftLeft") {
    finishHotkeyCapture(keyPart, keyPart);
    return;
  }
  parts.push(keyPart);
  finishHotkeyCapture(parts.join("+"), parts.join("+"));
}

function onGlobalKeyUp(event: KeyboardEvent) {
  if (!recordingHotkey) return;
  if (event.code === "ControlRight" || event.code === "AltRight" || event.code === "ShiftRight") {
    event.preventDefault();
    event.stopPropagation();
    if (event.code === "ControlRight") {
      finishHotkeyCapture("ControlRight", "Right Ctrl");
    } else if (event.code === "AltRight") {
      finishHotkeyCapture("AltRight", "Right Alt");
    } else {
      finishHotkeyCapture("ShiftRight", "Right Shift");
    }
  }
}

async function refreshStatuses() { statuses = await invoke<Record<string, ModelStatus>>("get_model_statuses"); }
async function refreshHistory() { history = await invoke<HistoryEntry[]>("get_history"); }
async function refreshRuntime() {
  try {
    runtime = await invoke<RuntimeStatus>("get_runtime_status");
    if (typeof runtime.recording === "boolean") {
      recording = runtime.recording;
      if (!runtime.recording) testListening = false;
    }
  } catch { /* ignore */ }
}
async function refreshLogs() {
  try { logLines = await invoke<LogLine[]>("get_log_lines"); } catch { logLines = []; }
}
async function refreshDebugReport() {
  try { debugReport = await invoke<DebugReport>("get_debug_report"); } catch { debugReport = null; }
}
async function refreshRunningApps() {
  try { runningApps = await invoke<RunningApp[]>("list_running_apps"); } catch { runningApps = []; }
}
async function clearHistory() {
  try { await invoke("clear_history"); history = []; showToast("History cleared."); } catch (error) { showToast(String(error)); }
  render();
}
async function copyLogs() {
  let text = "";
  try {
    const report = await invoke<DebugReport>("get_debug_report");
    debugReport = report;
    text = report.text;
  } catch {
    showToast("Could not build the debug report.");
    render();
    return;
  }
  if (!text.trim()) {
    showToast("Nothing to copy.");
    render();
    return;
  }
  try {
    await invoke("copy_text", { text });
    showToast("Debug report copied.");
  } catch {
    try {
      await navigator.clipboard.writeText(text);
      showToast("Debug report copied.");
    } catch {
      showToast("Could not copy the debug report.");
    }
  }
}
async function clearLogs() {
  if (!window.confirm("Clear the log buffer for this session? This does not delete files on disk.")) {
    return;
  }
  try {
    await invoke("clear_log_lines");
    logLines = [];
    showToast("Logs cleared.");
  } catch (error) {
    showToast(String(error));
  }
  render();
}
async function dismissWelcome() {
  try {
    const login = document.querySelector<HTMLInputElement>("#onboarding-login");
    if (login && login.checked !== settings.launchAtLogin) {
      settings.launchAtLogin = login.checked;
      await persistSettings(true, true);
    }
    await invoke("dismiss_welcome");
    settings.welcomeDismissed = true;
    onboardingStep = 0;
  } catch (error) {
    showToast(String(error));
  }
  render();
}

async function refreshStats() {
  try { stats = await invoke<StatsSummary>("get_stats"); } catch { stats = null; }
}

function historyId(button: HTMLElement, key: string) {
  return Number(button.dataset[key]);
}

function bindPageActions() {
  document.querySelectorAll<HTMLButtonElement>("[data-copy-history]").forEach(button => button.addEventListener("click", async () => {
    const entry = history.find(item => item.id === historyId(button, "copyHistory"));
    if (!entry?.text) return;
    try { await invoke("copy_text", { text: entry.text }); showToast("Copied."); } catch (error) { showToast(String(error)); }
  }));
  document.querySelectorAll<HTMLButtonElement>("[data-play-history]").forEach(button => button.addEventListener("click", async () => {
    const id = historyId(button, "playHistory");
    try {
      if (playingId === id) {
        await invoke("stop_history_audio");
        playingId = null;
      } else {
        await invoke("play_history_audio", { id });
        playingId = id;
        const entry = history.find(item => item.id === id);
        const length = Math.max(1000, entry?.durationMs ?? 5000);
        window.setTimeout(() => { if (playingId === id) { playingId = null; render(); } }, length + 300);
      }
    } catch (error) { playingId = null; showToast(String(error)); }
    render();
  }));
  document.querySelectorAll<HTMLButtonElement>("[data-retry-history]").forEach(button => button.addEventListener("click", async () => {
    const id = historyId(button, "retryHistory");
    retrying.add(id);
    render();
    try {
      const text = await invoke<string>("retry_history_entry", { id });
      showToast(text ? "Retried with the current model." : "No speech was recognized.");
    } catch (error) { showToast(String(error)); }
    retrying.delete(id);
    await refreshHistory().catch(() => undefined);
    render();
  }));
  document.querySelectorAll<HTMLButtonElement>("[data-delete-history]").forEach(button => button.addEventListener("click", async () => {
    const id = historyId(button, "deleteHistory");
    try {
      await invoke("delete_history_entry", { id });
      if (playingId === id) playingId = null;
      history = history.filter(item => item.id !== id);
    } catch (error) { showToast(String(error)); }
    render();
  }));
  document.querySelector("#reset-stats")?.addEventListener("click", async () => {
    if (!window.confirm("Reset all stats on this PC? History is not affected.")) return;
    try { await invoke("reset_stats"); await refreshStats(); showToast("Stats reset."); } catch (error) { showToast(String(error)); }
    render();
  });
  document.querySelector("#add-replacement")?.addEventListener("click", () => {
    const heard = document.querySelector<HTMLInputElement>("#replacement-heard")?.value.trim() ?? "";
    const replacement = document.querySelector<HTMLInputElement>("#replacement-to")?.value.trim() ?? "";
    if (!heard || !replacement) { showToast("Fill in what is heard and what to type."); return; }
    settings.replacements = [...settings.replacements, { heard, replacement }];
    void persistSettings(true, true).then(render);
  });
  document.querySelectorAll<HTMLButtonElement>("[data-remove-replacement]").forEach(button => button.addEventListener("click", () => {
    const index = Number(button.dataset.removeReplacement);
    settings.replacements = settings.replacements.filter((_, position) => position !== index);
    void persistSettings(true, true).then(render);
  }));
  document.querySelector("#add-snippet")?.addEventListener("click", () => {
    const trigger = document.querySelector<HTMLInputElement>("#snippet-trigger")?.value.trim() ?? "";
    const expansion = document.querySelector<HTMLTextAreaElement>("#snippet-expansion")?.value ?? "";
    if (!trigger || !expansion.trim()) { showToast("Fill in the trigger and the text to type."); return; }
    settings.snippets = [...settings.snippets, { trigger, expansion }];
    void persistSettings(true, true).then(render);
  });
  document.querySelectorAll<HTMLButtonElement>("[data-remove-snippet]").forEach(button => button.addEventListener("click", () => {
    const index = Number(button.dataset.removeSnippet);
    settings.snippets = settings.snippets.filter((_, position) => position !== index);
    void persistSettings(true, true).then(render);
  }));
  const tryInput = document.querySelector<HTMLTextAreaElement>("#try-input");
  tryInput?.addEventListener("input", () => { tryText = tryInput.value; });
  document.querySelector("#try-run")?.addEventListener("click", async () => {
    try { tryResult = (await invoke<string>("preview_text", { text: tryText })) || "(nothing would be typed)"; } catch (error) { tryResult = String(error); }
    render();
  });
  document.querySelector("#export-settings")?.addEventListener("click", async () => {
    try { const path = await invoke<string>("export_settings"); showToast(`Saved to ${path}`); } catch (error) { showToast(String(error)); }
  });
  const importFile = document.querySelector<HTMLInputElement>("#import-file");
  document.querySelector("#import-settings")?.addEventListener("click", () => importFile?.click());
  importFile?.addEventListener("change", async () => {
    const file = importFile.files?.[0];
    if (!file) return;
    try {
      const contents = await file.text();
      settings = await invoke<Settings>("parse_settings_backup", { contents });
      await persistSettings(true, true);
      showToast("Settings imported.");
    } catch (error) { showToast(String(error)); }
    render();
  });
  document.querySelector("#run-setup")?.addEventListener("click", async () => {
    settings.welcomeDismissed = false;
    onboardingStep = 0;
    onboardingTryText = "";
    await persistSettings(true, true);
    render();
  });
}

function bindOnboarding() {
  document.querySelector("#onboarding-skip")?.addEventListener("click", dismissWelcome);
  document.querySelectorAll<HTMLButtonElement>("[data-onboarding-next]").forEach(button => button.addEventListener("click", async () => {
    if (onboardingStep === 1) {
      const language = document.querySelector<HTMLSelectElement>("#onboarding-language")?.value;
      if (language && language !== settings.language) {
        settings.language = language;
        await persistSettings(true, true);
      }
    }
    onboardingStep = Math.min(ONBOARDING_STEPS.length - 1, onboardingStep + 1);
    render();
    if (onboardingStep === 3) document.querySelector<HTMLTextAreaElement>("#onboarding-try")?.focus();
  }));
  document.querySelectorAll<HTMLButtonElement>("[data-onboarding-back]").forEach(button => button.addEventListener("click", () => {
    onboardingStep = Math.max(0, onboardingStep - 1);
    render();
  }));
  document.querySelectorAll<HTMLInputElement>('input[name="onboarding-model"]').forEach(radio => radio.addEventListener("change", () => {
    onboardingModel = radio.value;
    render();
  }));
  document.querySelector("#onboarding-model-go")?.addEventListener("click", async () => {
    const id = onboardingModel;
    if (!id) return;
    if (!statuses[id]?.installed) {
      await downloadModel(id);
      if (!statuses[id]?.installed) return;
    }
    await selectModel(id);
    onboardingStep = 3;
    render();
    document.querySelector<HTMLTextAreaElement>("#onboarding-try")?.focus();
  });
  const tryBox = document.querySelector<HTMLTextAreaElement>("#onboarding-try");
  tryBox?.addEventListener("input", () => {
    const hadText = !!onboardingTryText.trim();
    onboardingTryText = tryBox.value;
    // The first words turn "Skip for now" into "Next".
    if (hadText !== !!onboardingTryText.trim()) {
      const next = document.querySelector<HTMLButtonElement>("[data-onboarding-next]");
      if (next) next.textContent = onboardingTryText.trim() ? "Next" : "Skip for now";
    }
  });
}
async function downloadModel(id: string) {
  try {
    statuses[id] = { ...(statuses[id] ?? { installed: false, downloadable: true }), downloading: true, progress: 0, message: "Connecting…" };
    showToast(`Downloading ${models.find(model => model.id === id)?.name ?? "model"}…`);
    render();
    const timer = window.setInterval(() => refreshStatuses().then(render).catch(() => undefined), 500);
    await invoke("download_model", { modelId: id });
    window.clearInterval(timer); await refreshStatuses(); showToast("Model downloaded.");
  } catch (error) {
    await refreshStatuses().catch(() => undefined);
    const status = statuses[id];
    showToast(status?.message ? String(status.message) : String(error));
  }
  render();
}
async function deleteModel(id: string) {
  try { await invoke("delete_model", { modelId: id }); await refreshStatuses(); showToast("Model removed."); } catch (error) { showToast(String(error)); }
  render();
}
async function selectModel(id: string) {
  settings.selectedModel = id;
  try { await invoke("save_settings", { settings }); showToast(`${selected()?.name ?? "Model"} is active.`); render(); } catch (error) { showToast(String(error)); render(); }
}
async function toggleRecording() {
  if (testListening) return;
  try {
    if (!recording) {
      if (!modelInstalled()) {
        showToast(emptySpeechMessage());
        render();
        return;
      }
      await invoke("start_recording");
      recording = true;
      await refreshRuntime();
      render();
      return;
    }
    recording = false; render();
    const text = await invoke<string>("stop_and_transcribe");
    if (!text) { showToast(emptySpeechMessage()); render(); return; }
    await invoke("inject_text", { text }); await refreshHistory();
  } catch (error) { recording = false; showToast(String(error)); }
  await refreshRuntime();
  render();
}
async function toggleMicTest() {
  try {
    if (!micTesting) {
      await invoke("start_mic_test");
      micTesting = true;
      micPeak = 0;
      if (micMeterTimer) window.clearInterval(micMeterTimer);
      micMeterTimer = window.setInterval(async () => {
        try {
          micLevel = await invoke<number>("get_mic_level");
          micPeak = Math.max(micPeak, micLevel);
          document.querySelectorAll<HTMLElement>(".level-meter span").forEach(bar => {
            bar.style.width = `${Math.min(100, Math.round(micLevel * 140))}%`;
          });
        } catch { /* ignore */ }
      }, 80);
      render();
      return;
    }
    await invoke("stop_mic_test");
    micTesting = false;
    micLevel = 0;
    if (micMeterTimer) { window.clearInterval(micMeterTimer); micMeterTimer = null; }
  } catch (error) {
    const emptyCapture = String(error).includes("No microphone audio was captured");
    micTesting = false;
    micLevel = 0;
    if (micMeterTimer) { window.clearInterval(micMeterTimer); micMeterTimer = null; }
    if (!(emptyCapture && micPeak > 0)) {
      showToast(String(error));
    }
  }
  micPeak = 0;
  render();
}
async function testDictation() {
  if (runtime.paused || micTesting) return;
  try {
    if (!testListening) {
      if (!modelInstalled()) {
        showToast(emptySpeechMessage());
        render();
        return;
      }
      testingDictation = true;
      testListening = true;
      recording = true;
      testResult = "";
      render();
      await invoke("start_recording", { noInject: true });
      testingDictation = false;
      await refreshRuntime();
      render();
      return;
    }
    testingDictation = true;
    recording = false;
    testListening = false;
    render();
    const text = await invoke<string>("stop_and_transcribe");
    testResult = text || emptySpeechMessage();
    await refreshHistory();
  } catch (error) {
    recording = false;
    testListening = false;
    testResult = String(error);
    try { await invoke("stop_and_transcribe"); } catch { /* leftover session is cleared in rust */ }
  }
  testingDictation = false;
  await refreshRuntime();
  render();
}

window.addEventListener("keydown", onGlobalKeyDown, true);
window.addEventListener("keyup", onGlobalKeyUp, true);

/** While the setup guide's "Try it" box is up, dictation types into it; a
 *  full redraw mid-typing would drop keystrokes, so only the note updates. */
function paintOnboardingNote() {
  const note = document.querySelector(".onboarding-note");
  if (note) note.textContent = recording ? "Listening…" : "It works the same in any app: the text goes where the caret is.";
  paintSidebarStatus();
}

listen<boolean>("recording-changed", event => {
  recording = event.payload;
  if (!event.payload) testListening = false;
  if (onboardingTrying()) {
    paintOnboardingNote();
    void refreshRuntime();
    return;
  }
  refreshRuntime().then(render).catch(() => render());
}).catch(() => undefined);
listen<string>("dictation-finished", async () => {
  recording = false;
  await refreshHistory().catch(() => undefined);
  await refreshRuntime().catch(() => undefined);
  if (onboardingTrying()) {
    paintOnboardingNote();
    return;
  }
  if (view === "stats") await refreshStats();
  render();
}).catch(() => undefined);
listen("dictation-cancelled", () => {
  if (!onboardingTrying()) showToast("Cancelled. Nothing was typed.");
}).catch(() => undefined);
listen("history-changed", async () => {
  if (view !== "history" || onboardingTrying()) return;
  await refreshHistory().catch(() => undefined);
  render();
}).catch(() => undefined);
listen<string>("test-dictation-finished", async event => {
  recording = false;
  testListening = false;
  testResult = event.payload || emptySpeechMessage();
  await refreshHistory().catch(() => undefined);
  await refreshRuntime().catch(() => undefined);
  render();
}).catch(() => undefined);
listen<string>("dictation-error", event => {
  recording = false;
  showToast(event.payload);
  refreshRuntime().then(render).catch(() => render());
}).catch(() => undefined);
listen<RuntimeStatus>("runtime-status", event => {
  applyRuntimeStatus(event.payload);
}).catch(() => undefined);
listen<Settings>("settings-changed", event => {
  settings = { ...settings, ...event.payload };
  render();
}).catch(() => undefined);
listen<string>("navigate", event => {
  if (ALL_VIEWS.includes(event.payload as View)) openView(event.payload as View);
}).catch(() => undefined);
listen<LogLine>("log-line", event => {
  const line = event.payload;
  if (line && typeof line === "object" && "text" in line) {
    logLines = [...logLines.slice(-499), line];
  }
  if (view === "debug") render();
}).catch(() => undefined);

Promise.all([
  invoke<Model[]>("get_models"),
  invoke<Settings>("get_settings"),
  invoke<Record<string, ModelStatus>>("get_model_statuses"),
  invoke<HistoryEntry[]>("get_history"),
  invoke<HotkeyPreset[]>("get_hotkey_presets"),
  invoke<GpuStatus>("get_gpu_status"),
  invoke<InputDevice[]>("list_input_devices").catch(() => [] as InputDevice[]),
  invoke<ModelRecommendation>("recommend_model").catch(() => null),
  invoke<RuntimeStatus>("get_runtime_status").catch(() => runtime),
  invoke<LogLine[]>("get_log_lines").catch(() => [] as LogLine[]),
]).then(([catalog, saved, installs, entries, hotkeyPresets, gpuStatus, inputDevices, modelRec, runtimeStatus, logs]) => {
  models = catalog;
  settings = {
    ...saved,
    soundTheme: saved.soundTheme || (saved.soundEffects === false ? "off" : "voca"),
    soundEffects: saved.soundEffects ?? true,
    maxRecordingSeconds: saved.maxRecordingSeconds ?? 60,
    appendTrailingSpace: saved.appendTrailingSpace ?? true,
    autoCapitalize: saved.autoCapitalize ?? true,
    inputDevice: saved.inputDevice ?? "",
    autoPauseEnabled: saved.autoPauseEnabled ?? false,
    autoPauseApps: saved.autoPauseApps ?? "",
    idleUnloadEnabled: saved.idleUnloadEnabled ?? false,
    idleUnloadSeconds: saved.idleUnloadSeconds ?? 300,
    welcomeDismissed: saved.welcomeDismissed ?? false,
    historyEnabled: saved.historyEnabled ?? true,
    debugLogging: saved.debugLogging ?? false,
    customVocabulary: saved.customVocabulary ?? "",
    copyToClipboard: saved.copyToClipboard ?? false,
    cleanupLevel: saved.cleanupLevel ?? "medium",
    numbersAsDigits: saved.numbersAsDigits ?? false,
    numberSymbols: saved.numberSymbols ?? false,
    spokenEmoji: saved.spokenEmoji ?? false,
    replacements: saved.replacements ?? [],
    snippets: saved.snippets ?? [],
    escapeCancels: saved.escapeCancels ?? true,
    handsFreeHotkey: saved.handsFreeHotkey ?? "",
    pasteLastHotkey: saved.pasteLastHotkey ?? "",
    mouseButton: saved.mouseButton ?? "",
    overlayStyle: saved.overlayStyle ?? "minimal",
    overlayPosition: saved.overlayPosition ?? "bottom",
    readyPill: saved.readyPill ?? true,
    skipSilence: saved.skipSilence ?? true,
    muteOtherAudio: saved.muteOtherAudio ?? false,
    historyRetentionDays: saved.historyRetentionDays ?? 30,
    historyKeepAudio: saved.historyKeepAudio ?? true,
    insertionMode: saved.insertionMode ?? "type",
    pasteApps: saved.pasteApps ?? "",
  };
  statuses = installs;
  history = entries;
  presets = hotkeyPresets;
  gpu = {
    ...gpuStatus,
    deviceIndex: gpuStatus.deviceIndex ?? -1,
    discrete: gpuStatus.discrete ?? false,
    vramMb: gpuStatus.vramMb ?? 0,
    detail: gpuStatus.detail ?? "",
  };
  devices = inputDevices;
  recommendation = modelRec;
  runtime = runtimeStatus;
  logLines = logs;
  render();
}).catch(error => { app.textContent = `Could not start VocaWin: ${error}`; });
