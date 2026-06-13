# Provider API

Indwell uses provider traits instead of a central model proxy.

Implemented for host/desktop:

- `MockLlmProvider`
- `OpenAiCompatibleProvider`
- Request body serialization for `/chat/completions`
- Response parsing from OpenAI-compatible `choices[].message.content`
- OpenAI-compatible `tools` serialization from Indwell `ToolSpec`
- OpenAI-compatible `choices[].message.tool_calls` parsing into provider-neutral `ToolCall`
- API errors and response parse errors
- Missing base URL and missing API key checks

Host-sim Agent runs now send the current allowed tool subset to the LLM
provider. If the provider returns structured tool calls, host-sim executes
those calls through the local tool runtime and records the result in the
AgentRun audit trail. The older keyword planner remains as a mock fallback when
the provider returns plain text only.

Provider config uses `api_key_ref`. In host simulator, the ref can be resolved from:

- PWA secret storage endpoint for the current process
- environment variable, for example `INDWELL_SECRET_KEY_LLM_MAIN`

The PWA can configure LLM, Vision, ASR, TTS, and Embedding providers. Optional
providers may be disabled, set to `mock`, set to `same_as_llm` to inherit the
LLM base URL and key ref, or configured as independent `openai_compatible`
providers.

Firmware should call providers directly through ESP-IDF HTTP/TLS clients or delegate heavier speech/vision providers to the paired phone/local gateway, without any project-owned proxy.
