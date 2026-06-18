const DEFAULT_BASE_URL = "http://127.0.0.1:3030";
const SESSION_ID = `pwa-${Date.now().toString(36)}`;
const STORAGE_SESSION_TOKEN = "indwell.console.sessionToken";
const STORAGE_PAIRED_DEVICE = "indwell.console.pairedDevice";
const STORAGE_PUBLIC_KEY_JWK = "indwell.console.publicKeyJwk";
const STORAGE_PRIVATE_KEY_JWK = "indwell.console.privateKeyJwk";

const els = {
  baseUrl: document.querySelector("#baseUrl"),
  statusDot: document.querySelector("#statusDot"),
  statusText: document.querySelector("#statusText"),
  connectionForm: document.querySelector("#connectionForm"),
  providerForm: document.querySelector("#providerForm"),
  loadProviderButton: document.querySelector("#loadProviderButton"),
  providerKind: document.querySelector("#providerKind"),
  providerModel: document.querySelector("#providerModel"),
  providerBaseUrl: document.querySelector("#providerBaseUrl"),
  providerApiKeyRef: document.querySelector("#providerApiKeyRef"),
  providerApiKeySecret: document.querySelector("#providerApiKeySecret"),
  providerMaxOutput: document.querySelector("#providerMaxOutput"),
  providerVisionKind: document.querySelector("#providerVisionKind"),
  providerVisionModel: document.querySelector("#providerVisionModel"),
  providerVisionBaseUrl: document.querySelector("#providerVisionBaseUrl"),
  providerVisionApiKeyRef: document.querySelector("#providerVisionApiKeyRef"),
  providerAsrKind: document.querySelector("#providerAsrKind"),
  providerAsrModel: document.querySelector("#providerAsrModel"),
  providerAsrBaseUrl: document.querySelector("#providerAsrBaseUrl"),
  providerAsrApiKeyRef: document.querySelector("#providerAsrApiKeyRef"),
  providerTtsKind: document.querySelector("#providerTtsKind"),
  providerTtsModel: document.querySelector("#providerTtsModel"),
  providerTtsBaseUrl: document.querySelector("#providerTtsBaseUrl"),
  providerTtsApiKeyRef: document.querySelector("#providerTtsApiKeyRef"),
  providerEmbeddingKind: document.querySelector("#providerEmbeddingKind"),
  providerEmbeddingModel: document.querySelector("#providerEmbeddingModel"),
  providerEmbeddingBaseUrl: document.querySelector("#providerEmbeddingBaseUrl"),
  providerEmbeddingApiKeyRef: document.querySelector("#providerEmbeddingApiKeyRef"),
  saveSecretButton: document.querySelector("#saveSecretButton"),
  testLlmProviderButton: document.querySelector("#testLlmProviderButton"),
  testVisionProviderButton: document.querySelector("#testVisionProviderButton"),
  testAsrProviderButton: document.querySelector("#testAsrProviderButton"),
  testTtsProviderButton: document.querySelector("#testTtsProviderButton"),
  testEmbeddingProviderButton: document.querySelector("#testEmbeddingProviderButton"),
  providerResult: document.querySelector("#providerResult"),
  provisioningForm: document.querySelector("#provisioningForm"),
  loadProvisioningButton: document.querySelector("#loadProvisioningButton"),
  provisioningDeviceId: document.querySelector("#provisioningDeviceId"),
  provisioningSsid: document.querySelector("#provisioningSsid"),
  provisioningPasswordRef: document.querySelector("#provisioningPasswordRef"),
  provisioningOwnerLabel: document.querySelector("#provisioningOwnerLabel"),
  provisioningResult: document.querySelector("#provisioningResult"),
  channelForm: document.querySelector("#channelForm"),
  subjectHint: document.querySelector("#subjectHint"),
  channelText: document.querySelector("#channelText"),
  channelResult: document.querySelector("#channelResult"),
  voiceForm: document.querySelector("#voiceForm"),
  voiceText: document.querySelector("#voiceText"),
  voiceResult: document.querySelector("#voiceResult"),
  memoryForm: document.querySelector("#memoryForm"),
  memoryWing: document.querySelector("#memoryWing"),
  memoryRoom: document.querySelector("#memoryRoom"),
  memoryText: document.querySelector("#memoryText"),
  memoryLimit: document.querySelector("#memoryLimit"),
  addMemoryForm: document.querySelector("#addMemoryForm"),
  addMemoryKind: document.querySelector("#addMemoryKind"),
  addMemoryWing: document.querySelector("#addMemoryWing"),
  addMemoryRoom: document.querySelector("#addMemoryRoom"),
  addMemoryContent: document.querySelector("#addMemoryContent"),
  memoryResult: document.querySelector("#memoryResult"),
  memoryExportResult: document.querySelector("#memoryExportResult"),
  refreshMemoryButton: document.querySelector("#refreshMemoryButton"),
  reviewInboxButton: document.querySelector("#reviewInboxButton"),
  exportMemoryButton: document.querySelector("#exportMemoryButton"),
  metabolizeMemoryButton: document.querySelector("#metabolizeMemoryButton"),
  reflectionForm: document.querySelector("#reflectionForm"),
  reflectionLimit: document.querySelector("#reflectionLimit"),
  reflectionSkills: document.querySelector("#reflectionSkills"),
  reflectionResult: document.querySelector("#reflectionResult"),
  pairingChallengeButton: document.querySelector("#pairingChallengeButton"),
  issueSessionButton: document.querySelector("#issueSessionButton"),
  clearSessionButton: document.querySelector("#clearSessionButton"),
  listPairingButton: document.querySelector("#listPairingButton"),
  pairingForm: document.querySelector("#pairingForm"),
  pairingSessionId: document.querySelector("#pairingSessionId"),
  pairingCode: document.querySelector("#pairingCode"),
  pairingLabel: document.querySelector("#pairingLabel"),
  pairingResult: document.querySelector("#pairingResult"),
  sessionResult: document.querySelector("#sessionResult"),
  passphraseChallengeButton: document.querySelector("#passphraseChallengeButton"),
  passphraseForm: document.querySelector("#passphraseForm"),
  passphraseChallengeId: document.querySelector("#passphraseChallengeId"),
  passphraseSpoken: document.querySelector("#passphraseSpoken"),
  passphraseResult: document.querySelector("#passphraseResult"),
  loadToolsButton: document.querySelector("#loadToolsButton"),
  toolCheckButton: document.querySelector("#toolCheckButton"),
  toolStatusButton: document.querySelector("#toolStatusButton"),
  toolSensorButton: document.querySelector("#toolSensorButton"),
  toolCameraOwnerButton: document.querySelector("#toolCameraOwnerButton"),
  toolWriteMemoryButton: document.querySelector("#toolWriteMemoryButton"),
  toolBlockedCameraButton: document.querySelector("#toolBlockedCameraButton"),
  toolCatalogResult: document.querySelector("#toolCatalogResult"),
  toolResult: document.querySelector("#toolResult"),
  loadOtaButton: document.querySelector("#loadOtaButton"),
  checkOtaButton: document.querySelector("#checkOtaButton"),
  toolUpdateButton: document.querySelector("#toolUpdateButton"),
  otaResult: document.querySelector("#otaResult"),
  webhookForm: document.querySelector("#webhookForm"),
  webhookSource: document.querySelector("#webhookSource"),
  webhookText: document.querySelector("#webhookText"),
  webhookResult: document.querySelector("#webhookResult"),
  refreshRunsButton: document.querySelector("#refreshRunsButton"),
  runsResult: document.querySelector("#runsResult"),
  runDetailResult: document.querySelector("#runDetailResult"),
  apiLog: document.querySelector("#apiLog"),
  clearLogButton: document.querySelector("#clearLogButton"),
};

let lastMemoryQuery = null;
let lastConfirmationGrant = null;

function loadBaseUrl() {
  const saved = window.localStorage.getItem("indwell.console.baseUrl");
  return saved || DEFAULT_BASE_URL;
}

function normalizeBaseUrl(value) {
  return value.trim().replace(/\/+$/, "");
}

function setBaseUrl(value) {
  const normalized = normalizeBaseUrl(value || DEFAULT_BASE_URL);
  els.baseUrl.value = normalized;
  window.localStorage.setItem("indwell.console.baseUrl", normalized);
}

function setStatus(kind, text) {
  els.statusDot.className = `status-dot status-${kind}`;
  els.statusText.textContent = text;
}

function logApi(method, path, request, response) {
  const entry = {
    at: new Date().toISOString(),
    method,
    path,
    request,
    response,
  };
  const next = `${JSON.stringify(entry, null, 2)}\n\n${els.apiLog.textContent}`;
  els.apiLog.textContent = next.slice(0, 12000);
}

async function requestJson(path, options = {}) {
  const baseUrl = normalizeBaseUrl(els.baseUrl.value);
  const method = options.method || "GET";
  const requestBody = options.body;
  const sessionToken = window.localStorage.getItem(STORAGE_SESSION_TOKEN);
  const init = {
    method,
    headers: {
      Accept: "application/json",
      ...(sessionToken ? { Authorization: `Bearer ${sessionToken}` } : {}),
      ...(requestBody === undefined ? {} : { "Content-Type": "application/json" }),
    },
    body: requestBody === undefined ? undefined : JSON.stringify(requestBody),
  };

  let payload;
  try {
    const response = await fetch(`${baseUrl}${path}`, init);
    payload = await response.json();
    logApi(method, path, requestBody || null, payload);

    if (!response.ok || !payload.ok) {
      throw new Error(payload.error || `HTTP ${response.status}`);
    }

    return payload.data;
  } catch (error) {
    logApi(method, path, requestBody || null, { ok: false, error: error.message });
    throw error;
  }
}

function setBusy(button, busy) {
  button.disabled = busy;
  button.dataset.originalText ||= button.textContent;
  button.textContent = busy ? "Working..." : button.dataset.originalText;
}

function renderError(target, error) {
  target.classList.remove("empty");
  target.innerHTML = `<strong>Request failed</strong><p>${escapeHtml(error.message)}</p>`;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

function formatTime(ms) {
  if (!ms) {
    return "unknown time";
  }
  return new Date(ms).toLocaleString();
}

function formatJson(value) {
  return JSON.stringify(value, null, 2);
}

function utf8Bytes(value) {
  return new TextEncoder().encode(value);
}

function bytesToHex(bytes) {
  return Array.from(bytes)
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

function hexToBytes(value) {
  const trimmed = value.trim();
  const out = new Uint8Array(trimmed.length / 2);
  for (let i = 0; i < out.length; i += 1) {
    out[i] = Number.parseInt(trimmed.slice(i * 2, i * 2 + 2), 16);
  }
  return out;
}

async function sha256Hex(value) {
  const bytes = value instanceof Uint8Array ? value : utf8Bytes(value);
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return bytesToHex(new Uint8Array(digest));
}

function randomHex(byteLength = 16) {
  const bytes = new Uint8Array(byteLength);
  crypto.getRandomValues(bytes);
  return bytesToHex(bytes);
}

function pairingPayload(sessionId, code, label, publicKeyBytes, publicKeySha256) {
  return [
    "indwell-pairing-v1",
    `session_id=${sessionId.trim()}`,
    `code=${code.trim().toUpperCase()}`,
    `label=${label.trim()}`,
    `public_key_sha256=${publicKeySha256}`,
    "",
  ].join("\n");
}

function signedRequestPayload(request) {
  return [
    "indwell-request-v1",
    `device_id=${request.device_id.trim()}`,
    `timestamp_ms=${request.timestamp_ms}`,
    `nonce=${request.nonce.trim()}`,
    `method=${request.method.trim().toUpperCase()}`,
    `path=${request.path.trim()}`,
    `body_sha256=${request.body_sha256.trim().toLowerCase()}`,
    "",
  ].join("\n");
}

function loadJsonStorage(key) {
  const value = window.localStorage.getItem(key);
  return value ? JSON.parse(value) : null;
}

function saveJsonStorage(key, value) {
  window.localStorage.setItem(key, JSON.stringify(value));
}

function hasSessionToken() {
  return Boolean(window.localStorage.getItem(STORAGE_SESSION_TOKEN));
}

async function ensureConsoleKeyPair() {
  if (!crypto?.subtle) {
    throw new Error("WebCrypto is not available in this browser context.");
  }

  const publicJwk = loadJsonStorage(STORAGE_PUBLIC_KEY_JWK);
  const privateJwk = loadJsonStorage(STORAGE_PRIVATE_KEY_JWK);
  if (publicJwk && privateJwk) {
    const publicKey = await crypto.subtle.importKey(
      "jwk",
      publicJwk,
      { name: "Ed25519" },
      true,
      ["verify"],
    );
    const privateKey = await crypto.subtle.importKey(
      "jwk",
      privateJwk,
      { name: "Ed25519" },
      true,
      ["sign"],
    );
    return { publicKey, privateKey };
  }

  const pair = await crypto.subtle.generateKey({ name: "Ed25519" }, true, [
    "sign",
    "verify",
  ]);
  saveJsonStorage(STORAGE_PUBLIC_KEY_JWK, await crypto.subtle.exportKey("jwk", pair.publicKey));
  saveJsonStorage(STORAGE_PRIVATE_KEY_JWK, await crypto.subtle.exportKey("jwk", pair.privateKey));
  return pair;
}

async function exportRawPublicKeyHex(publicKey) {
  const raw = await crypto.subtle.exportKey("raw", publicKey);
  return bytesToHex(new Uint8Array(raw));
}

async function signHex(privateKey, payload) {
  const signature = await crypto.subtle.sign({ name: "Ed25519" }, privateKey, utf8Bytes(payload));
  return bytesToHex(new Uint8Array(signature));
}

function renderSessionState() {
  const paired = loadJsonStorage(STORAGE_PAIRED_DEVICE);
  const token = window.localStorage.getItem(STORAGE_SESSION_TOKEN);

  els.sessionResult.classList.remove("empty");
  els.sessionResult.innerHTML = `
    <dl class="kv">
      <div><dt>Paired device</dt><dd>${escapeHtml(paired?.device_id || "none")}</dd></div>
      <div><dt>Session token</dt><dd>${escapeHtml(token ? "present" : "missing")}</dd></div>
      <div><dt>Subject</dt><dd>${escapeHtml(paired?.subject_id || "owner")}</dd></div>
    </dl>
  `;
}

function renderProtectedApiHint(target, label) {
  target.classList.remove("empty");
  target.innerHTML = `<strong>Session required</strong><p>Complete signed pairing and issue a session before loading ${escapeHtml(label)}.</p>`;
}

function requireSessionFor(target, label) {
  if (hasSessionToken()) {
    return true;
  }
  renderProtectedApiHint(target, label);
  renderSessionState();
  return false;
}

function collectMemoryQuery() {
  return {
    wing: els.memoryWing.value.trim() || null,
    room: els.memoryRoom.value.trim() || null,
    text: els.memoryText.value.trim() || null,
    limit: Number.parseInt(els.memoryLimit.value, 10) || 10,
  };
}

function collectNewMemory() {
  return {
    kind: els.addMemoryKind.value,
    wing: els.addMemoryWing.value.trim() || "user_unknown",
    room: els.addMemoryRoom.value.trim() || "preferences",
    content: els.addMemoryContent.value.trim(),
  };
}

function renderMemoryCard(record) {
  const encoded = encodeURIComponent(JSON.stringify(record));
  const tags = Array.isArray(record.tags) ? record.tags : [];
  const isUnverified = tags.includes("unverified_ingress");
  return `
    <section class="memory-card" data-memory-id="${escapeHtml(record.id)}">
      <header>
        <span>${escapeHtml(record.kind)}</span>
        <span>${escapeHtml(record.wing)} / ${escapeHtml(record.room)}</span>
        <span>${escapeHtml(formatTime(record.created_at_ms))}</span>
      </header>
      <p>${escapeHtml(record.content)}</p>
      <p class="subtle">ID: ${escapeHtml(record.id)} · source: ${escapeHtml(formatMemorySource(record.source))} · confidence: ${escapeHtml(record.confidence ?? "n/a")}${isUnverified ? " · inbox/unverified" : ""}</p>
      <div class="memory-card-actions">
        <button class="secondary compact" type="button" data-memory-copy-id="${escapeHtml(record.id)}">Copy ID</button>
        <button class="secondary compact" type="button" data-memory-audit-id="${escapeHtml(record.id)}">Audit</button>
        <button class="secondary compact" type="button" data-memory-json="${encoded}">Show JSON</button>
        ${isUnverified ? `<button class="secondary compact" type="button" data-memory-accept-id="${escapeHtml(record.id)}">Accept</button>` : ""}
        <button class="secondary compact danger" type="button" data-memory-delete-id="${escapeHtml(record.id)}">Delete</button>
      </div>
    </section>
  `;
}

function formatMemorySource(source) {
  if (!source) {
    return "unknown";
  }
  if (typeof source === "string") {
    return source;
  }
  if (source.agent_run?.run_id) {
    return `agent_run:${source.agent_run.run_id}`;
  }
  return JSON.stringify(source);
}

function describePolicyDecision(decision) {
  if (typeof decision === "string") {
    return {
      decision,
      detail: decision === "allow" ? "Tool call may proceed." : "Additional authorization is required.",
    };
  }

  if (decision && typeof decision === "object" && decision.deny) {
    return {
      decision: "deny",
      detail: decision.deny.reason || "Policy denied the tool call.",
    };
  }

  return {
    decision: "unknown",
    detail: JSON.stringify(decision),
  };
}

async function checkHealth() {
  try {
    const data = await requestJson("/health");
    setStatus("ok", `${data.service}: ${data.status}`);
  } catch (error) {
    setStatus("error", error.message);
  }
}

const OPTIONAL_PROVIDER_FIELDS = {
  vision: {
    kind: "providerVisionKind",
    model: "providerVisionModel",
    baseUrl: "providerVisionBaseUrl",
    apiKeyRef: "providerVisionApiKeyRef",
  },
  asr: {
    kind: "providerAsrKind",
    model: "providerAsrModel",
    baseUrl: "providerAsrBaseUrl",
    apiKeyRef: "providerAsrApiKeyRef",
  },
  tts: {
    kind: "providerTtsKind",
    model: "providerTtsModel",
    baseUrl: "providerTtsBaseUrl",
    apiKeyRef: "providerTtsApiKeyRef",
  },
  embedding: {
    kind: "providerEmbeddingKind",
    model: "providerEmbeddingModel",
    baseUrl: "providerEmbeddingBaseUrl",
    apiKeyRef: "providerEmbeddingApiKeyRef",
  },
};

function setOptionalProviderFields(name, config) {
  const fields = OPTIONAL_PROVIDER_FIELDS[name];
  els[fields.kind].value = config?.kind || "";
  els[fields.model].value = config?.model || "";
  els[fields.baseUrl].value = config?.base_url || "";
  els[fields.apiKeyRef].value = config?.api_key_ref || "";
}

function collectOptionalProvider(name) {
  const fields = OPTIONAL_PROVIDER_FIELDS[name];
  const kind = els[fields.kind].value.trim();
  if (!kind) {
    return null;
  }
  return {
    kind,
    base_url: els[fields.baseUrl].value.trim() || null,
    api_key_ref: els[fields.apiKeyRef].value.trim() || null,
    model: els[fields.model].value.trim(),
    max_input_tokens: null,
    max_output_tokens: null,
  };
}

function providerSummary(label, config) {
  if (!config) {
    return `<div><dt>${escapeHtml(label)}</dt><dd>disabled</dd></div>`;
  }
  return `<div><dt>${escapeHtml(label)}</dt><dd>${escapeHtml(config.kind)} · ${escapeHtml(config.model || "inherits model")}</dd></div>`;
}

function renderProviderConfig(config) {
  const llm = config.llm || {};
  els.providerKind.value = llm.kind || "mock";
  els.providerModel.value = llm.model || "mock:phase0";
  els.providerBaseUrl.value = llm.base_url || "";
  els.providerApiKeyRef.value = llm.api_key_ref || "";
  els.providerMaxOutput.value = llm.max_output_tokens || 600;
  setOptionalProviderFields("vision", config.vision);
  setOptionalProviderFields("asr", config.asr);
  setOptionalProviderFields("tts", config.tts);
  setOptionalProviderFields("embedding", config.embedding);

  els.providerResult.classList.remove("empty");
  els.providerResult.innerHTML = `
    <dl class="kv">
      <div><dt>LLM</dt><dd>${escapeHtml(llm.kind || "unknown")} · ${escapeHtml(llm.model || "unknown")}</dd></div>
      <div><dt>Key ref</dt><dd>${escapeHtml(llm.api_key_ref || "none")}</dd></div>
      ${providerSummary("Vision", config.vision)}
      ${providerSummary("ASR", config.asr)}
      ${providerSummary("TTS", config.tts)}
      ${providerSummary("Embedding", config.embedding)}
    </dl>
    <pre class="json-preview">${escapeHtml(formatJson(config))}</pre>
  `;
}

async function loadProviderConfig() {
  if (!requireSessionFor(els.providerResult, "provider config")) {
    return;
  }
  setBusy(els.loadProviderButton, true);

  try {
    const config = await requestJson("/v1/providers");
    renderProviderConfig(config);
  } catch (error) {
    renderError(els.providerResult, error);
  } finally {
    setBusy(els.loadProviderButton, false);
  }
}

async function saveProviderConfig(event) {
  event.preventDefault();
  if (!requireSessionFor(els.providerResult, "provider config")) {
    return;
  }
  const button = event.submitter;
  setBusy(button, true);

  const config = collectProviderConfig();

  try {
    const saved = await requestJson("/v1/providers", {
      method: "PUT",
      body: config,
    });
    renderProviderConfig(saved);
  } catch (error) {
    renderError(els.providerResult, error);
  } finally {
    setBusy(button, false);
  }
}

function collectProviderConfig() {
  const maxOutput = Number.parseInt(els.providerMaxOutput.value, 10);
  return {
    llm: {
      kind: els.providerKind.value.trim() || "mock",
      base_url: els.providerBaseUrl.value.trim() || null,
      api_key_ref: els.providerApiKeyRef.value.trim() || null,
      model: els.providerModel.value.trim() || "mock:phase0",
      max_input_tokens: 4000,
      max_output_tokens: Number.isFinite(maxOutput) ? maxOutput : null,
    },
    vision: collectOptionalProvider("vision"),
    asr: collectOptionalProvider("asr"),
    tts: collectOptionalProvider("tts"),
    embedding: collectOptionalProvider("embedding"),
  };
}

async function saveProviderSecret() {
  if (!requireSessionFor(els.providerResult, "provider secrets")) {
    return;
  }
  setBusy(els.saveSecretButton, true);
  const keyRef = els.providerApiKeyRef.value.trim() || "key_llm_main";

  try {
    const stored = await requestJson(`/v1/secrets/${encodeURIComponent(keyRef)}`, {
      method: "PUT",
      body: {
        secret: els.providerApiKeySecret.value,
      },
    });
    els.providerApiKeySecret.value = "";
    els.providerResult.classList.remove("empty");
    els.providerResult.innerHTML = `
      <dl class="kv">
        <div><dt>Secret ref</dt><dd>${escapeHtml(stored.key_ref)}</dd></div>
        <div><dt>Fingerprint</dt><dd>${escapeHtml(stored.fingerprint)}</dd></div>
        <div><dt>Stored</dt><dd>${escapeHtml(formatTime(stored.stored_at_ms))}</dd></div>
      </dl>
    `;
  } catch (error) {
    renderError(els.providerResult, error);
  } finally {
    setBusy(els.saveSecretButton, false);
  }
}

async function testProvider(target, button) {
  if (!requireSessionFor(els.providerResult, "provider diagnostics")) {
    return;
  }
  setBusy(button, true);

  try {
    const result = await requestJson("/v1/providers/test", {
      method: "POST",
      body: { target },
    });
    els.providerResult.classList.remove("empty");
    els.providerResult.innerHTML = `
      <dl class="kv">
        <div><dt>Target</dt><dd>${escapeHtml(result.target)}</dd></div>
        <div><dt>Status</dt><dd>${result.ok ? "OK" : "Failed"}</dd></div>
        <div><dt>Summary</dt><dd>${escapeHtml(result.summary)}</dd></div>
        <div><dt>Provider</dt><dd>${escapeHtml(result.provider?.kind || "mock/default")}</dd></div>
      </dl>
      <pre class="json-preview">${escapeHtml(formatJson(result.details))}</pre>
    `;
  } catch (error) {
    renderError(els.providerResult, error);
  } finally {
    setBusy(button, false);
  }
}

async function saveProvisioning(event) {
  event.preventDefault();
  if (!requireSessionFor(els.provisioningResult, "provisioning")) {
    return;
  }
  const button = event.submitter;
  setBusy(button, true);

  try {
    const response = await requestJson("/v1/provisioning", {
      method: "POST",
      body: {
        device_id: els.provisioningDeviceId.value.trim() || "indwell-proto-v1",
        wifi: {
          ssid: els.provisioningSsid.value.trim(),
          password_ref: els.provisioningPasswordRef.value.trim() || null,
        },
        providers: collectProviderConfig(),
        owner_pairing_label: els.provisioningOwnerLabel.value.trim() || null,
      },
    });
    els.provisioningResult.classList.remove("empty");
    els.provisioningResult.innerHTML = `
      <dl class="kv">
        <div><dt>Accepted</dt><dd>${escapeHtml(String(response.accepted))}</dd></div>
        <div><dt>Next state</dt><dd>${escapeHtml(response.next_state)}</dd></div>
        <div><dt>Message</dt><dd>${escapeHtml(response.message)}</dd></div>
      </dl>
    `;
  } catch (error) {
    renderError(els.provisioningResult, error);
  } finally {
    setBusy(button, false);
  }
}

async function loadProvisioning() {
  if (!requireSessionFor(els.provisioningResult, "provisioning")) {
    return;
  }
  setBusy(els.loadProvisioningButton, true);

  try {
    const provisioning = await requestJson("/v1/provisioning");
    els.provisioningResult.classList.remove("empty");
    els.provisioningResult.innerHTML = provisioning
      ? `<pre class="json-preview">${escapeHtml(formatJson(provisioning))}</pre>`
      : "No provisioning config stored.";
  } catch (error) {
    renderError(els.provisioningResult, error);
  } finally {
    setBusy(els.loadProvisioningButton, false);
  }
}

async function sendChannelInput(event) {
  event.preventDefault();
  const button = event.submitter;
  setBusy(button, true);

  try {
    const data = await requestJson("/v1/channel/input", {
      method: "POST",
      body: {
        channel: "local_pwa",
        session_id: SESSION_ID,
        subject_hint: els.subjectHint.value.trim() || null,
        text: els.channelText.value.trim(),
      },
    });

    els.channelResult.classList.remove("empty");
    els.channelResult.innerHTML = `
      <dl class="kv">
        <div><dt>Reply</dt><dd>${escapeHtml(data.reply)}</dd></div>
        <div><dt>Run ID</dt><dd>${escapeHtml(data.run_id || "none")}</dd></div>
        <div><dt>Memory ID</dt><dd>${escapeHtml(data.memory_id || "none")}</dd></div>
        <div><dt>Event</dt><dd>${escapeHtml(data.event.type || "unknown")}</dd></div>
      </dl>
    `;
  } catch (error) {
    renderError(els.channelResult, error);
  } finally {
    setBusy(button, false);
  }
}

async function runVoiceTurn(event) {
  event.preventDefault();
  const button = event.submitter;
  setBusy(button, true);

  try {
    const data = await requestJson("/v1/voice/mock-turn", {
      method: "POST",
      body: {
        text_hint: els.voiceText.value.trim(),
        voice: "warm_indwell",
      },
    });
    els.voiceResult.classList.remove("empty");
    els.voiceResult.innerHTML = `
      <dl class="kv">
        <div><dt>Transcript</dt><dd>${escapeHtml(data.transcript.text)}</dd></div>
        <div><dt>Reply</dt><dd>${escapeHtml(data.reply)}</dd></div>
        <div><dt>Audio</dt><dd>${escapeHtml(data.audio.mime_type)} / ${escapeHtml(data.audio.duration_ms || 0)}ms</dd></div>
      </dl>
      <pre class="json-preview">${escapeHtml(formatJson(data))}</pre>
    `;
  } catch (error) {
    renderError(els.voiceResult, error);
  } finally {
    setBusy(button, false);
  }
}

async function runMemorySearch(query, button) {
  if (!requireSessionFor(els.memoryResult, "memory")) {
    return;
  }
  setBusy(button, true);

  try {
    const records = await requestJson("/v1/memory/search", {
      method: "POST",
      body: query,
    });

    els.memoryResult.classList.remove("empty");
    if (!records.length) {
      els.memoryResult.textContent = "No matching memories.";
      return;
    }

    els.memoryResult.innerHTML = records.map(renderMemoryCard).join("");
  } catch (error) {
    renderError(els.memoryResult, error);
  } finally {
    setBusy(button, false);
  }
}

async function searchMemory(event) {
  event.preventDefault();
  const query = collectMemoryQuery();
  lastMemoryQuery = query;
  await runMemorySearch(query, event.submitter);
}

async function refreshMemorySearch() {
  const query = lastMemoryQuery || collectMemoryQuery();
  lastMemoryQuery = query;
  await runMemorySearch(query, els.refreshMemoryButton);
}

async function reviewMemoryInbox() {
  const query = {
    wing: "inbox",
    room: "unverified",
    text: null,
    limit: 50,
  };
  lastMemoryQuery = query;
  await runMemorySearch(query, els.reviewInboxButton);
}

async function addMemory(event) {
  event.preventDefault();
  if (!requireSessionFor(els.memoryResult, "memory")) {
    return;
  }
  const button = event.submitter;
  setBusy(button, true);

  try {
    const memory = collectNewMemory();
    if (!memory.content) {
      throw new Error("Memory content is required.");
    }
    const record = await requestJson("/v1/memory", {
      method: "POST",
      body: memory,
    });
    els.addMemoryContent.value = "";
    els.memoryExportResult.classList.remove("empty");
    els.memoryExportResult.innerHTML = `
      <strong>Memory added</strong>
      <dl class="kv">
        <div><dt>ID</dt><dd>${escapeHtml(record.id)}</dd></div>
        <div><dt>Room</dt><dd>${escapeHtml(record.wing)} / ${escapeHtml(record.room)}</dd></div>
      </dl>
      <pre class="json-preview">${escapeHtml(formatJson(record))}</pre>
    `;
    await refreshMemorySearch();
  } catch (error) {
    renderError(els.memoryExportResult, error);
  } finally {
    setBusy(button, false);
  }
}

async function auditMemory(memoryId, button) {
  if (!requireSessionFor(els.memoryExportResult, "memory audit")) {
    return;
  }
  setBusy(button, true);

  try {
    const audit = await requestJson(`/v1/memory/${encodeURIComponent(memoryId)}/audit`);
    els.memoryExportResult.classList.remove("empty");
    els.memoryExportResult.innerHTML = `
      <strong>Memory audit</strong>
      <dl class="kv">
        <div><dt>ID</dt><dd>${escapeHtml(audit.record?.id || memoryId)}</dd></div>
        <div><dt>Status</dt><dd>${escapeHtml(audit.status)}</dd></div>
        <div><dt>Related run</dt><dd>${escapeHtml(audit.related_run_id || "none")}</dd></div>
        <div><dt>Recommendation</dt><dd>${escapeHtml(audit.recommendation)}</dd></div>
      </dl>
      <pre class="json-preview">${escapeHtml(formatJson(audit))}</pre>
    `;
  } catch (error) {
    renderError(els.memoryExportResult, error);
  } finally {
    setBusy(button, false);
  }
}

async function acceptMemory(memoryId, button) {
  if (!requireSessionFor(els.memoryExportResult, "memory review")) {
    return;
  }
  setBusy(button, true);

  try {
    const accepted = await requestJson(`/v1/memory/${encodeURIComponent(memoryId)}/accept`, {
      method: "POST",
      body: {
        wing: "user_unknown",
        room: "episodes",
        confidence: 0.75,
        importance: 0.45,
      },
    });
    els.memoryExportResult.classList.remove("empty");
    els.memoryExportResult.innerHTML = `
      <strong>Inbox memory accepted</strong>
      <dl class="kv">
        <div><dt>ID</dt><dd>${escapeHtml(accepted.record?.id || memoryId)}</dd></div>
        <div><dt>Room</dt><dd>${escapeHtml(accepted.record?.wing || "unknown")} / ${escapeHtml(accepted.record?.room || "unknown")}</dd></div>
        <div><dt>Confidence</dt><dd>${escapeHtml(accepted.record?.confidence ?? "n/a")}</dd></div>
      </dl>
      <pre class="json-preview">${escapeHtml(formatJson(accepted.record || accepted))}</pre>
    `;
    await refreshMemorySearch();
    refreshRuns();
  } catch (error) {
    renderError(els.memoryExportResult, error);
  } finally {
    setBusy(button, false);
  }
}

async function deleteMemory(memoryId, button) {
  if (!requireSessionFor(els.memoryResult, "memory deletion")) {
    return;
  }
  setBusy(button, true);

  try {
    const data = await executeTool(
      "memory.delete",
      "local_pwa",
      { id: memoryId },
      button,
      { render: false },
    );
    els.memoryExportResult.classList.remove("empty");
    els.memoryExportResult.innerHTML = `
      <strong>Memory delete requested</strong>
      <dl class="kv">
        <div><dt>ID</dt><dd>${escapeHtml(memoryId)}</dd></div>
        <div><dt>Run ID</dt><dd>${escapeHtml(data.run_id || "none")}</dd></div>
        <div><dt>Decision</dt><dd>${escapeHtml(describePolicyDecision(data.decision).decision)}</dd></div>
      </dl>
      <pre class="json-preview">${escapeHtml(formatJson(data.output || data))}</pre>
    `;
    await refreshMemorySearch();
    refreshRuns();
  } catch (error) {
    renderError(els.memoryExportResult, error);
  } finally {
    setBusy(button, false);
  }
}

async function copyMemoryId(memoryId, button) {
  try {
    await navigator.clipboard.writeText(memoryId);
    button.dataset.originalText ||= button.textContent;
    button.textContent = "Copied";
    window.setTimeout(() => {
      button.textContent = button.dataset.originalText;
    }, 1100);
  } catch (error) {
    renderError(els.memoryExportResult, error);
  }
}

function showMemoryJson(encodedRecord) {
  const record = JSON.parse(decodeURIComponent(encodedRecord));
  els.memoryExportResult.classList.remove("empty");
  els.memoryExportResult.innerHTML = `
    <strong>Memory JSON</strong>
    <pre class="json-preview">${escapeHtml(formatJson(record))}</pre>
  `;
}

async function exportMemory() {
  if (!requireSessionFor(els.memoryExportResult, "memory export")) {
    return;
  }
  setBusy(els.exportMemoryButton, true);

  try {
    const data = await requestJson("/v1/memory/export");
    const records = Array.isArray(data) ? data : Array.isArray(data?.records) ? data.records : [];
    const snapshotKeys = data?.snapshots && typeof data.snapshots === "object"
      ? Object.keys(data.snapshots)
      : [];

    els.memoryExportResult.classList.remove("empty");
    els.memoryExportResult.innerHTML = `
      <dl class="kv">
        <div><dt>Records</dt><dd>${escapeHtml(records.length)}</dd></div>
        <div><dt>Snapshots</dt><dd>${escapeHtml(snapshotKeys.length ? snapshotKeys.join(", ") : "none or unavailable")}</dd></div>
      </dl>
      <pre class="json-preview">${escapeHtml(formatJson(data))}</pre>
    `;
  } catch (error) {
    renderError(els.memoryExportResult, error);
  } finally {
    setBusy(els.exportMemoryButton, false);
  }
}

async function metabolizeMemory() {
  if (!requireSessionFor(els.memoryExportResult, "memory metabolism")) {
    return;
  }
  setBusy(els.metabolizeMemoryButton, true);

  try {
    const report = await requestJson("/v1/memory/metabolize", { method: "POST" });
    els.memoryExportResult.classList.remove("empty");
    els.memoryExportResult.innerHTML = `
      <dl class="kv">
        <div><dt>Decayed</dt><dd>${escapeHtml((report.decayed || []).length)}</dd></div>
        <div><dt>Expired</dt><dd>${escapeHtml((report.expired || []).length)}</dd></div>
        <div><dt>Consolidated</dt><dd>${escapeHtml((report.consolidated || []).length)}</dd></div>
      </dl>
      <pre class="json-preview">${escapeHtml(formatJson(report))}</pre>
    `;
    refreshMemorySearch();
  } catch (error) {
    renderError(els.memoryExportResult, error);
  } finally {
    setBusy(els.metabolizeMemoryButton, false);
  }
}

async function runReflection(event) {
  event.preventDefault();
  if (!requireSessionFor(els.reflectionResult, "reflection")) {
    return;
  }
  const button = event.submitter;
  setBusy(button, true);

  try {
    const report = await requestJson("/v1/reflection/run", {
      method: "POST",
      body: {
        limit: Number.parseInt(els.reflectionLimit.value, 10) || 20,
        allow_sensitive: false,
        allow_skill_generation: els.reflectionSkills.checked,
      },
    });
    els.reflectionResult.classList.remove("empty");
    els.reflectionResult.innerHTML = `
      <dl class="kv">
        <div><dt>New memories</dt><dd>${escapeHtml((report.new_memories || []).length)}</dd></div>
        <div><dt>Skills</dt><dd>${escapeHtml((report.skills || []).length)}</dd></div>
        <div><dt>Warnings</dt><dd>${escapeHtml((report.warnings || []).length)}</dd></div>
      </dl>
      <pre class="json-preview">${escapeHtml(formatJson(report))}</pre>
    `;
    refreshMemorySearch();
  } catch (error) {
    renderError(els.reflectionResult, error);
  } finally {
    setBusy(button, false);
  }
}

async function issuePairingChallenge() {
  setBusy(els.pairingChallengeButton, true);

  try {
    const challenge = await requestJson("/v1/pairing/challenge", { method: "POST" });
    els.pairingSessionId.value = challenge.session_id;
    els.pairingCode.value = challenge.code;
    els.pairingResult.classList.remove("empty");
    els.pairingResult.innerHTML = `
      <dl class="kv">
        <div><dt>Session</dt><dd>${escapeHtml(challenge.session_id)}</dd></div>
        <div><dt>Code</dt><dd>${escapeHtml(challenge.code)}</dd></div>
        <div><dt>Expires</dt><dd>${escapeHtml(formatTime(challenge.expires_at_ms))}</dd></div>
      </dl>
    `;
  } catch (error) {
    renderError(els.pairingResult, error);
  } finally {
    setBusy(els.pairingChallengeButton, false);
  }
}

async function completePairing(event) {
  event.preventDefault();
  const button = event.submitter;
  setBusy(button, true);

  try {
    const keyPair = await ensureConsoleKeyPair();
    const publicKeyHex = await exportRawPublicKeyHex(keyPair.publicKey);
    const publicKeyBytes = hexToBytes(publicKeyHex);
    const label = els.pairingLabel.value.trim() || "Paired phone";
    const publicKeySha256 = await sha256Hex(publicKeyBytes);
    const payload = pairingPayload(
      els.pairingSessionId.value,
      els.pairingCode.value,
      label,
      publicKeyBytes,
      publicKeySha256,
    );
    const signature = await signHex(keyPair.privateKey, payload);
    const paired = await requestJson("/v1/pairing/complete", {
      method: "POST",
      body: {
        session_id: els.pairingSessionId.value.trim(),
        code: els.pairingCode.value.trim(),
        label,
        public_key: publicKeyHex,
        signature,
      },
    });
    saveJsonStorage(STORAGE_PAIRED_DEVICE, { ...paired, subject_id: "owner" });
    els.pairingResult.classList.remove("empty");
    els.pairingResult.innerHTML = `
      <dl class="kv">
        <div><dt>Device ID</dt><dd>${escapeHtml(paired.device_id)}</dd></div>
        <div><dt>Label</dt><dd>${escapeHtml(paired.label)}</dd></div>
        <div><dt>Key hash</dt><dd>${escapeHtml(paired.public_key_hash)}</dd></div>
      </dl>
    `;
    await issueAuthSession();
  } catch (error) {
    renderError(els.pairingResult, error);
  } finally {
    setBusy(button, false);
  }
}

async function issueAuthSession(button = els.issueSessionButton) {
  setBusy(button, true);

  try {
    const paired = loadJsonStorage(STORAGE_PAIRED_DEVICE);
    if (!paired?.device_id) {
      throw new Error("Complete pairing before issuing a session.");
    }
    const keyPair = await ensureConsoleKeyPair();
    const request = {
      device_id: paired.device_id,
      subject_id: paired.subject_id || "owner",
      timestamp_ms: Date.now(),
      nonce: randomHex(16),
      method: "POST",
      path: "/v1/auth/session",
      body_sha256: await sha256Hex(""),
    };
    const signature = await signHex(keyPair.privateKey, signedRequestPayload(request));
    const response = await requestJson("/v1/auth/session", {
      method: "POST",
      body: {
        ...request,
        signature,
      },
    });

    window.localStorage.setItem(STORAGE_SESSION_TOKEN, response.token);
    saveJsonStorage(STORAGE_PAIRED_DEVICE, {
      ...paired,
      subject_id: response.session.subject_id,
      session_expires_at_ms: response.session.expires_at_ms,
    });
    els.sessionResult.classList.remove("empty");
    els.sessionResult.innerHTML = `
      <strong>Session issued</strong>
      <dl class="kv">
        <div><dt>Device</dt><dd>${escapeHtml(response.session.device_id)}</dd></div>
        <div><dt>Subject</dt><dd>${escapeHtml(response.session.subject_id)}</dd></div>
        <div><dt>Expires</dt><dd>${escapeHtml(formatTime(response.session.expires_at_ms))}</dd></div>
      </dl>
    `;
    await loadProtectedDashboard();
  } catch (error) {
    renderError(els.sessionResult, error);
  } finally {
    setBusy(button, false);
  }
}

function clearConsoleSession() {
  window.localStorage.removeItem(STORAGE_SESSION_TOKEN);
  renderSessionState();
}

async function listPairedDevices() {
  if (!requireSessionFor(els.pairingResult, "paired devices")) {
    return;
  }
  setBusy(els.listPairingButton, true);

  try {
    const devices = await requestJson("/v1/pairing/devices");
    els.pairingResult.classList.remove("empty");
    els.pairingResult.innerHTML = devices.length
      ? `<pre class="json-preview">${escapeHtml(formatJson(devices))}</pre>`
      : "No paired devices.";
  } catch (error) {
    renderError(els.pairingResult, error);
  } finally {
    setBusy(els.listPairingButton, false);
  }
}

async function issuePassphraseChallenge() {
  setBusy(els.passphraseChallengeButton, true);

  try {
    const challenge = await requestJson("/v1/auth/passphrase/challenge", { method: "POST" });
    els.passphraseChallengeId.value = challenge.challenge_id;
    els.passphraseSpoken.value = challenge.phrase;
    els.passphraseResult.classList.remove("empty");
    els.passphraseResult.innerHTML = `
      <dl class="kv">
        <div><dt>Challenge</dt><dd>${escapeHtml(challenge.challenge_id)}</dd></div>
        <div><dt>Phrase</dt><dd>${escapeHtml(challenge.phrase)}</dd></div>
        <div><dt>Expires</dt><dd>${escapeHtml(formatTime(challenge.expires_at_ms))}</dd></div>
      </dl>
    `;
  } catch (error) {
    renderError(els.passphraseResult, error);
  } finally {
    setBusy(els.passphraseChallengeButton, false);
  }
}

async function verifyPassphraseChallenge(event) {
  event.preventDefault();
  const button = event.submitter;
  setBusy(button, true);

  try {
    const result = await requestJson("/v1/auth/passphrase/verify", {
      method: "POST",
      body: {
        challenge_id: els.passphraseChallengeId.value.trim(),
        spoken_phrase: els.passphraseSpoken.value.trim(),
        subject_id: "owner",
        allowed_tool: "system.update.apply",
      },
    });
    lastConfirmationGrant = result.grant;
    els.passphraseResult.classList.remove("empty");
    els.passphraseResult.innerHTML = `
      <strong>Verified</strong>
      <dl class="kv">
        <div><dt>Grant</dt><dd>${escapeHtml(result.grant.grant_id)}</dd></div>
        <div><dt>Tool</dt><dd>${escapeHtml(result.grant.allowed_tool)}</dd></div>
        <div><dt>Expires</dt><dd>${escapeHtml(formatTime(result.grant.expires_at_ms))}</dd></div>
      </dl>
    `;
  } catch (error) {
    renderError(els.passphraseResult, error);
  } finally {
    setBusy(button, false);
  }
}

async function checkCameraTool() {
  if (!requireSessionFor(els.toolResult, "tool policy checks")) {
    return;
  }
  setBusy(els.toolCheckButton, true);

  try {
    const data = await requestJson("/v1/tools/device.camera.capture/check", {
      method: "POST",
      body: "local_pwa",
    });

    const policy = describePolicyDecision(data.decision);
    els.toolResult.classList.remove("empty");
    els.toolResult.innerHTML = `
      <dl class="kv">
        <div><dt>Tool</dt><dd>${escapeHtml(data.tool)}</dd></div>
        <div><dt>Decision</dt><dd>${escapeHtml(policy.decision)}</dd></div>
        <div><dt>Detail</dt><dd>${escapeHtml(policy.detail)}</dd></div>
      </dl>
    `;
  } catch (error) {
    renderError(els.toolResult, error);
  } finally {
    setBusy(els.toolCheckButton, false);
  }
}

async function loadToolCatalog() {
  if (!requireSessionFor(els.toolCatalogResult, "tool catalog")) {
    return;
  }
  setBusy(els.loadToolsButton, true);

  try {
    const data = await requestJson("/v1/tools");
    const tools = data.tools || [];
    els.toolCatalogResult.classList.remove("empty");
    els.toolCatalogResult.innerHTML = tools
      .map(
        (tool) => `
          <section class="memory-card">
            <header>
              <span>${escapeHtml(tool.risk)}</span>
              <span>${escapeHtml(tool.name)}</span>
              <span>${tool.requires_confirmation ? "confirmation" : tool.requires_owner ? "owner" : "auto"}</span>
            </header>
            <p>${escapeHtml(tool.description)}</p>
          </section>
        `,
      )
      .join("");
  } catch (error) {
    renderError(els.toolCatalogResult, error);
  } finally {
    setBusy(els.loadToolsButton, false);
  }
}

function renderToolExecution(data) {
  const policy = describePolicyDecision(data.decision);
  els.toolResult.classList.remove("empty");
  els.toolResult.innerHTML = `
    <dl class="kv">
      <div><dt>Tool</dt><dd>${escapeHtml(data.tool)}</dd></div>
      <div><dt>Decision</dt><dd>${escapeHtml(policy.decision)}</dd></div>
      <div><dt>Run ID</dt><dd>${escapeHtml(data.run_id || "none")}</dd></div>
      <div><dt>Memory ID</dt><dd>${escapeHtml(data.memory_id || "none")}</dd></div>
      <div><dt>Detail</dt><dd>${escapeHtml(policy.detail)}</dd></div>
    </dl>
    <pre class="json-preview">${escapeHtml(formatJson(data.output || data))}</pre>
  `;
}

async function executeTool(tool, channel, input, button, options = {}) {
  if (!requireSessionFor(els.toolResult, "tool execution")) {
    return;
  }
  setBusy(button, true);

  try {
    const sessionToken = window.localStorage.getItem(STORAGE_SESSION_TOKEN);
    const data = await requestJson(`/v1/tools/${encodeURIComponent(tool)}/execute`, {
      method: "POST",
      body: {
        channel,
        subject_id: channel === "local_pwa" ? "owner" : null,
        session_token: sessionToken || null,
        confirmation_grant_id:
          lastConfirmationGrant && lastConfirmationGrant.allowed_tool === tool
            ? lastConfirmationGrant.grant_id
            : null,
        input,
      },
    });

    if (options.render !== false) {
      renderToolExecution(data);
      refreshRuns();
    }
    return data;
  } catch (error) {
    renderError(els.toolResult, error);
    throw error;
  } finally {
    setBusy(button, false);
  }
}

async function executeStatusTool() {
  await executeTool("system.status", "custom_webhook", {}, els.toolStatusButton);
}

async function executeSensorTool() {
  await executeTool(
    "device.sensor.read",
    "local_pwa",
    { sensor: "temperature" },
    els.toolSensorButton,
  );
}

async function executeOwnerCameraTool() {
  await executeTool(
    "device.camera.capture",
    "local_pwa",
    {},
    els.toolCameraOwnerButton,
  );
}

async function executeWriteMemoryTool() {
  await executeTool(
    "memory.write_candidate",
    "local_pwa",
    {
      wing: "user_unknown",
      room: "episodes",
      content: "PWA executed memory.write_candidate through Tool Runtime.",
    },
    els.toolWriteMemoryButton,
  );
}

async function executeBlockedCameraTool() {
  await executeTool("device.camera.capture", "telegram", {}, els.toolBlockedCameraButton);
}

async function loadOtaManifest() {
  if (!requireSessionFor(els.otaResult, "OTA manifest")) {
    return;
  }
  setBusy(els.loadOtaButton, true);

  try {
    const manifest = await requestJson("/v1/ota/manifest");
    els.otaResult.classList.remove("empty");
    els.otaResult.innerHTML = `
      <dl class="kv">
        <div><dt>Version</dt><dd>${escapeHtml(manifest.version)}</dd></div>
        <div><dt>Target</dt><dd>${escapeHtml(manifest.target)}</dd></div>
        <div><dt>Channel</dt><dd>${escapeHtml(manifest.channel)}</dd></div>
        <div><dt>SHA-256</dt><dd>${escapeHtml(manifest.sha256)}</dd></div>
      </dl>
      <pre class="json-preview">${escapeHtml(formatJson(manifest))}</pre>
    `;
  } catch (error) {
    renderError(els.otaResult, error);
  } finally {
    setBusy(els.loadOtaButton, false);
  }
}

async function checkOtaManifest() {
  if (!requireSessionFor(els.otaResult, "OTA verification")) {
    return;
  }
  setBusy(els.checkOtaButton, true);

  try {
    const report = await requestJson("/v1/ota/check", { method: "POST" });
    const failed = (report.checks || []).filter((check) => !check.passed);
    els.otaResult.classList.remove("empty");
    els.otaResult.innerHTML = `
      <dl class="kv">
        <div><dt>Valid</dt><dd>${escapeHtml(String(report.valid))}</dd></div>
        <div><dt>Version</dt><dd>${escapeHtml(report.version)}</dd></div>
        <div><dt>Target</dt><dd>${escapeHtml(report.target)}</dd></div>
        <div><dt>Failures</dt><dd>${escapeHtml(failed.length ? failed.map((check) => check.name).join(", ") : "none")}</dd></div>
      </dl>
      <pre class="json-preview">${escapeHtml(formatJson(report))}</pre>
    `;
  } catch (error) {
    renderError(els.otaResult, error);
  } finally {
    setBusy(els.checkOtaButton, false);
  }
}

async function executeUpdateCheckTool() {
  await executeTool("system.update.check", "local_pwa", {}, els.toolUpdateButton);
}

function renderChannelResponse(target, data) {
  target.classList.remove("empty");
  target.innerHTML = `
    <dl class="kv">
      <div><dt>Reply</dt><dd>${escapeHtml(data.reply)}</dd></div>
      <div><dt>Run ID</dt><dd>${escapeHtml(data.run_id || "none")}</dd></div>
      <div><dt>Memory ID</dt><dd>${escapeHtml(data.memory_id || "none")}</dd></div>
      <div><dt>Channel</dt><dd>${escapeHtml(data.event.channel || "unknown")}</dd></div>
      <div><dt>Event</dt><dd>${escapeHtml(data.event.type || "unknown")}</dd></div>
    </dl>
  `;
}

async function sendWebhookInput(event) {
  event.preventDefault();
  const button = event.submitter;
  setBusy(button, true);

  try {
    const data = await requestJson("/v1/gateway/custom-webhook", {
      method: "POST",
      body: {
        session_id: `webhook-${Date.now().toString(36)}`,
        subject_hint: els.subjectHint.value.trim() || null,
        text: els.webhookText.value.trim(),
        command: null,
        source: els.webhookSource.value.trim() || "local-test-gateway",
      },
    });

    renderChannelResponse(els.webhookResult, data);
  } catch (error) {
    renderError(els.webhookResult, error);
  } finally {
    setBusy(button, false);
  }
}

async function refreshRuns() {
  if (!requireSessionFor(els.runsResult, "run audit")) {
    return;
  }
  setBusy(els.refreshRunsButton, true);

  try {
    const runs = await requestJson("/v1/runs");
    els.runsResult.classList.remove("empty");
    if (!runs.length) {
      els.runsResult.textContent = "No runs recorded yet.";
      return;
    }

    els.runsResult.innerHTML = runs
      .slice()
      .reverse()
      .slice(0, 12)
      .map((run) => {
        const output = run.audit?.provider_output_summary || "no output summary";
        const written = run.audit?.written_memory_ids || [];
        return `
          <section class="memory-card">
            <header>
              <span>${escapeHtml(run.status)}</span>
              <span>${escapeHtml(run.id)}</span>
              <span>${escapeHtml(formatTime(run.created_at_ms))}</span>
            </header>
            <p>${escapeHtml(output)}</p>
            <p class="subtle">Written memories: ${escapeHtml(written.length ? written.join(", ") : "none")}</p>
            <button class="secondary compact" type="button" data-run-id="${escapeHtml(run.id)}">Fetch detail</button>
          </section>
        `;
      })
      .join("");
  } catch (error) {
    renderError(els.runsResult, error);
  } finally {
    setBusy(els.refreshRunsButton, false);
  }
}

async function fetchRunDetail(runId, button) {
  setBusy(button, true);

  try {
    const data = await requestJson(`/v1/runs/${encodeURIComponent(runId)}`);
    const run = data?.run || data;
    const audit = run?.audit || {};

    els.runDetailResult.classList.remove("empty");
    els.runDetailResult.innerHTML = `
      <dl class="kv">
        <div><dt>Run ID</dt><dd>${escapeHtml(run?.id || runId)}</dd></div>
        <div><dt>Status</dt><dd>${escapeHtml(run?.status || "unknown")}</dd></div>
        <div><dt>Created</dt><dd>${escapeHtml(formatTime(run?.created_at_ms))}</dd></div>
        <div><dt>Tools</dt><dd>${escapeHtml((audit.exposed_tool_names || audit.allowed_tool_names || []).join(", ") || "none recorded")}</dd></div>
      </dl>
      <pre class="json-preview">${escapeHtml(formatJson(data))}</pre>
    `;
  } catch (error) {
    renderError(els.runDetailResult, error);
  } finally {
    setBusy(button, false);
  }
}

function registerServiceWorker() {
  if (!("serviceWorker" in navigator)) {
    return;
  }

  navigator.serviceWorker.register("./sw.js").catch(() => {
    // Static file previews over file:// cannot register service workers.
  });
}

async function loadProtectedDashboard() {
  if (!hasSessionToken()) {
    renderProtectedApiHint(els.providerResult, "provider config");
    renderProtectedApiHint(els.toolCatalogResult, "tool catalog");
    renderProtectedApiHint(els.otaResult, "OTA manifest");
    renderProtectedApiHint(els.runsResult, "run audit");
    return;
  }

  await Promise.allSettled([
    loadProviderConfig(),
    loadToolCatalog(),
    loadOtaManifest(),
    refreshRuns(),
  ]);
}

setBaseUrl(loadBaseUrl());
els.connectionForm.addEventListener("submit", (event) => {
  event.preventDefault();
  setBaseUrl(els.baseUrl.value);
  checkHealth();
});
els.providerForm.addEventListener("submit", saveProviderConfig);
els.loadProviderButton.addEventListener("click", loadProviderConfig);
els.saveSecretButton.addEventListener("click", saveProviderSecret);
els.testLlmProviderButton.addEventListener("click", () =>
  testProvider("llm", els.testLlmProviderButton)
);
els.testVisionProviderButton.addEventListener("click", () =>
  testProvider("vision", els.testVisionProviderButton)
);
els.testAsrProviderButton.addEventListener("click", () =>
  testProvider("asr", els.testAsrProviderButton)
);
els.testTtsProviderButton.addEventListener("click", () =>
  testProvider("tts", els.testTtsProviderButton)
);
els.testEmbeddingProviderButton.addEventListener("click", () =>
  testProvider("embedding", els.testEmbeddingProviderButton)
);
els.provisioningForm.addEventListener("submit", saveProvisioning);
els.loadProvisioningButton.addEventListener("click", loadProvisioning);
els.channelForm.addEventListener("submit", sendChannelInput);
els.voiceForm.addEventListener("submit", runVoiceTurn);
els.memoryForm.addEventListener("submit", searchMemory);
els.addMemoryForm.addEventListener("submit", addMemory);
els.refreshMemoryButton.addEventListener("click", refreshMemorySearch);
els.reviewInboxButton.addEventListener("click", reviewMemoryInbox);
els.exportMemoryButton.addEventListener("click", exportMemory);
els.metabolizeMemoryButton.addEventListener("click", metabolizeMemory);
els.memoryResult.addEventListener("click", (event) => {
  const copyButton = event.target.closest("[data-memory-copy-id]");
  if (copyButton) {
    copyMemoryId(copyButton.dataset.memoryCopyId, copyButton);
    return;
  }

  const auditButton = event.target.closest("[data-memory-audit-id]");
  if (auditButton) {
    auditMemory(auditButton.dataset.memoryAuditId, auditButton);
    return;
  }

  const jsonButton = event.target.closest("[data-memory-json]");
  if (jsonButton) {
    showMemoryJson(jsonButton.dataset.memoryJson);
    return;
  }

  const acceptButton = event.target.closest("[data-memory-accept-id]");
  if (acceptButton) {
    acceptMemory(acceptButton.dataset.memoryAcceptId, acceptButton);
    return;
  }

  const deleteButton = event.target.closest("[data-memory-delete-id]");
  if (deleteButton) {
    deleteMemory(deleteButton.dataset.memoryDeleteId, deleteButton);
  }
});
els.reflectionForm.addEventListener("submit", runReflection);
els.pairingChallengeButton.addEventListener("click", issuePairingChallenge);
els.pairingForm.addEventListener("submit", completePairing);
els.issueSessionButton.addEventListener("click", () => issueAuthSession());
els.clearSessionButton.addEventListener("click", clearConsoleSession);
els.listPairingButton.addEventListener("click", listPairedDevices);
els.passphraseChallengeButton.addEventListener("click", issuePassphraseChallenge);
els.passphraseForm.addEventListener("submit", verifyPassphraseChallenge);
els.loadToolsButton.addEventListener("click", loadToolCatalog);
els.toolCheckButton.addEventListener("click", checkCameraTool);
els.toolStatusButton.addEventListener("click", executeStatusTool);
els.toolSensorButton.addEventListener("click", executeSensorTool);
els.toolCameraOwnerButton.addEventListener("click", executeOwnerCameraTool);
els.toolWriteMemoryButton.addEventListener("click", executeWriteMemoryTool);
els.toolBlockedCameraButton.addEventListener("click", executeBlockedCameraTool);
els.loadOtaButton.addEventListener("click", loadOtaManifest);
els.checkOtaButton.addEventListener("click", checkOtaManifest);
els.toolUpdateButton.addEventListener("click", executeUpdateCheckTool);
els.webhookForm.addEventListener("submit", sendWebhookInput);
els.refreshRunsButton.addEventListener("click", refreshRuns);
els.runsResult.addEventListener("click", (event) => {
  const button = event.target.closest("[data-run-id]");
  if (!button) {
    return;
  }

  fetchRunDetail(button.dataset.runId, button);
});
els.clearLogButton.addEventListener("click", () => {
  els.apiLog.textContent = "";
});
registerServiceWorker();
checkHealth();
renderSessionState();
loadProtectedDashboard();
