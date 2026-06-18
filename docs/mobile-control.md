# Mobile Control

Proto v1 mobile control is local-first:

- BLE / USB / temporary AP for provisioning.
- Local PWA served by the device.
- LAN HTTP/WebSocket for status, text input, memory, provider config, tool checks, and OTA confirmation.
- Third-party or user-owned gateways for internet remote access.

Implemented in host simulator:

- `POST /v1/channel/input`
- `POST /v1/gateway/custom-webhook`
- paired-device session auth for protected local control APIs
- dynamic passphrase confirmation grants for high-risk actions
- channel policy defaults for local, LAN, chat apps, MQTT, Home Assistant, and custom webhooks
- mobile command normalization for command-style events
- run audit for every channel input and mock voice turn
- context-aware LLM requests that include compact persona/device/memory/policy context
- owner-authenticated camera capture can request Vision Provider analysis
- OTA Apply checks for a scoped `system.update.apply` confirmation grant before executing
- public ingress quarantine for unauthenticated channel memory
- mock-provider fallback for unauthenticated ingress when real providers are configured

No project-owned relay is assumed or required.

The local PWA automatically attaches a stored `indwell.console.sessionToken`
as a bearer token. A production mobile app should keep the paired-device
private key in the OS keystore and sign session requests instead of storing
long-lived raw credentials in application state.

The host-sim console can create that token directly:

1. Click `Issue challenge`.
2. Click `Complete signed pairing`.
3. Click `Issue session` if a token was not issued automatically.

The browser signs both the pairing proof and the session request with an
Ed25519 keypair. If the browser does not support WebCrypto Ed25519 in the
current context, use a browser with Ed25519 support on `http://127.0.0.1` or
`http://localhost`.
