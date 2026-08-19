import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./style.css";

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
};
type View = "dictation" | "models" | "history" | "settings";
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
  hotkey: string;
  inputDevice: string;
  gpuName: string;
  gpuBackend: string;
  gpuDetail?: string;
};

type SettingsItem = {
  group: "Dictation" | "Audio" | "Application";
  title: string;
  subtitle: string;
  keywords: string;
  html: string;
};

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

const LANGUAGES = [
  "Auto-detect",
  "English",
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

const app = document.querySelector<HTMLDivElement>("#app")!;
const isLogsWindow = location.hash === "#logs";
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
  hotkey: "",
  inputDevice: "Default microphone",
  gpuName: "",
  gpuBackend: "",
};
let recording = false;
let recordingHotkey = false;
let testingDictation = false;
let testListening = false;
let micTesting = false;
let micLevel = 0;
let micMeterTimer: number | null = null;
let settingsQuery = "";
let languageQuery = "";
let view: View = "dictation";
let noticeText = "";
let showAbout = false;
let logLines: string[] = [];
let previewStartNext = true;

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

function modelAction(model: Model, status?: ModelStatus) {
  if (status?.downloading) {
    return `<button class="model-action brand-action" data-download="${model.id}" disabled>Downloading ${status.progress}%</button>`;
  }
  if (status?.installed) {
    return `<button class="model-action" data-delete="${model.id}">Remove</button>`;
  }
  return `<button class="model-action brand-action" data-download="${model.id}">Download</button>`;
}

function modelCards() {
  return models.map(model => {
    const status = statuses[model.id];
    const failed = !status?.installed && !status?.downloading && !!status?.message
      && status.message !== "Installed";
    const state = status?.downloading
      ? `Downloading ${status.progress}%`
      : status?.installed
        ? status.bytesOnDisk
          ? `Installed · ${formatBytes(status.bytesOnDisk)}`
          : "Installed"
        : failed
          ? `Failed: ${status?.message}`
          : "Not installed";
    const isSelected = model.id === settings.selectedModel;
    const recommended = recommendation?.modelId === model.id;
    return `<article class="model-card ${isSelected ? "selected" : ""}" data-model="${model.id}" role="button" tabindex="0" aria-pressed="${isSelected}">
      <span class="check" aria-hidden="true">${isSelected ? "✓" : ""}</span>
      <b>${escape(model.name)}</b>
      <span class="engine">${escape(model.engine)}${recommended ? " · Suggested start" : ""}</span>
      <small>${escape(model.description)}</small>
      <ul class="model-meta">
        <li>${escape(model.size)}</li>
        <li>${escape(model.languages)}</li>
        <li class="install-state ${failed ? "failed" : ""}">${escape(state)}</li>
      </ul>
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
  const query = languageQuery.trim().toLowerCase();
  const list = LANGUAGES.filter(language => !query || language.toLowerCase().includes(query));
  const selectedLanguage = settings.language || "Auto-detect";
  const options = list.map(language =>
    `<option value="${escape(language)}" ${language === selectedLanguage ? "selected" : ""}>${escape(language)}</option>`
  );
  if (selectedLanguage && !list.includes(selectedLanguage)) {
    options.unshift(`<option value="${escape(selectedLanguage)}" selected>${escape(selectedLanguage)}</option>`);
  }
  return options.join("");
}

function dictationPage() {
  const model = selected();
  const modeLabel = settings.activationMode === "toggle" ? "Toggle on and off" : "Press and hold to dictate";
  return `<header><div><p class="overline">VOICE DICTATION</p><h1>Speak naturally.<br><em>Keep it private.</em></h1><p class="lede">VocaWin turns your voice into text on your own computer — never in the cloud.</p></div><span class="state"><i></i>${recording ? "Listening" : runtime.paused ? "Paused" : "Ready"}</span></header>
  <section class="record-panel"><div class="mic ${recording ? "recording" : ""}">${recording ? "❚❚" : "⌁"}</div><h2>${recording ? "Listening…" : runtime.paused ? "Paused for a watched app" : "Ready to dictate"}</h2><p>${recording ? "Speak now, then stop when you are finished." : `Use ${escape(settings.hotkey)} or start below.`}</p><button class="primary" id="record" ${runtime.paused && !recording ? "disabled" : ""}>${recording ? "Stop & transcribe" : "Start dictation"}</button><small>Everything is processed locally on this device.</small></section>
  <section class="overview"><div class="info-card"><p class="card-label">ACTIVE MODEL</p><strong>${escape(model?.name ?? "Choose a model")}</strong><span>${escape(model?.engine ?? "")} · ${escape(model?.languages ?? "")}</span><button class="text-button" data-go="models">Change model →</button></div><div class="info-card"><p class="card-label">ACTIVATION</p><strong>${escape(settings.hotkey)}</strong><span>${modeLabel}</span><button class="text-button" data-go="settings">Edit shortcut →</button></div></section>`;
}

function modelsPage() {
  const tip = recommendation
    ? `<p class="hw-tip"><strong>Starting size:</strong> ${escape(recommendation.modelName)}. ${escape(recommendation.reason)}</p>`
    : "";
  return `<header><div><p class="overline">ON-DEVICE MODELS</p><h1>Choose your <em>engine.</em></h1><p class="lede">Models stay on your PC. Pick the trade-off between speed, accuracy, and language coverage. Click a card to select it.</p></div></header>
  ${tip}
  <div class="model-grid">${modelCards()}</div>`;
}

function historyPage() {
  const entries = history.length ? history.map(entry => `<article class="history-entry"><p>${escape(entry.text)}</p><footer>${escape(models.find(model => model.id === entry.modelId)?.name ?? entry.modelId)} · ${new Date(entry.createdAtMs).toLocaleString()}</footer></article>`).join("") : `<div class="empty-history">Your local transcription history will appear here.</div>`;
  return `<header><div><p class="overline">LOCAL HISTORY</p><h1>Your recent <em>dictation.</em></h1><p class="lede">History is stored only on this computer and can be cleared at any time.</p></div>${history.length ? `<button class="quiet-button" id="clear-history">Clear history</button>` : ""}</header><section class="history-list">${entries}</section>`;
}

function settingsItems(): SettingsItem[] {
  const levelPct = Math.min(100, Math.round(micLevel * 140));
  return [
    {
      group: "Dictation",
      title: "Activation hotkey",
      subtitle: "Pick a preset or press Record. New installs default to Right Alt (same hold-default as VocaLinux). AltGr (Ctrl+Right Alt) is not consumed, so layout characters still type. Escape cancels. The live listener pauses while recording.",
      keywords: "hotkey shortcut keyboard record preset right alt altright",
      html: `<div class="hotkey-controls"><select id="hotkey-preset">${hotkeyOptions()}</select>
    <button type="button" class="quiet-button" id="record-hotkey">${recordingHotkey ? "Cancel" : "Record"}</button></div>`,
    },
    {
      group: "Dictation",
      title: "Activation style",
      subtitle: "Hold to talk, or tap to toggle. Toggle uses silence auto-stop.",
      keywords: "push to talk toggle mode",
      html: `<select id="activation"><option value="pushToTalk">Push to talk</option><option value="toggle">Toggle</option></select>`,
    },
    {
      group: "Dictation",
      title: "Dictation language",
      subtitle: "Search the list. Auto-detect is best for multilingual speech.",
      keywords: "language locale english auto detect search",
      html: `<div class="language-picker"><input id="language-filter" type="search" placeholder="Search languages" value="${escape(languageQuery)}" />
      <select id="language">${languageOptions()}</select></div>`,
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
      group: "Audio",
      title: "Microphone",
      subtitle: "WASAPI capture device used for dictation.",
      keywords: "mic microphone device wasapi input",
      html: `<select id="input-device">${deviceOptions()}</select>`,
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
      subtitle: "These play when listening starts and stops.",
      keywords: "sound beep audio cue",
      html: `<div class="sound-theme-controls"><select id="sound-theme">${soundThemeOptions()}</select>
      <button type="button" class="quiet-button" id="preview-sound" ${settings.soundTheme === "off" ? "disabled" : ""}>Preview</button></div>`,
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
      title: "Auto-pause apps",
      subtitle: "While these processes run, unload the hotkey so games and capture tools keep their keys. One name per line.",
      keywords: "pause game fortnite obs process",
      html: `<div class="stacked-control"><label class="switch"><input id="auto-pause" type="checkbox" ${settings.autoPauseEnabled ? "checked" : ""}/><span></span></label>
      <textarea id="auto-pause-apps" rows="3" placeholder="obs64.exe&#10;fortnite.exe">${escape(settings.autoPauseApps)}</textarea></div>`,
    },
    {
      group: "Application",
      title: "Idle model unload",
      subtitle: "Keep Whisper loaded between takes, then unload after idle seconds. Off loads per utterance.",
      keywords: "idle unload keepalive memory model",
      html: `<div class="stacked-control"><label class="switch"><input id="idle-unload" type="checkbox" ${settings.idleUnloadEnabled ? "checked" : ""}/><span></span></label>
      <input id="idle-unload-seconds" type="number" min="30" max="3600" step="30" value="${settings.idleUnloadSeconds}" /></div>`,
    },
    {
      group: "Application",
      title: "GPU",
      subtitle: gpu.detail || gpu.backend,
      keywords: "gpu vulkan directml cuda hardware discrete",
      html: `<div class="gpu-readout"><strong>${escape(gpu.name)}</strong><span>${escape(gpu.backend)}${gpu.discrete ? " · discrete" : ""}${gpu.vramMb ? ` · ~${gpu.vramMb} MB` : ""}</span></div>`,
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
  return `<header><div><p class="overline">PREFERENCES</p><h1>Make it <em>yours.</em></h1><p class="lede">VocaWin only stores these choices locally on this PC.</p></div></header>
  <div class="settings-search"><input id="settings-search" type="search" placeholder="Search settings" value="${escape(settingsQuery)}" /></div>
  ${cards || `<div class="empty-history">No settings match “${escape(settingsQuery)}”.</div>`}
  <div class="settings-save"><button class="primary" id="save">Save changes</button></div>
  <footer class="settings-status">
    <div><strong>${escape(runtime.status)}</strong><span>${escape(runtime.inputDevice)} · ${escape(runtime.gpuName || gpu.name)}</span></div>
    <div class="status-actions">
      <button type="button" class="quiet-button" id="mic-test-footer" ${recording || testListening ? "disabled" : ""}>${micTesting ? "Stop Mic Test" : "Mic Test"}</button>
      <button type="button" class="quiet-button" id="test-dictation" ${runtime.paused || micTesting ? "disabled" : ""}>${testListening ? "Stop test" : testingDictation ? "Testing…" : "Test Dictation"}</button>
    </div>
  </footer>
  ${recordingHotkey ? `<p class="notice recording-hint">Press a key combo, or Escape to cancel.</p>` : ""}`;
}

function welcomeOverlay() {
  if (settings.welcomeDismissed || isLogsWindow) return "";
  return `<div class="welcome-overlay" role="dialog" aria-modal="true" aria-labelledby="welcome-title">
    <div class="welcome-card">
      <p class="overline">WELCOME</p>
      <h2 id="welcome-title">VocaWin is in your tray</h2>
      <p>Hold your hotkey (Right Alt by default, like VocaLinux) to dictate into any app. Optional: turn on Start on Login from the tray menu or Settings.</p>
      <button class="primary" id="welcome-dismiss">Got it</button>
    </div>
  </div>`;
}

function aboutOverlay() {
  if (!showAbout) return "";
  return `<div class="welcome-overlay" role="dialog" aria-modal="true">
    <div class="welcome-card">
      <p class="overline">ABOUT</p>
      <h2>VocaWin</h2>
      <p>Private, offline voice dictation for Windows. Audio and models stay on this PC.</p>
      <button class="primary" id="about-dismiss">Close</button>
    </div>
  </div>`;
}

function logsPage() {
  const body = logLines.length
    ? logLines.map(line => `<div class="log-line">${escape(line)}</div>`).join("")
    : `<div class="empty-history">No log lines yet.</div>`;
  return `<header><div><p class="overline">DIAGNOSTICS</p><h1>App <em>logs.</em></h1><p class="lede">Recent messages from this session. Opening View Logs again reuses this window.</p></div>
    <button class="quiet-button" id="clear-logs">Clear</button></header>
    <section class="log-panel">${body}</section>`;
}

function render() {
  if (isLogsWindow) {
    app.innerHTML = `<main class="logs-main">${logsPage()}</main>`;
    document.querySelector("#clear-logs")?.addEventListener("click", async () => {
      await invoke("clear_log_lines");
      logLines = [];
      render();
    });
    return;
  }

  const pages: Record<View, () => string> = { dictation: dictationPage, models: modelsPage, history: historyPage, settings: settingsPage };
  app.innerHTML = `<aside><div class="brand"><span class="mark"><img src="/src/assets/voca-logo.svg" alt="Voca"/></span><span>VocaWin</span><span class="brand-tag" title="Developer-only build">Alpha</span></div><p class="brand-subtitle">Voice dictation, kept private.</p><nav>${nav("dictation", "Dictation", "◉")}${nav("models", "Models", "◇")}${nav("history", "History", "≡")}${nav("settings", "Settings", "⚙")}</nav><div class="privacy"><i>✓</i><div><b>Private by default</b><small>Your audio stays here</small></div></div></aside><main>${pages[view]()}${welcomeOverlay()}${aboutOverlay()}<p class="notice" role="status">${escape(noticeText)}</p></main>`;
  document.querySelectorAll<HTMLButtonElement>("[data-view]").forEach(button => button.addEventListener("click", () => { view = button.dataset.view as View; render(); }));
  document.querySelectorAll<HTMLButtonElement>("[data-go]").forEach(button => button.addEventListener("click", () => { view = button.dataset.go as View; render(); }));
  document.querySelector("#record")?.addEventListener("click", toggleRecording);
  document.querySelector("#save")?.addEventListener("click", save);
  document.querySelector("#clear-history")?.addEventListener("click", clearHistory);
  document.querySelector("#test-dictation")?.addEventListener("click", testDictation);
  document.querySelector("#mic-test")?.addEventListener("click", toggleMicTest);
  document.querySelector("#mic-test-footer")?.addEventListener("click", toggleMicTest);
  document.querySelector("#welcome-dismiss")?.addEventListener("click", dismissWelcome);
  document.querySelector("#about-dismiss")?.addEventListener("click", () => { showAbout = false; render(); });
  document.querySelector("#settings-search")?.addEventListener("input", event => {
    settingsQuery = (event.target as HTMLInputElement).value;
    const active = document.activeElement === event.target;
    render();
    if (active) {
      const input = document.querySelector<HTMLInputElement>("#settings-search");
      if (input) {
        input.focus();
        input.setSelectionRange(settingsQuery.length, settingsQuery.length);
      }
    }
  });
  document.querySelector("#language-filter")?.addEventListener("input", event => {
    languageQuery = (event.target as HTMLInputElement).value;
    const active = document.activeElement === event.target;
    render();
    if (active) {
      const input = document.querySelector<HTMLInputElement>("#language-filter");
      if (input) {
        input.focus();
        input.setSelectionRange(languageQuery.length, languageQuery.length);
      }
    }
  });
  document.querySelector("#record-hotkey")?.addEventListener("click", () => {
    void toggleHotkeyRecording();
  });
  const soundTheme = document.querySelector<HTMLSelectElement>("#sound-theme");
  const previewSound = document.querySelector<HTMLButtonElement>("#preview-sound");
  soundTheme?.addEventListener("change", () => {
    previewStartNext = true;
    if (previewSound) previewSound.disabled = soundTheme.value === "off";
  });
  previewSound?.addEventListener("click", async () => {
    const theme = soundTheme?.value ?? settings.soundTheme;
    if (theme === "off") return;
    try {
      await invoke("preview_sound", { theme, start: previewStartNext });
      previewStartNext = !previewStartNext;
    } catch (error) {
      noticeText = String(error);
      render();
    }
  });
  const language = document.querySelector<HTMLSelectElement>("#language"); if (language) language.value = settings.language;
  const activation = document.querySelector<HTMLSelectElement>("#activation"); if (activation) activation.value = settings.activationMode;
  document.querySelectorAll<HTMLElement>("[data-model]").forEach(card => {
    const select = () => selectModel(card.dataset.model!);
    card.addEventListener("click", event => {
      if ((event.target as HTMLElement).closest("[data-download], [data-delete]")) return;
      select();
    });
    card.addEventListener("keydown", event => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        select();
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
    noticeText = "Hotkey recording cancelled.";
    try { await invoke("resume_hotkey_listener"); } catch { /* ignore */ }
    render();
    return;
  }
  try { await invoke("pause_hotkey_listener"); } catch { /* ignore */ }
  recordingHotkey = true;
  noticeText = "Press a key or combo. Escape cancels. Lone Right Ctrl/Alt/Shift are valid.";
  render();
}

function finishHotkeyCapture(spec: string, label: string) {
  settings.hotkey = spec;
  recordingHotkey = false;
  noticeText = `Hotkey set to ${label}. Save to apply.`;
  void invoke("resume_hotkey_listener").catch(() => undefined);
  render();
}

function onGlobalKeyDown(event: KeyboardEvent) {
  if (!recordingHotkey) return;
  event.preventDefault();
  event.stopPropagation();
  if (event.key === "Escape") {
    recordingHotkey = false;
    noticeText = "Hotkey recording cancelled.";
    void invoke("resume_hotkey_listener").catch(() => undefined);
    render();
    return;
  }
  if (event.key === "Meta" || event.code.startsWith("Meta") || event.code === "OSLeft" || event.code === "OSRight") {
    noticeText = "Win/Super is reserved on Windows. Pick another key.";
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
  try { runtime = await invoke<RuntimeStatus>("get_runtime_status"); } catch { /* ignore */ }
}
async function refreshLogs() {
  try { logLines = await invoke<string[]>("get_log_lines"); } catch { logLines = []; }
}
async function clearHistory() {
  try { await invoke("clear_history"); history = []; noticeText = "History cleared."; } catch (error) { noticeText = String(error); }
  render();
}
async function dismissWelcome() {
  try {
    await invoke("dismiss_welcome");
    settings.welcomeDismissed = true;
  } catch (error) {
    noticeText = String(error);
  }
  render();
}
async function downloadModel(id: string) {
  try {
    statuses[id] = { ...(statuses[id] ?? { installed: false, downloadable: true }), downloading: true, progress: 0, message: "Connecting…" };
    noticeText = `Downloading ${models.find(model => model.id === id)?.name ?? "model"}…`; render();
    const timer = window.setInterval(() => refreshStatuses().then(render).catch(() => undefined), 500);
    await invoke("download_model", { modelId: id });
    window.clearInterval(timer); await refreshStatuses(); noticeText = "Model installed locally.";
  } catch (error) {
    await refreshStatuses().catch(() => undefined);
    const status = statuses[id];
    noticeText = status?.message ? String(status.message) : String(error);
  }
  render();
}
async function deleteModel(id: string) {
  try { await invoke("delete_model", { modelId: id }); await refreshStatuses(); noticeText = "Model removed."; } catch (error) { noticeText = String(error); }
  render();
}
async function selectModel(id: string) {
  settings.selectedModel = id;
  try { await invoke("save_settings", { settings }); noticeText = `${selected()?.name ?? "Model"} selected.`; render(); } catch (error) { noticeText = String(error); render(); }
}
async function toggleRecording() {
  try {
    if (!recording) {
      if (!modelInstalled()) {
        noticeText = emptySpeechMessage();
        render();
        return;
      }
      await invoke("start_recording");
      recording = true;
      noticeText = "Listening locally…";
      await refreshRuntime();
      render();
      return;
    }
    recording = false; noticeText = "Transcribing on this PC…"; render();
    const text = await invoke<string>("stop_and_transcribe");
    if (!text) { noticeText = emptySpeechMessage(); render(); return; }
    await invoke("inject_text", { text }); await refreshHistory(); noticeText = `Inserted: ${text}`;
  } catch (error) { recording = false; noticeText = String(error); }
  await refreshRuntime();
  render();
}
async function toggleMicTest() {
  try {
    if (!micTesting) {
      if (recording || testListening) return;
      await invoke("start_mic_test");
      micTesting = true;
      noticeText = "Mic Test: speak to watch the level. This does not recognize speech.";
      if (micMeterTimer) window.clearInterval(micMeterTimer);
      micMeterTimer = window.setInterval(async () => {
        try {
          micLevel = await invoke<number>("get_mic_level");
          const bar = document.querySelector<HTMLElement>(".level-meter span");
          if (bar) bar.style.width = `${Math.min(100, Math.round(micLevel * 140))}%`;
        } catch { /* ignore */ }
      }, 80);
      render();
      return;
    }
    await invoke("stop_mic_test");
    micTesting = false;
    micLevel = 0;
    if (micMeterTimer) { window.clearInterval(micMeterTimer); micMeterTimer = null; }
    noticeText = "Mic Test stopped.";
  } catch (error) {
    micTesting = false;
    micLevel = 0;
    if (micMeterTimer) { window.clearInterval(micMeterTimer); micMeterTimer = null; }
    noticeText = String(error);
  }
  render();
}
async function testDictation() {
  if (runtime.paused || micTesting) return;
  try {
    if (!testListening) {
      if (!modelInstalled()) {
        noticeText = emptySpeechMessage();
        render();
        return;
      }
      testingDictation = true;
      await invoke("start_recording", { noInject: true });
      recording = true;
      testListening = true;
      testingDictation = false;
      noticeText = "Test Dictation listening… click Stop test. Result stays here (no inject).";
      await refreshRuntime();
      render();
      return;
    }
    testingDictation = true;
    recording = false;
    testListening = false;
    noticeText = "Transcribing test take…";
    render();
    const text = await invoke<string>("stop_and_transcribe");
    noticeText = text ? `Test result: ${text}` : emptySpeechMessage();
    await refreshHistory();
  } catch (error) {
    recording = false;
    testListening = false;
    noticeText = String(error);
  }
  testingDictation = false;
  await refreshRuntime();
  render();
}
async function save() {
  const preset = document.querySelector<HTMLSelectElement>("#hotkey-preset");
  if (preset) settings.hotkey = preset.value;
  const language = document.querySelector<HTMLSelectElement>("#language");
  if (language) settings.language = language.value;
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
  }
  const autoCap = document.querySelector<HTMLInputElement>("#auto-cap");
  if (autoCap) settings.autoCapitalize = autoCap.checked;
  const trailing = document.querySelector<HTMLInputElement>("#trailing-space");
  if (trailing) settings.appendTrailingSpace = trailing.checked;
  const launch = document.querySelector<HTMLInputElement>("#launch-login");
  if (launch) settings.launchAtLogin = launch.checked;
  const inputDevice = document.querySelector<HTMLSelectElement>("#input-device");
  if (inputDevice) settings.inputDevice = inputDevice.value;
  const autoPause = document.querySelector<HTMLInputElement>("#auto-pause");
  if (autoPause) settings.autoPauseEnabled = autoPause.checked;
  const autoPauseApps = document.querySelector<HTMLTextAreaElement>("#auto-pause-apps");
  if (autoPauseApps) settings.autoPauseApps = autoPauseApps.value;
  const idleUnload = document.querySelector<HTMLInputElement>("#idle-unload");
  if (idleUnload) settings.idleUnloadEnabled = idleUnload.checked;
  const idleSeconds = document.querySelector<HTMLInputElement>("#idle-unload-seconds");
  if (idleSeconds) settings.idleUnloadSeconds = Number(idleSeconds.value) || 300;
  try {
    await invoke("save_settings", { settings });
    settings = await invoke<Settings>("get_settings");
    await refreshRuntime();
    noticeText = "Settings saved. Hotkey is live.";
  } catch (error) { noticeText = String(error); }
  render();
}

window.addEventListener("keydown", onGlobalKeyDown, true);
window.addEventListener("keyup", onGlobalKeyUp, true);

listen<boolean>("recording-changed", event => {
  recording = event.payload;
  if (!event.payload) testListening = false;
  refreshRuntime().then(render).catch(() => render());
}).catch(() => undefined);
listen<string>("dictation-finished", async event => {
  recording = false;
  await refreshHistory().catch(() => undefined);
  await refreshRuntime().catch(() => undefined);
  noticeText = event.payload ? `Inserted: ${event.payload}` : emptySpeechMessage();
  render();
}).catch(() => undefined);
listen<string>("test-dictation-finished", async event => {
  recording = false;
  testListening = false;
  await refreshHistory().catch(() => undefined);
  await refreshRuntime().catch(() => undefined);
  noticeText = event.payload ? `Test result: ${event.payload}` : emptySpeechMessage();
  render();
}).catch(() => undefined);
listen<string>("dictation-error", event => {
  recording = false;
  noticeText = event.payload;
  refreshRuntime().then(render).catch(() => render());
}).catch(() => undefined);
listen<string>("runtime-status", event => {
  runtime = { ...runtime, status: event.payload, paused: event.payload === "Paused" };
  render();
}).catch(() => undefined);
listen<Settings>("settings-changed", event => {
  settings = { ...settings, ...event.payload };
  render();
}).catch(() => undefined);
listen("show-about", () => {
  showAbout = true;
  render();
}).catch(() => undefined);
listen<string>("navigate", event => {
  if (event.payload === "settings" || event.payload === "models" || event.payload === "history" || event.payload === "dictation") {
    view = event.payload;
    render();
  }
}).catch(() => undefined);
listen<string>("log-line", event => {
  logLines = [...logLines.slice(-499), event.payload];
  if (isLogsWindow) render();
}).catch(() => undefined);

if (isLogsWindow) {
  refreshLogs().then(render).catch(error => { app.textContent = `Could not open logs: ${error}`; });
} else {
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
  ]).then(([catalog, saved, installs, entries, hotkeyPresets, gpuStatus, inputDevices, modelRec, runtimeStatus]) => {
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
    render();
  }).catch(error => { app.textContent = `Could not start VocaWin: ${error}`; });
}
