# Mobile Control

Proto v1 mobile control is local-first:

- BLE / USB / temporary AP for provisioning.
- Local PWA served by the device.
- LAN HTTP/WebSocket for status, text input, memory, provider config, tool checks, and OTA confirmation.
- Third-party or user-owned gateways for internet remote access.

Implemented in host simulator:

- `POST /v1/channel/input`
- `POST /v1/gateway/custom-webhook`
- channel policy defaults for local, LAN, chat apps, MQTT, Home Assistant, and custom webhooks
- mobile command normalization for command-style events
- run audit for every channel input

No project-owned relay is assumed or required.
