# Indwell Security Model

Phase 0 implements the policy shape that Proto v1 needs:

- Tools carry risk levels.
- Channels carry capability policies.
- Medium-risk tools require owner authentication.
- High-risk tools require owner authentication plus a strong factor or confirmation.
- Public chat channels cannot access camera, memory read, system config, or OTA apply by default.
- Provider config stores key references, not raw API keys.
- Pairing challenges create paired device records from a short-lived code plus optional Ed25519 proof.
- Signed requests verify a paired device over method, path, timestamp, nonce, and body hash.
- Session tokens are signed locally and required for protected host-sim routes.
- Dynamic passphrase challenges can be verified once and converted into scoped confirmation grants.
- Confirmation grants are subject-bound, tool-bound, expiring, and single-use.
- OTA manifest signatures can be verified with Ed25519 public keys.
- Local secrets can be sealed and opened with ChaCha20-Poly1305 primitives.

Host simulator secrets are sealed at rest for local development. ESP32-S3 and production builds must replace this with encrypted local storage backed by ESP-IDF NVS or a platform keystore.

Owner identity is layered:

- P0: paired phone or local console with signed session token.
- P1: dynamic spoken challenge that issues a scoped confirmation grant.
- P2: physical button confirmation for high-risk actions.
- P3: optional voiceprint as a weak auxiliary signal.

## Host Simulator Auth Flow

Public bootstrap routes:

- `GET /health`
- `POST /v1/pairing/challenge`
- `POST /v1/pairing/complete`
- `POST /v1/auth/session`
- `POST /v1/auth/passphrase/challenge`
- `POST /v1/auth/passphrase/verify`
- channel input and webhook routes for low-trust ingress testing

Protected routes require either:

```text
Authorization: Bearer <session-token>
```

or:

```text
x-indwell-session-token: <session-token>
```

Protected routes include memory, provider config, secrets, provisioning, paired device listing/revocation, OTA, run audit, and tool runtime endpoints.

To issue a session token, the paired client signs:

```text
indwell-request-v1
device_id=<paired-device-id>
timestamp_ms=<unix-ms>
nonce=<random-nonce>
method=<HTTP method>
path=<request path>
body_sha256=<lowercase hex sha256>
```

The host simulator verifies that signature against the paired device public key before returning a session token.

High-risk tool execution requires:

1. a valid paired-device session token
2. a successful passphrase verification
3. a matching, unexpired, unconsumed confirmation grant for the exact tool name
