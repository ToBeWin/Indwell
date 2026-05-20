# Indwell Security Model

Phase 0 implements the policy shape that Proto v1 needs:

- Tools carry risk levels.
- Channels carry capability policies.
- Medium-risk tools require owner authentication.
- High-risk tools require owner authentication plus a strong factor or confirmation.
- Public chat channels cannot access camera, memory read, system config, or OTA apply by default.
- Provider config stores key references, not raw API keys.
- Pairing challenges create paired device records from a short-lived code.
- Dynamic passphrase challenges can be verified once with normalized spoken text.
- OTA manifest signatures can be verified with Ed25519 public keys.
- Local secrets can be sealed and opened with ChaCha20-Poly1305 primitives.

Host simulator secrets are intentionally local development scaffolding. ESP32-S3 and production builds must replace this with encrypted local storage backed by ESP-IDF NVS or a platform keystore, plus paired-device signatures for privileged operations.

Owner identity is layered:

- P0: paired phone / local console hint plus passphrase.
- P1: dynamic spoken challenge for replay resistance.
- P2: optional voiceprint as a weak auxiliary signal.
- P3: physical button confirmation for high-risk actions.
