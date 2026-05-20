# Provider API

Indwell uses provider traits instead of a central model proxy.

Implemented for host/desktop:

- `MockLlmProvider`
- `OpenAiCompatibleProvider`
- Request body serialization for `/chat/completions`
- Response parsing from OpenAI-compatible `choices[].message.content`
- API errors and response parse errors
- Missing base URL and missing API key checks

Provider config uses `api_key_ref`. In host simulator, the ref can be resolved from:

- PWA secret storage endpoint for the current process
- environment variable, for example `INDWELL_SECRET_KEY_LLM_MAIN`

Firmware should call providers directly through ESP-IDF HTTP/TLS clients or delegate heavier speech/vision providers to the paired phone/local gateway, without any project-owned proxy.
