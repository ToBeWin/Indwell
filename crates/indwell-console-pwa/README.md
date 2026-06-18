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
- `GET/PUT /v1/providers`
- `POST /v1/providers/test`
- `GET/PUT/DELETE /v1/secrets/:key_ref`
- `GET/POST /v1/provisioning`
- `POST /v1/memory/search`
- `POST /v1/memory`
- `GET /v1/memory/:id/audit`
- `POST /v1/memory/:id/accept`
- `GET /v1/memory/export`
- `POST /v1/memory/metabolize`
- `POST /v1/reflection/run`
- `POST /v1/tools/device.camera.capture/check`
- `POST /v1/tools/:tool/execute`
- `GET /v1/ota/manifest`
- `POST /v1/ota/check`
- `GET /v1/runs`
- `GET /v1/runs/:id`
