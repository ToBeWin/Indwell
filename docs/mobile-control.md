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
- run audit for every channel input

No project-owned relay is assumed or required.

The local PWA automatically attaches a stored `indwell.console.sessionToken`
as a bearer token. A production mobile app should keep the paired-device
private key in the OS keystore and sign session requests instead of storing
long-lived raw credentials in application state.
