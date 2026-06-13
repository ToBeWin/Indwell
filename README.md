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

## Current Phase 0 Surface

- Local memory: append/search/delete/compact/export over JSONL drawers.
- Memory metabolism: TTL expiry, decay, and preference consolidation into reflection records.
- Reflection Engine: derives preference, relationship, emotional, and skill memories from episodic records with source tags.
- Channel gateway: local PWA, LAN/BLE/USB style channels, chat app style channels, MQTT/Home Assistant/custom webhook normalization.
- Policy engine: tool risk levels, owner checks, high-risk confirmation gates, public-channel camera blocking.
- Agent run audit: trigger, retrieved memories, written memories, exposed tools, policy blocks, provider output summary.
- Provider config: local JSON config with API key references, rejecting raw API keys in config.
- Provider runtime: mock by default, plus OpenAI-compatible HTTP chat, vision, ASR, TTS, and embedding paths for host/desktop.
- Local secrets: host simulator seals local API secrets for an API key ref without returning the raw value.
- Session auth: protected host-sim routes require a signed paired-device session token.
- Confirmation grants: high-risk tools require a valid session plus a scoped, single-use passphrase grant.
- Tool runtime: status, LED, speaker, camera capture mock, sensor read mock, memory search/write/delete, identity, confirmation, OTA check.
- OTA: local manifest store, shape checks, HTTPS URL check, SHA-256 format check, Ed25519 manifest signature verification, apply plan, slot alternation.
- Console PWA: provider config, channel input, custom webhook, memory search/export, tool catalog/runtime, OTA manifest/check, run audit, raw API log.
- Firmware scaffold: ESP32-S3 boot plan, HAL boundary, partition table, and sdkconfig defaults.
- ESP-IDF driver map: Proto v1 peripheral bindings to HAL traits.
- Protocol provisioning schema: Wi-Fi password references and provider config without raw secrets.

## Provider Secrets

Provider config stores `api_key_ref`, not raw API keys. In the host simulator, either store a temporary secret through the PWA or set an environment variable derived from the ref:

```sh
export INDWELL_SECRET_KEY_LLM_MAIN="..."
```

Then set provider kind to `openai_compatible`, base URL to a compatible `/v1` endpoint, and model to the target model name.

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

## Verification

```sh
cargo fmt --all --check
cargo test
node --check crates/indwell-console-pwa/app.js
node --check crates/indwell-console-pwa/sw.js
```
