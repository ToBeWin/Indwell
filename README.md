# Indwell OS

Indwell OS is an open-source, local-first Agent OS for embodied AI devices.

This repository currently contains the Phase 0 host simulator foundation:

- `indwell-core`: events, state, agent runs, tools.
- `indwell-protocol`: mobile/device/channel protocol messages.
- `indwell-channel`: channel normalization and channel policies.
- `indwell-security`: policy decisions, sealed secrets, pairing, signed requests, sessions, and confirmation grants.
- `indwell-memory`: local JSONL memory store.
- `indwell-provider`: mock and provider-agnostic traits.
- `indwell-ota`: OTA manifest shape and firmware hash verification.
- `indwell-hal`: hardware traits for LED, microphone, speaker, camera, sensors, and storage.
- `indwell-fw-esp32s3`: ESP32-S3 firmware boot-plan and state-to-LED mapping scaffold.
- `indwell-runs`: append-only AgentRun audit log.
- `indwell-host-sim`: desktop simulator HTTP API.
- `indwell-console-pwa`: static local-first control console.

The implementation follows the engineering specification in `AGENTS.md`.

## Run Locally

Run the non-hardware verification gate:

```sh
make verify-nonhardware
```

Run a host simulator HTTP smoke test:

```sh
make smoke-host-sim
```

Start the host simulator:

```sh
cargo run -p indwell-host-sim
```

Serve the PWA console:

```sh
python3 -m http.server 4174 --directory crates/indwell-console-pwa
```

Then open:

```text
http://127.0.0.1:4174
```

The simulator API listens on:

```text
http://127.0.0.1:3030
```

In the PWA, use `Pairing` in this order:

1. `Issue challenge`
2. `Complete signed pairing`
3. `Issue session`

The console generates an Ed25519 keypair in the browser, signs the pairing
payload, exchanges a signed request for a local session token, and stores that
token in localStorage for protected management APIs.

Minimal public API smoke path:

```sh
curl -fsS http://127.0.0.1:3030/health

curl -fsS -X POST http://127.0.0.1:3030/v1/channel/input \
  -H 'content-type: application/json' \
  -d '{"channel":"local_pwa","session_id":"demo","subject_hint":"owner","text":"remember I like quiet mornings"}'

curl -fsS -X POST http://127.0.0.1:3030/v1/voice/mock-turn \
  -H 'content-type: application/json' \
  -d '{"text_hint":"hello indwell","voice":"warm_indwell"}'
```

## Current Phase 0 Surface

- Local memory: append/search/delete/compact/export over JSONL drawers, with PWA add/delete/audit/accept/JSON inspection controls.
- Memory metabolism: TTL expiry, decay, and preference consolidation into reflection records.
- Reflection Engine: derives preference, relationship, emotional, and skill memories from episodic records with source tags.
- Channel gateway: local PWA, LAN/BLE/USB style channels, chat app style channels, MQTT/Home Assistant/custom webhook normalization.
- Policy engine: tool risk levels, owner checks, high-risk confirmation gates, public-channel camera blocking.
- Public ingress guard: unauthenticated channel and mock voice ingress cannot spend user-owned provider keys; unverified input is quarantined in `inbox/unverified` for owner review.
- Agent run audit: trigger, retrieved memories, written memories, exposed tools, policy blocks, provider output summary, and failure reason, including channel and mock voice turns, with checkpoint replay entries for durable execution review.
- Context assembly: persona snapshot, device state, retrieved memories, policy notes, and the current allowed tool subset are rendered into LLM chat requests instead of only being stored for audit.
- Provider config: local JSON config with API key references, rejecting raw API keys in config.
- Provider runtime: mock by default, plus OpenAI-compatible HTTP chat, structured tool calls, vision, ASR, TTS, and embedding paths for host/desktop.
- Provider diagnostics: protected host-sim API and PWA buttons test LLM/Vision/ASR/TTS/Embedding slots independently.
- Local secrets: host simulator seals local API secrets for an API key ref without returning the raw value.
- Session auth: protected host-sim routes require a signed paired-device session token; the PWA can generate a browser Ed25519 keypair, complete signed pairing, and issue a session with persisted nonce replay protection.
- Confirmation grants: high-risk tools require a valid session plus a scoped, single-use passphrase grant; issued and consumed grants are persisted locally so replay attempts remain blocked after host restart.
- Tool runtime: centralized host-sim tool catalog/schema lookup, provider-returned structured tool calls, planner fallback, status, LED, speaker, camera capture with optional Vision Provider analysis, sensor read mock, memory search/write/delete, identity, confirmation, OTA check.
- OTA: local manifest store, HTTPS URL check, SHA-256 format check, trust-store Ed25519 manifest signature verification, confirmed apply UX, and signed apply plans with slot alternation.
- Console PWA: LLM/Vision/ASR/TTS/Embedding provider config, channel input, custom webhook, memory add/search/delete/inbox review/export/JSON inspection, tool catalog/runtime, OTA manifest/check/apply, run audit, raw API log.
- Firmware scaffold: ESP32-S3 boot plan, HAL boundary, partition table, and sdkconfig defaults.
- ESP-IDF driver map: Proto v1 peripheral bindings to HAL traits.
- Protocol provisioning schema: Wi-Fi password references and provider config without raw secrets.

## Provider Secrets

Provider config stores `api_key_ref`, not raw API keys. In the host simulator, either store a temporary secret through the PWA or set an environment variable derived from the ref:

```sh
export INDWELL_SECRET_KEY_LLM_MAIN="..."
```

Then set provider kind to `openai_compatible`, base URL to a compatible `/v1` endpoint, and model to the target model name. Vision, ASR, TTS, and Embedding providers can be disabled, set to `mock`, inherit LLM connection details with `same_as_llm`, or use their own OpenAI-compatible connection. After pairing the PWA, use the provider test buttons or `POST /v1/providers/test` to validate each provider slot without running a full Agent turn.

## Protected API Auth

Most management APIs require a session token:

```text
Authorization: Bearer <session-token>
```

or:

```text
x-indwell-session-token: <session-token>
```

The host simulator can issue a session after a paired device signs an
`indwell-request-v1` payload. High-risk tools such as OTA apply additionally
need a passphrase-derived confirmation grant bound to the exact tool name.

Unauthenticated public ingress remains available for local smoke tests, but it
is deliberately constrained: if a real provider is configured, public channel
input and mock voice turns are forced to mock providers, and unauthenticated
channel memories are written to `inbox/unverified` for later review.

## Verification

```sh
make verify-nonhardware
make smoke-host-sim
cargo fmt --all --check
cargo test
node --check crates/indwell-console-pwa/app.js
node --check crates/indwell-console-pwa/sw.js
```
