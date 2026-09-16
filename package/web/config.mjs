import {
  RF73_MAX_FILE_BYTES,
  createRf73File,
  parseRf73File,
} from "./rf73-format.mjs";

const PROTOCOL = "rackforge.plugin.web@1";
const PLUGIN_ID = "org.rackforge.rhodes";
const REQUEST_TIMEOUT_MS = 10_000;
const source = document.querySelector("#export-source");
const exportButton = document.querySelector("#export-button");
const importButton = document.querySelector("#import-button");
const fileInput = document.querySelector("#import-file");
const activity = document.querySelector("#activity");
const error = document.querySelector("#error");
const statusCard = document.querySelector("#status-card");
const connection = document.querySelector("#connection");
const recoverButton = document.querySelector("#recover-button");

let context = null;
let requestSerial = 0;
let busy = false;
let showedPendingDraft = false;
const requests = new Map();
const contextWaiters = new Set();

function setStatus(message, failure = "") {
  activity.textContent = message;
  error.textContent = failure;
  statusCard.hidden = !message && !failure;
}

function setBusy(value) {
  busy = value;
  render();
}

function call(method, params = {}) {
  const requestId = `rf73-config-${++requestSerial}`;
  return new Promise((resolve, reject) => {
    const timer = window.setTimeout(() => {
      requests.delete(requestId);
      reject(new Error("RackForge did not answer in time."));
    }, REQUEST_TIMEOUT_MS);
    requests.set(requestId, { resolve, reject, timer });
    parent.postMessage({ protocol: PROTOCOL, kind: "request", request_id: requestId, method, params }, location.origin);
  });
}

function waitForContext(predicate) {
  if (context && predicate(context)) return Promise.resolve(context);
  return new Promise((resolve, reject) => {
    const waiter = { predicate, resolve, reject, timer: 0 };
    waiter.timer = window.setTimeout(() => {
      contextWaiters.delete(waiter);
      reject(new Error("RackForge did not finish the program operation in time."));
    }, REQUEST_TIMEOUT_MS);
    contextWaiters.add(waiter);
  });
}

function publishContext(next) {
  context = next;
  for (const waiter of [...contextWaiters]) {
    if (waiter.predicate(next)) {
      window.clearTimeout(waiter.timer);
      contextWaiters.delete(waiter);
      waiter.resolve(next);
    }
  }
  render();
}

function sounds() {
  return Array.isArray(context?.instance?.sounds) ? context.instance.sounds : [];
}

function render() {
  const connected = context?.instance?.plugin_id === PLUGIN_ID;
  connection.textContent = connected ? "Connected to RackForge" : "Connecting to RackForge…";
  connection.dataset.ready = connected ? "true" : "false";
  connection.hidden = connected;
  if (connected && activity.textContent === "Waiting for RackForge.") setStatus("");
  if (connected) {
    const signature = sounds().map((sound) => `${sound.id}:${sound.name}`).join("|");
    if (source.dataset.catalog !== signature) {
      const previous = source.value;
      source.replaceChildren();
      for (const sound of sounds()) source.add(new Option(sound.name, sound.id));
      source.value = sounds().some((sound) => sound.id === previous)
        ? previous
        : context.instance.selected_sound_id || sounds()[0]?.id || "";
      source.dataset.catalog = signature;
    }
  }
  const foreignDraft = context?.program_draft && !busy;
  const ready = connected && !busy && !foreignDraft;
  source.disabled = !ready;
  exportButton.disabled = !ready || !source.value;
  importButton.disabled = !ready;
  recoverButton.hidden = !foreignDraft;
  recoverButton.disabled = busy;
  if (foreignDraft) {
    showedPendingDraft = true;
    setStatus("A previous program operation is still open. Discard it to continue safely.");
  } else if (!context?.program_draft && showedPendingDraft) {
    showedPendingDraft = false;
    setStatus("Pending edit cleared. Program files are ready again.");
  }
}

function slug(value) {
  const result = value
    .normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "")
    .slice(0, 56);
  return result || "rf73-program";
}

function safeFileName(value) {
  return `${slug(value)}.rf73`;
}

function download(text, fileName) {
  const url = URL.createObjectURL(new Blob([text], { type: "application/vnd.rackforge.rf73+json" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = fileName;
  link.click();
  window.setTimeout(() => URL.revokeObjectURL(url), 0);
}

async function cancelDraft(draftId) {
  try {
    await call("plugin.cancel_program", { draft_id: draftId });
    await waitForContext((next) => !next.program_draft);
  } catch {
    // The original error is more useful; RackForge expires abandoned auditions.
  }
}

async function recoverDraft() {
  const draftId = context?.program_draft?.draft_id;
  if (draftId === undefined) return;
  setBusy(true);
  setStatus("Discarding the pending program edit…");
  try {
    await call("plugin.cancel_program", { draft_id: draftId });
    await waitForContext((next) => !next.program_draft);
    setStatus("Pending edit discarded. Program files are ready again.");
  } catch (cause) {
    setStatus("The pending edit could not be discarded.", cause instanceof Error ? cause.message : String(cause));
  } finally {
    setBusy(false);
  }
}

async function exportProgram() {
  if (!source.value) return;
  setBusy(true);
  setStatus("Preparing the program file…");
  let draftId = null;
  try {
    await call("plugin.begin_program_edit", { program_id: source.value });
    const next = await waitForContext((candidate) => candidate.program_draft);
    const draft = next.program_draft;
    draftId = draft.draft_id;
    const document = JSON.parse(draft.document_json);
    const text = await createRf73File(document);
    download(text, safeFileName(document.name));
    await cancelDraft(draftId);
    draftId = null;
    setStatus(`${document.name} downloaded with checksum.`);
  } catch (cause) {
    if (draftId !== null) await cancelDraft(draftId);
    setStatus("The program was not downloaded.", cause instanceof Error ? cause.message : String(cause));
  } finally {
    setBusy(false);
  }
}

async function importProgram(file) {
  setBusy(true);
  setStatus("Checking the .rf73 file…");
  let draftId = null;
  try {
    if (file.size > RF73_MAX_FILE_BYTES) throw new Error("The .rf73 file is larger than 32 KiB.");
    const imported = await parseRf73File(await file.text());
    await call("plugin.begin_program_edit", { program_id: null });
    const opened = await waitForContext((candidate) => candidate.program_draft);
    draftId = opened.program_draft.draft_id;
    const allocated = JSON.parse(opened.program_draft.document_json);
    imported.id = allocated.id;
    const id = imported.id;
    await call("plugin.replace_program_draft", { draft_id: draftId, document: imported });
    await waitForContext((candidate) => {
      if (candidate.program_draft?.draft_id !== draftId) return false;
      try { return JSON.parse(candidate.program_draft.document_json).id === id; } catch { return false; }
    });
    await call("plugin.save_program", { draft_id: draftId });
    await waitForContext(
      (candidate) => !candidate.program_draft && candidate.instance.sounds.some((sound) => sound.id === `custom.${id}`),
    );
    draftId = null;
    setStatus(`${imported.name} installed as a new user program.`);
  } catch (cause) {
    if (draftId !== null) await cancelDraft(draftId);
    setStatus("The program was not imported.", cause instanceof Error ? cause.message : String(cause));
  } finally {
    fileInput.value = "";
    setBusy(false);
  }
}

exportButton.addEventListener("click", () => void exportProgram());
importButton.addEventListener("click", () => fileInput.click());
recoverButton.addEventListener("click", () => void recoverDraft());
fileInput.addEventListener("change", () => {
  const file = fileInput.files?.[0];
  if (file) void importProgram(file);
});

window.addEventListener("message", (event) => {
  if (event.source !== parent || event.origin !== location.origin || event.data?.protocol !== PROTOCOL) return;
  if (event.data.kind === "context" && event.data.surface === "config" && event.data.instance?.plugin_id === PLUGIN_ID) {
    document.documentElement.dataset.lighting = ["day", "stage"].includes(event.data.host?.lighting)
      ? event.data.host.lighting
      : "stage";
    publishContext(event.data);
  } else if (event.data.kind === "response" && requests.has(event.data.request_id)) {
    const pending = requests.get(event.data.request_id);
    requests.delete(event.data.request_id);
    window.clearTimeout(pending.timer);
    if (event.data.ok) pending.resolve(event.data.result);
    else pending.reject(new Error(event.data.error || "RackForge rejected the operation."));
  }
});

parent.postMessage({ protocol: PROTOCOL, kind: "ready" }, location.origin);
render();
