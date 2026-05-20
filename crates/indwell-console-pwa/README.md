# Indwell Console PWA

Dependency-free static console for the Phase 0 host simulator.

## Open

Serve this directory with any static file server:

```sh
python3 -m http.server 4173 --directory crates/indwell-console-pwa
```

Then open:

```text
http://127.0.0.1:4173
```

The console defaults to the host simulator at `http://127.0.0.1:3030`.

## Simulator endpoints

- `GET /health`
- `POST /v1/channel/input`
- `POST /v1/memory/search`
- `GET /v1/memory/export`
- `POST /v1/tools/device.camera.capture/check`
- `GET /v1/runs`
- `GET /v1/runs/:id`
