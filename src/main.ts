import { invoke } from "@tauri-apps/api/core";
import "./style.css";

type Model = { id: string; name: string; engine: string; size: string; languages: string; acceleration: string; description: string };
type Settings = { hotkey: string; activationMode: string; language: string; silenceSeconds: number; launchAtLogin: boolean; soundEffects: boolean; selectedModel: string };
type View = "dictation" | "models" | "history" | "settings";
type HistoryEntry = { id: number; text: string; modelId: string; createdAtMs: number };
type ModelStatus = { installed: boolean; downloadable: boolean; downloading: boolean; progress: number; message?: string };

const app = document.querySelector<HTMLDivElement>("#app")!;
let models: Model[] = [];
let statuses: Record<string, ModelStatus> = {};
let history: HistoryEntry[] = [];
let settings: Settings;
let recording = false;
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
    return `<article class="model-card ${isSelected ? "selected" : ""}" data-model="${model.id}" role="button" tabindex="0" aria-pressed="${isSelected}">
      <span class="check" aria-hidden="true">${isSelected ? "✓" : ""}</span>
      <b>${escape(model.name)}</b>
      <span class="engine">${escape(model.engine)}</span>
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
function dictationPage() {
  const model = selected();
  return `<header><div><p class="overline">VOICE DICTATION</p><h1>Speak naturally.<br><em>Keep it private.</em></h1><p class="lede">VocaWin turns your voice into text on your own computer — never in the cloud.</p></div><span class="state"><i></i>${recording ? "Listening" : "Ready"}</span></header>
  <section class="record-panel"><div class="mic ${recording ? "recording" : ""}">${recording ? "❚❚" : "⌁"}</div><h2>${recording ? "Listening…" : "Ready to dictate"}</h2><p>${recording ? "Speak now, then stop when you are finished." : `Use ${escape(settings.hotkey)} or start below.`}</p><button class="primary" id="record">${recording ? "Stop & transcribe" : "Start dictation"}</button><small>Everything is processed locally on this device.</small></section>
  <section class="overview"><div class="info-card"><p class="card-label">ACTIVE MODEL</p><strong>${escape(model?.name ?? "Choose a model")}</strong><span>${escape(model?.engine ?? "")} · ${escape(model?.languages ?? "")}</span><button class="text-button" data-go="models">Change model →</button></div><div class="info-card"><p class="card-label">ACTIVATION</p><strong>${escape(settings.hotkey)}</strong><span>Press and hold to dictate</span><button class="text-button" data-go="settings">Edit shortcut →</button></div></section>`;
}
function modelsPage() {
  return `<header><div><p class="overline">ON-DEVICE MODELS</p><h1>Choose your <em>engine.</em></h1><p class="lede">Models stay on your PC. Pick the trade-off between speed, accuracy, and language coverage. Click a card to select it.</p></div></header>
  <div class="model-grid">${modelCards()}</div>`;
}
function historyPage() {
  const entries = history.length ? history.map(entry => `<article class="history-entry"><p>${escape(entry.text)}</p><footer>${escape(models.find(model => model.id === entry.modelId)?.name ?? entry.modelId)} · ${new Date(entry.createdAtMs).toLocaleString()}</footer></article>`).join("") : `<div class="empty-history">Your local transcription history will appear here.</div>`;
  return `<header><div><p class="overline">LOCAL HISTORY</p><h1>Your recent <em>dictation.</em></h1><p class="lede">History is stored only on this computer and can be cleared at any time.</p></div>${history.length ? `<button class="quiet-button" id="clear-history">Clear history</button>` : ""}</header><section class="history-list">${entries}</section>`;
}
function settingsPage() {
  return `<header><div><p class="overline">PREFERENCES</p><h1>Make it <em>yours.</em></h1><p class="lede">VocaWin only stores these choices locally on this PC.</p></div></header>
  <section class="settings-card"><div class="setting-row"><div><strong>Activation hotkey</strong><p>Hold this anywhere in Windows to dictate (push to talk).</p></div><input id="hotkey" value="${escape(settings.hotkey)}" /></div>
  <div class="setting-row"><div><strong>Dictation language</strong><p>Auto-detect is best for multilingual speech.</p></div><select id="language"><option>Auto-detect</option><option>English</option><option>Spanish</option><option>French</option><option>German</option><option>Japanese</option><option>Chinese</option></select></div>
  <div class="setting-row"><div><strong>Sound feedback</strong><p>Play a small cue when dictation starts and stops.</p></div><label class="switch"><input id="sound" type="checkbox" ${settings.soundEffects ? "checked" : ""}/><span></span></label></div>
  <div class="settings-footer"><button class="primary" id="save">Save changes</button></div></section>`;
}
function render() {
  const pages: Record<View, () => string> = { dictation: dictationPage, models: modelsPage, history: historyPage, settings: settingsPage };
  app.innerHTML = `<aside><div class="brand"><span class="mark"><img src="/src/assets/voca-logo.svg" alt="Voca"/></span><span>VocaWin</span></div><p class="brand-subtitle">Voice dictation, kept private.</p><nav>${nav("dictation", "Dictation", "◉")}${nav("models", "Models", "◇")}${nav("history", "History", "≡")}${nav("settings", "Settings", "⚙")}</nav><div class="privacy"><i>✓</i><div><b>Private by default</b><small>Your audio stays here</small></div></div></aside><main>${pages[view]()}<p class="notice" role="status">${escape(noticeText)}</p></main>`;
  document.querySelectorAll<HTMLButtonElement>("[data-view]").forEach(button => button.addEventListener("click", () => { view = button.dataset.view as View; render(); }));
  document.querySelectorAll<HTMLButtonElement>("[data-go]").forEach(button => button.addEventListener("click", () => { view = button.dataset.go as View; render(); }));
  document.querySelector("#record")?.addEventListener("click", toggleRecording);
  document.querySelector("#save")?.addEventListener("click", save);
  document.querySelector("#clear-history")?.addEventListener("click", clearHistory);
  const language = document.querySelector<HTMLSelectElement>("#language"); if (language) language.value = settings.language;
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
async function refreshStatuses() { statuses = await invoke<Record<string, ModelStatus>>("get_model_statuses"); }
async function refreshHistory() { history = await invoke<HistoryEntry[]>("get_history"); }
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
    if (!recording) { await invoke("start_recording"); recording = true; noticeText = "Listening locally…"; render(); return; }
    recording = false; noticeText = "Transcribing on this PC…"; render();
    const text = await invoke<string>("stop_and_transcribe");
    if (!text) { noticeText = "No speech was recognized."; render(); return; }
    await invoke("inject_text", { text }); await refreshHistory(); noticeText = `Inserted: ${text}`;
  } catch (error) { recording = false; noticeText = String(error); }
  render();
}
async function save() {
  settings.hotkey = document.querySelector<HTMLInputElement>("#hotkey")!.value.trim() || "Ctrl+Alt+Space";
  settings.language = document.querySelector<HTMLSelectElement>("#language")!.value;
  settings.activationMode = "pushToTalk";
  settings.launchAtLogin = false;
  settings.soundEffects = document.querySelector<HTMLInputElement>("#sound")!.checked;
  try { await invoke("save_settings", { settings }); noticeText = "Settings saved locally."; } catch (error) { noticeText = String(error); }
  render();
}
Promise.all([invoke<Model[]>("get_models"), invoke<Settings>("get_settings"), invoke<Record<string, ModelStatus>>("get_model_statuses"), invoke<HistoryEntry[]>("get_history")]).then(([catalog, saved, installs, entries]) => { models = catalog; settings = saved; statuses = installs; history = entries; render(); }).catch(error => { app.textContent = `Could not start VocaWin: ${error}`; });
