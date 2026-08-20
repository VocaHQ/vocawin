import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./style.css";
import sidebarMark from "./assets/voca-logo.svg?raw";
import dictateIdle from "./assets/vocawin-dictate-idle.svg?raw";
import dictateListening from "./assets/vocawin-dictate-listening.svg?raw";
import discordMark from "./assets/social/discord.svg?raw";
import githubMark from "./assets/social/github.svg?raw";
import mailMark from "./assets/social/mail.svg?raw";
import xMark from "./assets/social/x.svg?raw";
import familyLogo from "../web/assets/brand/voca-logo-512.png";

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
};
type View = "dictation" | "models" | "history" | "settings" | "debug" | "about";
type HistoryEntry = { id: number; text: string; modelId: string; createdAtMs: number };
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
type RunningApp = { name: string; label: string };
type EngineFilter = "all" | "whisper" | "onnx";
type LanguageFilter = "any" | "english" | "multilingual";

type SettingsItem = {
  group: "Dictation" | "Audio" | "Application";
  title: string;
  subtitle: string;
  keywords: string;
  html: string;
};

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

const app = document.querySelector<HTMLDivElement>("#app")!;
let models: Model[] = [];
let statuses: Record<string, ModelStatus> = {};
let history: HistoryEntry[] = [];
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
let recordingHotkey = false;
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
const nav = (id: View, label: string, icon: string) => `<button class="nav ${view === id ? "active" : ""}" data-view="${id}"><span class="nav-icon">${icon}</span>${label}</button>`;

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
  const modeLabel = settings.activationMode === "toggle" ? "Toggle on and off" : "Press and hold to dictate";
  const parked = !!runtime.parkKind;
  const heading = recording
    ? "Listening…"
    : runtime.parkKind === "autopause"
      ? "Paused for a watched app"
      : runtime.parkKind === "idle"
        ? "Model unloaded to save RAM"
        : "Ready to dictate";
  const modelState = installed
    ? `${escape(model?.engine ?? "")} · ${escape(model?.languages ?? "")} · On this PC`
    : "Not on this PC yet. Open Models to download it.";
  const modelAction = installed ? "Change model" : "Download this model";
  const stateLabel = recording ? "Listening" : parked ? (runtime.parkKind === "idle" ? "Unloaded" : "Paused") : "Ready";
  return `<header><div><p class="overline">VOICE DICTATION</p><h1>Speak naturally.<br><em>Keep it private.</em></h1><p class="lede">VocaWin turns your voice into text on your own computer, never in the cloud.</p></div><span class="state ${parked ? "parked" : ""}"><i></i>${stateLabel}</span></header>
  ${dictationParkBanner()}
  <section class="record-panel"><div class="mic ${recording ? "listening" : parked ? "parked" : ""}">${dictateMark(recording ? "listening" : "idle")}</div><h2>${heading}</h2><p>${recording ? "Speak now, then stop when you are finished." : `Use ${escape(settings.hotkey)} or start below.`}</p><button class="primary" id="record" ${runtime.paused && !recording ? "disabled" : ""}>${recording ? "Stop & transcribe" : "Start dictation"}</button><small>Everything is processed locally on this device.</small></section>
  <section class="overview"><button class="info-card model-cta ${installed ? "" : "needs-download"}" data-go="models" type="button"><p class="card-label">ACTIVE MODEL</p><strong>${escape(model?.name ?? "Choose a model")}</strong><span>${modelState}</span><span class="text-button">${modelAction}</span></button><div class="info-card"><p class="card-label">ACTIVATION</p><strong>${escape(settings.hotkey)}</strong><span>${modeLabel}</span><button class="text-button" data-go="settings">Edit shortcut</button></div></section>`;
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
  return `<header><div><p class="overline">ON-DEVICE MODELS</p><h1>Choose your <em>engine.</em></h1><p class="lede">Models stay on your PC. Pick the trade-off between speed, accuracy, and language coverage.</p></div></header>
  ${tip}
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

function historyPage() {
  const entries = history.length
    ? history.map(entry => `<article class="history-entry"><p>${escape(entry.text)}</p><footer>${escape(models.find(model => model.id === entry.modelId)?.name ?? entry.modelId)} · ${new Date(entry.createdAtMs).toLocaleString()}</footer></article>`).join("")
    : `<div class="empty-history">${settings.historyEnabled ? "Your local transcription history will appear here." : "Nothing is saved yet. Turn history back on in Settings if you want new takes kept on this PC."}</div>`;
  const lede = settings.historyEnabled
    ? "History is stored only on this computer and can be cleared at any time."
    : "New takes are not being saved. Older entries stay on this PC until you clear them.";
  return `<header><div><p class="overline">LOCAL HISTORY</p><h1>Your recent <em>dictation.</em></h1><p class="lede">${lede}</p></div>${history.length ? `<button class="quiet-button" id="clear-history">Clear history</button>` : ""}</header><section class="history-list">${entries}</section>`;
}

function chipLabel(name: string) {
  return runningApps.find(app => app.name.toLowerCase() === name.toLowerCase())?.label
    ?? name.replace(/\.exe$/i, "");
}

function runningAppOptions() {
  const watched = new Set(watchedApps().map(name => name.toLowerCase()));
  const options = runningApps
    .filter(app => !watched.has(app.name.toLowerCase()))
    .map(app => `<option value="${escape(app.name)}">${escape(app.label)}</option>`);
  if (!options.length) {
    return `<option value="">No other running apps right now</option>`;
  }
  return `<option value="">Add a running app</option>${options.join("")}`;
}

function watchedAppChips() {
  const apps = watchedApps();
  if (!apps.length) return "";
  return `<ul class="app-chips">${apps.map(name => `<li class="app-chip"><span>${escape(chipLabel(name))}</span><button type="button" data-unwatch="${escape(name)}" title="Remove ${escape(chipLabel(name))}" aria-label="Remove ${escape(chipLabel(name))}">×</button></li>`).join("")}</ul>`;
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

function powerMatches(query: string) {
  if (!query) return true;
  const hay = "power pause while these apps are running voca stays quiet so they can use the mic unload the model after idle frees ram next dictation loads it again never minutes hour autopause";
  return query.split(/\s+/).every(part => hay.includes(part));
}

function powerSection() {
  return `<section class="settings-card power-card" data-settings-group="Power"><p class="settings-group">Power</p>
    <div class="setting-row">
      <div><strong>Pause while these apps are running</strong><p>Voca stays quiet so they can use the mic.</p></div>
      <select id="auto-pause-app" class="themed-select power-combo">${runningAppOptions()}</select>
    </div>
    <div class="power-chips">
      <div id="watched-app-chips">${watchedAppChips()}</div>
      <p class="power-note">Empty list means off. Each chip removes that app.</p>
    </div>
    <div class="setting-row">
      <div><strong>Unload the model after idle</strong><p>Frees RAM. Next dictation loads it again.</p></div>
      <select id="idle-unload" class="themed-select power-combo">${idleUnloadOptions()}</select>
    </div>
  </section>`;
}

function previewSoundControl() {
  const label = previewStartNext ? "Preview start" : "Preview end";
  const icon = previewStartNext ? ICON_PLAY : ICON_STOP;
  const off = settings.soundTheme === "off";
  return `<button type="button" class="quiet-button preview-sound" id="preview-sound" ${off ? "disabled " : ""}aria-label="${label}" title="${label}">${icon}</button>`;
}

function settingsItems(): SettingsItem[] {
  const levelPct = Math.min(100, Math.round(micLevel * 140));
  return [
    {
      group: "Dictation",
      title: "Activation hotkey",
      subtitle: "Pick a preset or press Record. New installs default to Right Alt (Option), the same hold-default as VocaLinux. AltGr (Ctrl+Right Alt) is not consumed, so layout characters still type. Escape cancels. The live listener pauses while recording.",
      keywords: "hotkey shortcut keyboard record preset right alt altright",
      html: `<div class="hotkey-controls"><select id="hotkey-preset" class="themed-select">${hotkeyOptions()}</select>
    <button type="button" class="quiet-button" id="record-hotkey">${recordingHotkey ? "Cancel" : "Record"}</button></div>`,
    },
    {
      group: "Dictation",
      title: "Activation style",
      subtitle: "Hold to talk, or tap to toggle. Toggle uses silence auto-stop.",
      keywords: "push to talk toggle mode",
      html: `<select id="activation" class="themed-select"><option value="pushToTalk">Push to talk</option><option value="toggle">Toggle</option></select>`,
    },
    {
      group: "Dictation",
      title: "Dictation language",
      subtitle: "One list. Auto-detect is first, then English, then A to Z.",
      keywords: "language locale english auto detect",
      html: languageControl(),
    },
    {
      group: "Dictation",
      title: "Auto-capitalize",
      subtitle: "Capitalize the start of sentences.",
      keywords: "capitalize formatting output",
      html: `<label class="switch"><input id="auto-cap" type="checkbox" ${settings.autoCapitalize ? "checked" : ""}/><span></span></label>`,
    },
    {
      group: "Dictation",
      title: "Trailing space",
      subtitle: "Append a space after each utterance.",
      keywords: "space formatting output",
      html: `<label class="switch"><input id="trailing-space" type="checkbox" ${settings.appendTrailingSpace ? "checked" : ""}/><span></span></label>`,
    },
    {
      group: "Dictation",
      title: "Custom Vocabulary",
      subtitle: "Bias Whisper toward names and jargon. It is a hint, not a guarantee.",
      keywords: "custom vocabulary dictionary glossary names jargon initial prompt whisper",
      html: `<textarea id="custom-vocabulary" rows="5" placeholder="kubectl, PostgreSQL, nginx, Grafana">${escape(settings.customVocabulary)}</textarea>`,
    },
    {
      group: "Audio",
      title: "Microphone",
      subtitle: "WASAPI capture device used for dictation.",
      keywords: "mic microphone device wasapi input",
      html: `<select id="input-device" class="themed-select">${deviceOptions()}</select>`,
    },
    {
      group: "Audio",
      title: "Mic Test",
      subtitle: "Level meter only. Does not recognize or inject text.",
      keywords: "mic test level meter volume",
      html: `<div class="mic-test"><button type="button" class="quiet-button" id="mic-test" ${recording || testListening ? "disabled" : ""}>${micTesting ? "Stop Mic Test" : "Mic Test"}</button>
      <div class="level-meter" aria-hidden="true"><span style="width:${levelPct}%"></span></div></div>`,
    },
    {
      group: "Audio",
      title: "Silence auto-stop",
      subtitle: "Seconds of quiet before toggle mode ends a take. Push-to-talk ignores this and stops on key-up.",
      keywords: "vad silence timeout toggle",
      html: `<input id="silence" type="number" min="0.3" max="10" step="0.1" value="${settings.silenceSeconds}" />`,
    },
    {
      group: "Audio",
      title: "Max recording",
      subtitle: "Hard stop so a stuck session cannot run forever.",
      keywords: "duration limit max",
      html: `<input id="max-recording" type="number" min="3" max="300" step="1" value="${settings.maxRecordingSeconds}" />`,
    },
    {
      group: "Audio",
      title: "Dictation sounds",
      subtitle: "These play when listening starts and stops. Preview is two clicks: start tone, then end tone.",
      keywords: "sound beep audio cue",
      html: `<div class="sound-theme-controls"><select id="sound-theme" class="themed-select">${soundThemeOptions()}</select>
      ${previewSoundControl()}</div>`,
    },
    {
      group: "Application",
      title: "Launch at login",
      subtitle: "Start VocaWin with Windows for this user (starts minimized).",
      keywords: "startup autostart login",
      html: `<label class="switch"><input id="launch-login" type="checkbox" ${settings.launchAtLogin ? "checked" : ""}/><span></span></label>`,
    },
    {
      group: "Application",
      title: "Keep dictation history",
      subtitle: "When this is off, new takes are not added to History. Older entries stay until you clear them.",
      keywords: "history transcript save local",
      html: `<label class="switch"><input id="history-enabled" type="checkbox" ${settings.historyEnabled ? "checked" : ""}/><span></span></label>`,
    },
  ];
}

function settingsPage() {
  const query = settingsQuery.trim().toLowerCase();
  const items = settingsItems().filter(item => {
    if (!query) return true;
    const hay = `${item.group} ${item.title} ${item.subtitle} ${item.keywords}`.toLowerCase();
    return hay.includes(query);
  });
  const groups: Array<SettingsItem["group"]> = ["Dictation", "Audio", "Application"];
  const cards = groups.map(group => {
    const rows = items.filter(item => item.group === group);
    if (!rows.length) return "";
    return `<section class="settings-card" data-settings-group="${group}"><p class="settings-group">${group}</p>
      ${rows.map(item => `<div class="setting-row"><div><strong>${escape(item.title)}</strong><p>${escape(item.subtitle)}</p></div>${item.html}</div>`).join("")}
      </section>`;
  }).join("");
  const power = powerMatches(query) ? powerSection() : "";
  return `<header><div><p class="overline">PREFERENCES</p><h1>Make it <em>yours.</em></h1><p class="lede">VocaWin only stores these choices locally on this PC. Each change is saved as you make it.</p></div></header>
  <div class="settings-search"><input id="settings-search" type="search" placeholder="Search settings" value="${escape(settingsQuery)}" /></div>
  ${cards}${power || (cards ? "" : `<div class="empty-history">No settings match “${escape(settingsQuery)}”.</div>`)}
  ${recordingHotkey ? `<p class="recording-hint">Press a key combo, or Escape to cancel.</p>` : ""}`;
}

function debugPage() {
  const lines = visibleLogs();
  const body = lines.length
    ? lines.map(line => `<div class="log-line level-${escape(line.level)}"><span class="log-level">${escape(line.level)}</span>${escape(line.text)}</div>`).join("")
    : `<div class="empty-history">No warning or error lines yet.${settings.debugLogging ? "" : " Turn on debug logging to see the quieter chatter."}</div>`;
  return `<header><div><p class="overline">DEBUG</p><h1>GPU and <em>logs.</em></h1><p class="lede">This is for testers. Debug logging stays off unless you turn it on. Clear only wipes the in-memory buffer, not files on disk.</p></div></header>
    <section class="settings-card"><p class="settings-group">GPU</p>
      <div class="setting-row"><div><strong>${escape(gpu.name)}</strong><p>${escape(gpu.detail || gpu.backend)}</p></div>
      <div class="gpu-readout"><span>${escape(gpu.backend)}${gpu.discrete ? " · discrete" : ""}${gpu.vramMb ? ` · ~${gpu.vramMb} MB` : ""}</span></div></div>
    </section>
    <section class="settings-card"><p class="settings-group">Logs</p>
      <div class="setting-row"><div><strong>Debug logging</strong><p>Off shows warning and error. On also shows debug and info.</p></div>
        <label class="switch"><input id="debug-logging" type="checkbox" ${settings.debugLogging ? "checked" : ""}/><span></span></label>
      </div>
      <div class="log-toolbar">
        <button type="button" class="quiet-button" id="copy-logs">Copy</button>
        <button type="button" class="quiet-button" id="clear-logs">Clear</button>
      </div>
      <section class="log-panel">${body}</section>
    </section>`;
}

function aboutPage() {
  return `<header><div><p class="overline">ABOUT</p><h1>VocaWin <em>alpha.</em></h1></div></header>
    <section class="about-hero">
      <img class="about-logo" src="${familyLogo}" width="96" height="96" alt="Voca" />
      <h2>VocaWin</h2>
      <p class="about-tagline">Voice-to-text for Windows, kept on this PC.</p>
      <button type="button" class="text-button" data-open="https://vocawin.com">vocawin.com</button>
    </section>
    <section class="settings-card">
      <p class="settings-group">This build</p>
      <div class="about-copy">
        <p>This is an unsigned developer alpha. It is not a store listing and not a stable public release. Windows will likely say the publisher is unknown. That is SmartScreen. Use More info, then Run anyway, only if you trust the GitHub Release you downloaded.</p>
        <p>The community is expected to help improve it. If something breaks, file an issue.</p>
      </div>
    </section>
    <section class="settings-card">
      <p class="settings-group">Part of VocaHQ</p>
      <div class="about-copy">
        <p>VocaWin is one of the VocaHQ apps. The same private dictation already runs on Linux as VocaLinux, on macOS as VocaMac, and on phones as VocaPhone. VocaGateway is optional self-hosted compute for other Voca clients.</p>
        <ul class="about-links">
          <li><button type="button" class="text-button" data-open="https://vocahq.com">vocahq.com</button></li>
          <li><button type="button" class="text-button" data-open="https://vocalinux.com">vocalinux.com</button></li>
          <li><button type="button" class="text-button" data-open="https://vocamac.com">vocamac.com</button></li>
          <li><button type="button" class="text-button" data-open="https://vocaphone.vocahq.com">vocaphone.vocahq.com</button></li>
          <li><button type="button" class="text-button" data-open="https://vocagateway.vocahq.com">vocagateway.vocahq.com</button></li>
        </ul>
      </div>
    </section>
    <section class="settings-card">
      <p class="settings-group">Talk to us</p>
      <div class="about-copy">
        <p>Bugs, feedback, and feature ideas open a new GitHub issue. You pick the template on the next screen.</p>
        <button type="button" class="primary about-report" data-open="https://github.com/VocaHQ/vocawin/issues/new/choose">${githubMark}<span>Report a bug or idea</span></button>
        <ul class="about-talk" role="list">
          <li><button type="button" class="about-talk-btn" data-open="https://discord.gg/UMJduhcqn">${discordMark}<span>Discord</span></button></li>
          <li><button type="button" class="about-talk-btn" data-open="https://x.com/vocahq">${xMark}<span>X</span></button></li>
          <li><button type="button" class="about-talk-btn" data-open="mailto:hello@vocahq.com">${mailMark}<span>Email</span></button></li>
        </ul>
      </div>
    </section>`;
}

function welcomeOverlay() {
  if (settings.welcomeDismissed) return "";
  return `<div class="welcome-overlay" role="dialog" aria-modal="true" aria-labelledby="welcome-title">
    <div class="welcome-card">
      <p class="overline">WELCOME</p>
      <h2 id="welcome-title">VocaWin is in your tray</h2>
      <p>Hold your hotkey (Right Alt by default, like VocaLinux) to dictate into any app. Optional: turn on Start on Login from the tray menu or Settings.</p>
      <button class="primary" id="welcome-dismiss">Got it</button>
    </div>
  </div>`;
}

function sidebarFooter() {
  const parked = !!runtime.parkKind && !recording;
  const statusLabel = sidebarStatusLabel();
  const result = testResult
    ? `<p class="sidebar-result">${escape(testResult)}</p>`
    : `<p class="sidebar-result muted">Test dictation stays here. If history is on, this take is saved there too.</p>`;
  return `<div class="sidebar-footer">
    <div class="sidebar-status ${parked ? "parked" : recording ? "live" : ""}"><i></i><div><strong>${statusLabel}</strong>${runtime.parkDetail && !recording ? `<small>${escape(runtime.parkDetail)}</small>` : `<small>${escape(runtime.inputDevice)}</small>`}</div></div>
    ${toastMarkup()}
    <button type="button" class="quiet-button sidebar-test" id="test-dictation" ${runtime.paused || micTesting ? "disabled" : ""}>${testListening ? "Stop test" : testingDictation ? "Testing…" : "Test dictation"}</button>
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

function render() {
  captureChrome();
  const pages: Record<View, () => string> = {
    dictation: dictationPage,
    models: modelsPage,
    history: historyPage,
    settings: settingsPage,
    debug: debugPage,
    about: aboutPage,
  };
  app.innerHTML = `<aside>
    <div class="brand"><span class="mark">${sidebarMark}</span><span>VocaWin</span><span class="brand-tag" title="Developer-only build">Alpha</span></div>
    <p class="brand-subtitle">Voice dictation, kept private.</p>
    <nav>${nav("dictation", "Dictation", "◉")}${nav("models", "Models", "◇")}${nav("history", "History", "≡")}${nav("settings", "Settings", "⚙")}${nav("debug", "Debug", "⌗")}${nav("about", "About", "ⓘ")}</nav>
    ${sidebarFooter()}
  </aside>
  <main>
    ${pages[view]()}
    ${welcomeOverlay()}
  </main>`;
  bindChrome();
  restoreFocusedField();
  restorePaneScroll();
}

function openView(next: View) {
  view = next;
  resetPaneScroll = true;
  if (next === "settings") {
    void Promise.all([refreshRunningApps(), refreshRuntime()]).then(render);
    return;
  }
  if (next === "debug") {
    void refreshLogs().then(render);
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
  bindLiveSearch("#settings-search", value => { settingsQuery = value; });
  bindLiveSearch("#model-search", value => { modelQuery = value; });
  document.querySelector("#record-hotkey")?.addEventListener("click", () => {
    void toggleHotkeyRecording();
  });
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
    if (target.id === "settings-search" || target.id === "model-search" || target.id === "engine-filter" || target.id === "language-filter" || target.id === "auto-pause-app") return;
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
}

async function persistSettings(silent = false, skipCollect = false) {
  if (!skipCollect) collectSettingsFromDom();
  try {
    await invoke("save_settings", { settings });
    settings = await invoke<Settings>("get_settings");
    await refreshRuntime();
    if (view === "debug") {
      await refreshLogs();
      render();
      if (!silent) showToast("Settings saved");
      return;
    }
    syncSettingsControls();
    if (!silent) showToast("Settings saved");
  } catch (error) {
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

async function toggleHotkeyRecording() {
  if (recordingHotkey) {
    recordingHotkey = false;
    showToast("Hotkey recording cancelled.");
    try { await invoke("resume_hotkey_listener"); } catch { /* ignore */ }
    render();
    return;
  }
  try { await invoke("pause_hotkey_listener"); } catch { /* ignore */ }
  recordingHotkey = true;
  render();
}

function finishHotkeyCapture(spec: string, label: string) {
  settings.hotkey = spec;
  recordingHotkey = false;
  void invoke("resume_hotkey_listener").catch(() => undefined);
  void persistSettings(true, true).then(() => {
    syncSettingsControls();
    showToast(`Hotkey set to ${label}.`);
  });
}

function onGlobalKeyDown(event: KeyboardEvent) {
  if (!recordingHotkey) return;
  event.preventDefault();
  event.stopPropagation();
  if (event.key === "Escape") {
    recordingHotkey = false;
    showToast("Hotkey recording cancelled.");
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
async function refreshRunningApps() {
  try { runningApps = await invoke<RunningApp[]>("list_running_apps"); } catch { runningApps = []; }
}
async function clearHistory() {
  try { await invoke("clear_history"); history = []; showToast("History cleared."); } catch (error) { showToast(String(error)); }
  render();
}
async function copyLogs() {
  const text = visibleLogs().map(line => `[${line.level}] ${line.text}`).join("\n");
  if (!text) {
    showToast("No logs to copy.");
    render();
    return;
  }
  try {
    await invoke("copy_text", { text });
    showToast("Logs copied.");
  } catch {
    try {
      await navigator.clipboard.writeText(text);
      showToast("Logs copied.");
    } catch {
      showToast("Could not copy logs.");
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
    await invoke("dismiss_welcome");
    settings.welcomeDismissed = true;
  } catch (error) {
    showToast(String(error));
  }
  render();
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
      await invoke("start_recording", { noInject: true });
      recording = true;
      testListening = true;
      testingDictation = false;
      testResult = "";
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

listen<boolean>("recording-changed", event => {
  recording = event.payload;
  if (!event.payload) testListening = false;
  refreshRuntime().then(render).catch(() => render());
}).catch(() => undefined);
listen<string>("dictation-finished", async () => {
  recording = false;
  await refreshHistory().catch(() => undefined);
  await refreshRuntime().catch(() => undefined);
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
  if (event.payload === "settings" || event.payload === "models" || event.payload === "history" || event.payload === "dictation" || event.payload === "debug" || event.payload === "about") {
    openView(event.payload);
  }
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
