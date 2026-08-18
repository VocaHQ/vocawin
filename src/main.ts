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
  appendTrailingSpace: boolean;
  autoCapitalize: boolean;
  selectedModel: string;
  inputDevice: string;
  autoPauseEnabled: boolean;
  autoPauseApps: string;
  idleUnloadEnabled: boolean;
  idleUnloadSeconds: number;
};
type View = "dictation" | "models" | "history" | "settings";
type HistoryEntry = { id: number; text: string; modelId: string; createdAtMs: number };
type ModelStatus = { installed: boolean; downloadable: boolean; downloading: boolean; progress: number; message?: string };
type HotkeyPreset = { id: string; label: string };
type GpuStatus = { available: boolean; name: string; backend: string; detail: string };
type InputDevice = { name: string; isDefault: boolean };
type ModelRecommendation = { modelId: string; modelName: string; reason: string; vramMb: number; gpu: GpuStatus };
type RuntimeStatus = { status: string; recording: boolean; paused: boolean; hotkey: string; inputDevice: string; gpuName: string; gpuBackend: string };

type SettingsItem = {
  group: "Dictation" | "Audio" | "Application";
  title: string;
  subtitle: string;
  keywords: string;
  html: string;
};

const app = document.querySelector<HTMLDivElement>("#app")!;
let models: Model[] = [];
let statuses: Record<string, ModelStatus> = {};
let history: HistoryEntry[] = [];
let settings: Settings;
let presets: HotkeyPreset[] = [];
let gpu: GpuStatus = { available: false, name: "Checking…", backend: "CPU", detail: "" };
let devices: InputDevice[] = [];
let recommendation: ModelRecommendation | null = null;
let runtime: RuntimeStatus = { status: "Ready", recording: false, paused: false, hotkey: "", inputDevice: "Default microphone", gpuName: "", gpuBackend: "" };
let recording = false;
let recordingHotkey = false;
let testingDictation = false;
let testListening = false;
let settingsQuery = "";
let view: View = "dictation";
let noticeText = "";
const escape = (value: string) => value.replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]!));
const selected = () => models.find(model => model.id === settings.selectedModel);
const nav = (id: View, label: string, icon: string) => `<button class="nav ${view === id ? "active" : ""}" data-view="${id}"><span class="nav-icon">${icon}</span>${label}</button>`;

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
    const state = status?.downloading
      ? `Downloading ${status.progress}%`
      : status?.installed
        ? "Installed"
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
        <li class="install-state">${escape(state)}</li>
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
  return [
    {
      group: "Dictation",
      title: "Activation hotkey",
      subtitle: "Pick a preset or press Record, then the keys. Escape cancels. Windows consumes the combo so it does not type into the front app.",
      keywords: "hotkey shortcut keyboard record preset",
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
      subtitle: "Auto-detect is best for multilingual speech.",
      keywords: "language locale english",
      html: `<select id="language"><option>Auto-detect</option><option>English</option><option>Spanish</option><option>French</option><option>German</option><option>Japanese</option><option>Chinese</option></select>`,
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
      title: "Silence auto-stop",
      subtitle: "Seconds of quiet before toggle mode ends a take.",
      keywords: "vad silence timeout",
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
      title: "Sound feedback",
      subtitle: "Play a small cue when dictation starts and stops.",
      keywords: "sound beep audio cue",
      html: `<label class="switch"><input id="sound" type="checkbox" ${settings.soundEffects ? "checked" : ""}/><span></span></label>`,
    },
    {
      group: "Application",
      title: "Launch at login",
      subtitle: "Start VocaWin with Windows for this user.",
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
      keywords: "gpu vulkan directml cuda hardware",
      html: `<div class="gpu-readout"><strong>${escape(gpu.name)}</strong><span>${escape(gpu.backend)}</span></div>`,
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
    <button type="button" class="quiet-button" id="test-dictation" ${runtime.paused ? "disabled" : ""}>${testListening ? "Stop test" : testingDictation ? "Testing…" : "Test Dictation"}</button>
  </footer>
  ${recordingHotkey ? `<p class="notice recording-hint">Press a key combo, or Escape to cancel.</p>` : ""}`;
}

function render() {
  const pages: Record<View, () => string> = { dictation: dictationPage, models: modelsPage, history: historyPage, settings: settingsPage };
  app.innerHTML = `<aside><div class="brand"><span class="mark"><img src="/src/assets/voca-logo.svg" alt="Voca"/></span><span>VocaWin</span></div><p class="brand-subtitle">Voice dictation, kept private.</p><nav>${nav("dictation", "Dictation", "◉")}${nav("models", "Models", "◇")}${nav("history", "History", "≡")}${nav("settings", "Settings", "⚙")}</nav><div class="privacy"><i>✓</i><div><b>Private by default</b><small>Your audio stays here</small></div></div></aside><main>${pages[view]()}<p class="notice" role="status">${escape(noticeText)}</p></main>`;
  document.querySelectorAll<HTMLButtonElement>("[data-view]").forEach(button => button.addEventListener("click", () => { view = button.dataset.view as View; render(); }));
  document.querySelectorAll<HTMLButtonElement>("[data-go]").forEach(button => button.addEventListener("click", () => { view = button.dataset.go as View; render(); }));
  document.querySelector("#record")?.addEventListener("click", toggleRecording);
  document.querySelector("#save")?.addEventListener("click", save);
  document.querySelector("#clear-history")?.addEventListener("click", clearHistory);
  document.querySelector("#test-dictation")?.addEventListener("click", testDictation);
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
  document.querySelector("#record-hotkey")?.addEventListener("click", () => {
    recordingHotkey = !recordingHotkey;
    noticeText = recordingHotkey ? "Press a key combo…" : "";
    render();
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
    Pause: "Pause",
    ControlRight: "ControlRight",
    ControlLeft: "ControlLeft",
    AltRight: "AltRight",
    AltLeft: "AltLeft",
  };
  if (map[code]) return map[code];
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (key.length === 1) return key.toUpperCase();
  return null;
}

function onGlobalKeyDown(event: KeyboardEvent) {
  if (!recordingHotkey) return;
  event.preventDefault();
  event.stopPropagation();
  if (event.key === "Escape") {
    recordingHotkey = false;
    noticeText = "Hotkey recording cancelled.";
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
  if (keyPart === "ControlRight" || keyPart === "AltRight" || keyPart === "ControlLeft" || keyPart === "AltLeft") {
    settings.hotkey = keyPart;
  } else {
    parts.push(keyPart);
    settings.hotkey = parts.join("+");
  }
  recordingHotkey = false;
  noticeText = `Hotkey set to ${settings.hotkey}. Save to apply.`;
  render();
}

function onGlobalKeyUp(event: KeyboardEvent) {
  if (!recordingHotkey) return;
  if (event.code === "ControlRight" || event.code === "AltRight") {
    if (!event.shiftKey && !event.metaKey) {
      if (event.code === "ControlRight" && !event.altKey) {
        event.preventDefault();
        settings.hotkey = "ControlRight";
        recordingHotkey = false;
        noticeText = "Hotkey set to Right Ctrl. Save to apply.";
        render();
      } else if (event.code === "AltRight" && !event.ctrlKey) {
        event.preventDefault();
        settings.hotkey = "AltRight";
        recordingHotkey = false;
        noticeText = "Hotkey set to Right Alt. Save to apply.";
        render();
      }
    }
  }
}

async function refreshStatuses() { statuses = await invoke<Record<string, ModelStatus>>("get_model_statuses"); }
async function refreshHistory() { history = await invoke<HistoryEntry[]>("get_history"); }
async function refreshRuntime() {
  try { runtime = await invoke<RuntimeStatus>("get_runtime_status"); } catch { /* ignore */ }
}
async function clearHistory() {
  try { await invoke("clear_history"); history = []; noticeText = "History cleared."; } catch (error) { noticeText = String(error); }
  render();
}
async function downloadModel(id: string) {
  try {
    statuses[id] = { ...(statuses[id] ?? { installed: false, downloadable: true }), downloading: true, progress: 0 };
    noticeText = `Downloading ${models.find(model => model.id === id)?.name ?? "model"}…`; render();
    const timer = window.setInterval(() => refreshStatuses().then(render).catch(() => undefined), 500);
    await invoke("download_model", { modelId: id });
    window.clearInterval(timer); await refreshStatuses(); noticeText = "Model installed locally.";
  } catch (error) { await refreshStatuses().catch(() => undefined); noticeText = String(error); }
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
    if (!recording) { await invoke("start_recording"); recording = true; noticeText = "Listening locally…"; await refreshRuntime(); render(); return; }
    recording = false; noticeText = "Transcribing on this PC…"; render();
    const text = await invoke<string>("stop_and_transcribe");
    if (!text) { noticeText = "No speech was recognized."; render(); return; }
    await invoke("inject_text", { text }); await refreshHistory(); noticeText = `Inserted: ${text}`;
  } catch (error) { recording = false; noticeText = String(error); }
  await refreshRuntime();
  render();
}
async function testDictation() {
  if (runtime.paused) return;
  try {
    if (!testListening) {
      testingDictation = true;
      await invoke("start_recording");
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
    noticeText = text ? `Test result: ${text}` : "Test Dictation heard no speech.";
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
  const sound = document.querySelector<HTMLInputElement>("#sound");
  if (sound) settings.soundEffects = sound.checked;
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
  noticeText = event.payload ? `Inserted: ${event.payload}` : "No speech was recognized.";
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
    maxRecordingSeconds: saved.maxRecordingSeconds ?? 60,
    appendTrailingSpace: saved.appendTrailingSpace ?? true,
    autoCapitalize: saved.autoCapitalize ?? true,
    inputDevice: saved.inputDevice ?? "",
    autoPauseEnabled: saved.autoPauseEnabled ?? false,
    autoPauseApps: saved.autoPauseApps ?? "",
    idleUnloadEnabled: saved.idleUnloadEnabled ?? false,
    idleUnloadSeconds: saved.idleUnloadSeconds ?? 300,
  };
  statuses = installs;
  history = entries;
  presets = hotkeyPresets;
  gpu = gpuStatus;
  devices = inputDevices;
  recommendation = modelRec;
  runtime = runtimeStatus;
  render();
}).catch(error => { app.textContent = `Could not start VocaWin: ${error}`; });
