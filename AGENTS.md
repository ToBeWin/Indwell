# AGENTS.md — Indwell OS 工程规格

> 项目代号：**Indwell OS**  
> 版本：v0.1  
> 日期：2026-05-18  
> 目标读者：Codex / Rust 工程代理 / 固件工程师 / Agent 架构师  
> 语言要求：核心固件、Agent Runtime、协议、状态机、安全层、Provider 抽象层均优先使用 **Rust** 编写。必要时允许通过 FFI 调用 ESP-IDF / ESP-SR / esp32-camera 等底层 C 组件，但业务逻辑必须在 Rust 层表达。

---

## 0. 一句话定义

Indwell OS 是一个运行在极致低成本硬件上的、**本地优先、无自有服务器、用户自带模型 API Key、具备长期记忆和现实世界感知能力的端侧 AI Agent OS**。

它不是普通聊天机器人，也不是单纯智能音箱；它是一个可以被部署到廉价机器人、AI 玩偶、AI 台灯、AI 音箱、桌面摆件、老人陪伴设备、儿童学习设备上的 **Personal AI Runtime**。

核心范式：

```text
低成本端侧硬件
    = 感知 + 事件 + 本地身份 + 本地记忆 + 本地安全 + 本地控制

用户自选云端大模型 API
    = 推理 + 视觉理解 + ASR/TTS 可选 + 深度规划 + 反思

Indwell OS
    = 端侧 Agent Kernel + Memory OS + Emotion Engine + Tool System + Channel Layer + Provider Layer
```

---

## 0.1 命名与品牌体系

本规格统一采用 **Indwell OS** 作为项目名称。

命名原则：

- 使用纯英文名称，不使用拼音。
- `Indwell` 表达“AI 栖居在设备内部”的概念，适合本地优先、具身设备、长期记忆和人格化 Agent。
- 本项目是开源端侧 Agent OS，不是单一机器人硬件品牌；硬件只是 Indwell OS 的载体。
- 正式商用前仍需完成商标、域名、GitHub organization、crates.io、npm、App Store、CNIPA、USPTO、EUIPO、WIPO 等完整检索。

推荐命名体系：

```text
Indwell OS          # 整个端侧 AI Agent OS
Indwell Core        # Rust Agent Runtime / Kernel
Indwell Memory      # 本地长期记忆系统
Indwell Console     # 本地 PWA / 手机控制台
Indwell Firmware    # 烧录到 ESP32-S3 等硬件的固件
Indwell Proto v1    # 第一代极低成本原型硬件
indwell-os          # GitHub 仓库名
indwell.local       # 局域网 mDNS / PWA 入口
```

对外一句话定义：

```text
Indwell OS is an open-source, local-first Agent OS for embodied AI devices.
```

中文解释：

```text
Indwell OS 是一个让 AI 住进现实设备里的本地优先 Agent OS。
```

## 1. 项目硬约束

### 1.1 不允许依赖自有服务器

本项目默认 **没有任何中心化后端**。

不得设计以下必需依赖：

- 自建用户账户系统。
- 自建云端记忆服务器。
- 自建推理代理服务器。
- 自建 OTA 服务器。
- 自建消息中继服务器。
- 自建遥控中继服务器。

允许使用：

- 用户自己的大模型 API Key。
- GitHub Releases 作为开源固件发布渠道。
- 用户自己配置的第三方通道，例如 Telegram Bot、Discord Bot、MQTT Broker、Home Assistant、Tailscale、WireGuard、Cloudflare Tunnel 等。
- 设备本地 HTTP/WebSocket 服务。
- BLE / USB / Wi-Fi 局域网连接。

### 1.2 用户自带模型

必须支持用户自由选择模型提供商。不能绑定单一大模型厂商。

至少设计以下 Provider 抽象：

- LLM Provider：文本推理。
- Vision Provider：图像理解。
- ASR Provider：语音转文字。
- TTS Provider：文字转语音。
- Embedding Provider：记忆检索向量，可选。

Provider 可以来自同一厂商，也可以混搭。例如：

```json
{
  "llm": "openai:gpt-4o-mini",
  "vision": "openai:gpt-4o",
  "asr": "local-or-provider-whisper",
  "tts": "provider-tts-or-local",
  "embedding": "disabled-or-provider"
}
```

### 1.3 本地优先

默认情况下：

- API Key 存本地。
- 长期记忆存本地。
- 人格配置存本地。
- 设备配置存本地。
- 口令和主人身份配置存本地。
- 不上传原始音频、图片、聊天历史，除非当前任务需要调用用户选择的模型 API。

### 1.4 极致廉价硬件

首个原型必须以 **ESP32-S3 + microSD** 作为主要目标硬件层级。

推荐 Proto v1 硬件：

| 模块 | 推荐 | 说明 |
|---|---|---|
| MCU | ESP32-S3，优先 N16R8 / N8R8 | Wi-Fi、BLE、PSRAM、Rust/ESP-IDF 支持 |
| 摄像头 | OV2640 | 只做按需拍照，不做持续视频理解 |
| 麦克风 | INMP441 I2S Mic | 语音输入、VAD、唤醒词 |
| 扬声器 | 8Ω 小喇叭 | TTS 播放 |
| 功放 | MAX98357A I2S Amp | 数字音频输出 |
| 存储 | microSD / TF 卡模块 | 长期记忆、日志、音频/图片临时缓存 |
| 状态反馈 | WS2812 RGB LED | listening / thinking / speaking / error / sleep |
| 输入 | 物理按钮 | 配对、重置、手动唤醒、安全确认 |
| 供电 | USB Type-C | 原型阶段不考虑电池和外壳 |

> 不推荐首版用 ESP32-CAM 做完整语音+视觉+存储 Runtime。它便宜，但音频、摄像头、PSRAM、SD 卡、TLS、JSON、播放链路叠加后很容易不稳定。ESP32-CAM 可以作为极限低配实验目标，不作为主线目标。

---

## 2. 借鉴对象与设计原则

本项目需要借鉴现有优秀 Agent，但不能照搬。我们要提取它们背后的架构思想，再针对“廉价端侧实体 AI”重构。

### 2.1 Claude / Claude Code / MCP 系列

借鉴点：

- Tool Use 以结构化方式返回，由应用侧执行。
- 工具必须有清晰 schema、权限边界和输出约束。
- MCP 的思想适合抽象外部工具，但端侧设备不能盲目暴露大量工具。
- 工具定义不能全部塞进上下文；应该按需发现、按需加载。
- 工具返回必须 token-efficient，避免返回大量无意义字段。

落地到 Indwell OS：

- 建立 `ToolRegistry`，但每次只给模型暴露当前任务相关的工具。
- 工具必须有 namespace，例如 `device.camera.capture`、`device.speaker.speak`、`memory.search`。
- 高风险工具必须经过 `PolicyEngine` 与用户确认。
- 工具执行永远在本地 Runtime 内完成；模型只提出结构化 tool call。

### 2.2 OpenClaw / OpenClaw 衍生生态

> 说明：本文统一以 **OpenClaw** 作为核心借鉴对象，重点借鉴其 Personal Agent、用户自有设备运行、多渠道接入、Gateway / Control Plane、持续存在型 Agent 与 Skills / Persona 等 Runtime 思想。

借鉴点：

- Personal AI Assistant 运行在用户自己的设备上。
- 多渠道接入：CLI、聊天渠道、移动端、桌面端。
- Gateway / Control Plane 思想：网关只是控制面，真正产品是 assistant。
- 持续存在型 Agent，而不是一次性 Chat。
- Skills / SOUL / Persona 等人格和技能表达。

落地到 Indwell OS：

- 设备本身就是 “Agent Terminal”。
- 手机 App / Web UI / 语音 / 按钮 / 传感器都只是通道。
- 不在首版追求复杂多 Agent，而是先实现单一 Companion Agent + 本地 Tool Bus。
- 技能系统必须极简、可审计、可禁用，不能允许任意脚本默认执行。

### 2.3 Hermes Agent

借鉴点：

- 自我成长型 Agent。
- 从任务经验中生成 skill。
- 跨会话记忆。
- 反思和长期用户模型。
- 任意模型 Provider，避免供应商锁定。

落地到 Indwell OS：

- 实现 `ReflectionEngine`：从互动中提炼偏好、习惯、情绪规律和有效策略。
- 实现 `SkillMemory`：把用户常用任务压缩成可复用模板。
- 反思必须受成本预算和安全策略约束，不允许每次对话都调用昂贵模型。
- 反思写入长期记忆前应有置信度、敏感级别和可删除机制。

### 2.4 MemPalace / Memory Palace

借鉴点：

- Local-first memory。
- Verbatim storage：原始内容优先，不完全依赖模型总结。
- Palace 结构：wing / room / drawer，让记忆不是一坨扁平向量库。
- 可插拔后端。
- 用户数据默认不离开本机。

落地到 Indwell OS：

- 建立 `Memory Palace for Reality Agent`。
- 不是单纯保存聊天记录，而是保存关系、身份、情绪、生活习惯、设备事件。
- 原始记录必须有配额和隐私策略，不能无限保存音频/图片。
- 对 ESP32-S3，首版默认用 **append-only JSONL + compacted snapshots**；SQLite / sqlite-vec 作为 Linux SBC、桌面模拟器、手机端或后续硬件的增强后端。

### 2.5 OpenAI Agents SDK / LangGraph / smolagents / AutoGen / OpenHands / Open Interpreter

借鉴点：

- OpenAI Agents SDK：handoff、guardrails、state、observability。
- LangGraph：durable execution、checkpoint、human-in-the-loop。
- smolagents：轻量、少抽象、模型无关、工具无关。
- AutoGen：事件驱动、多 Agent 通信，但不照搬复杂度。
- OpenHands / Open Interpreter：本地工具执行、终端控制、沙箱的重要性。

落地到 Indwell OS：

- 不在首版做复杂多 Agent。
- 必须有 durable state：任何一次 Agent Run 都要能记录输入、决策、工具调用、结果、失败原因。
- 高风险操作必须支持 human-in-the-loop：手机确认、物理按钮确认、口令确认。
- 工具执行必须可回放、可审计。
- 不允许 Agent 任意执行 shell、下载脚本或运行未知代码。

### 2.6 ESPHome / Home Assistant / openWakeWord / sherpa-onnx

借鉴点：

- ESPHome：廉价 ESP 设备可以做语音助手端点，但音频组件会显著消耗 RAM/CPU，因此首版必须克制。
- Home Assistant：local-first、用户自管、设备发现、配置化、OTA、开源社区治理。
- openWakeWord：开放唤醒词思路。
- sherpa-onnx：本地 ASR、TTS、VAD、speaker verification 在更强端侧设备、手机、Linux SBC 上可行。

落地到 Indwell OS：

- ESP32-S3 首版主打：wake/VAD/短音频采集/播放/图片采集/本地事件。
- 更重的 ASR/TTS/speaker verification 可以放到：
  - 用户手机本地。
  - Linux SBC 高配版本。
  - 用户选择的大模型 API。
- ESP32-S3 上的声纹识别不应宣称为银行级安全。它只能作为辅助信号，必须叠加口令、手机配对或物理确认。

---

## 3. 产品目标

### 3.1 最小可行目标：Proto v1

Proto v1 必须实现：

1. 设备可刷入 **Indwell Firmware**。
2. 本地 PWA / 手机可以完成配网、API Key 配置和模型选择。
3. 设备可以通过唤醒词或按钮进入 listening 状态。
4. 设备可以采集短音频，进行 ASR，调用 LLM，播放 TTS。
5. 设备可以按需拍照，把 JPEG 发给 Vision Provider。
6. 设备可以把对话、事件、用户偏好写入本地 microSD 记忆。
7. 本地 PWA / 已授权 Channel 可以查看、编辑、删除记忆。
8. 系统支持主人识别：至少支持口令；声纹作为可选增强。
9. 系统支持本地与远程 Channel 控制：同 LAN WebSocket + BLE 配网；互联网远程通过用户自选中继或第三方聊天工具。
10. 系统支持 GitHub Releases 检查更新，但必须用户确认，不能强制自动更新。

### 3.2 非目标：Proto v1 不做

首版不做：

- 人形机器人。
- 多自由度机械结构。
- 本地大模型推理。
- 持续视频理解。
- 自建云服务。
- 自动保存所有原始音频和图片。
- 任意插件运行。
- 任意 shell 执行。
- 无用户确认的高风险动作。
- 无中继情况下的公网穿透承诺。

---

## 4. 系统总体架构

```text
┌────────────────────────────────────────────────────────────┐
│                 Channel / Web / Mobile Control             │
│ PWA | BLE | USB | API Key | 记忆管理 | 远程控制 | OTA确认      │
└──────────────────────────────┬─────────────────────────────┘
                               │ BLE / HTTP / WebSocket / USB
                               ▼
┌────────────────────────────────────────────────────────────┐
│                    Indwell OS Device                       │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Agent Kernel                                         │  │
│  │ Event Bus | State Machine | Tool Bus | Policy Engine │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Perception Layer                                     │  │
│  │ Wake Word | VAD | Mic | Camera | Sensors | Voice ID  │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Memory OS                                            │  │
│  │ Palace | JSONL Log | Snapshots | Index | Retention   │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Provider Layer                                       │  │
│  │ LLM | Vision | ASR | TTS | Embedding                 │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Security Layer                                       │  │
│  │ Pairing | Key Store | Permissions | OTA Signature    │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Hardware HAL                                         │  │
│  │ LED | Button | I2S Mic | I2S Speaker | Camera | SD   │  │
│  └──────────────────────────────────────────────────────┘  │
└──────────────────────────────┬─────────────────────────────┘
                               │ Direct HTTPS/WebSocket
                               ▼
┌────────────────────────────────────────────────────────────┐
│              User-selected Model / Speech Providers         │
│     OpenAI | Anthropic | Gemini | Qwen | DeepSeek | etc.    │
└────────────────────────────────────────────────────────────┘
```

---

## 5. Rust 技术栈建议

### 5.1 仓库结构

```text
indwell-os/
  AGENTS.md
  Cargo.toml
  crates/
    indwell-core/              # no_std-friendly core types, events, state machine
    indwell-memory/            # Memory Palace, JSONL backend, snapshot backend, optional SQLite backend
    indwell-provider/          # LLM/ASR/TTS/Vision provider traits and adapters
    indwell-security/          # pairing, auth, policy, crypto helpers
    indwell-protocol/          # phone/device/channel protocol schemas
    indwell-channel/           # PWA/BLE/USB/Telegram/Feishu/Dingtalk/WeCom/MQTT channel adapters
    indwell-gateway/           # optional user-hosted multi-channel gateway
    indwell-ota/               # GitHub release manifest, signature verification, rollback state
    indwell-hal/               # hardware traits independent of ESP-IDF
    indwell-fw-esp32s3/    # ESP32-S3 firmware target
    indwell-host-sim/      # desktop simulator, runs with tokio, easier testing
    indwell-console-pwa/           # static Web UI served by device or dev server
  firmware/
    partitions.csv
    sdkconfig.defaults
  docs/
    hardware.md
    security.md
    memory.md
    provider-api.md
    channel-layer.md
```

### 5.2 Embedded Rust 路线

优先采用：

- `esp-idf-sys`
- `esp-idf-hal`
- `esp-idf-svc`
- `embedded-svc`
- FreeRTOS task / channel 模型
- 必要时 FFI 调用 ESP-IDF C 组件

不建议首版强行全 `no_std`，因为需要 Wi-Fi、TLS、HTTP、WebSocket、microSD、camera、audio，ESP-IDF `std` 路线更现实。

### 5.3 Host Simulator 路线

`indwell-host-sim` 用于 Codex 快速开发和测试：

- Rust + `tokio`
- `axum` 本地控制 API
- 本地文件系统模拟 microSD
- mock camera / mic / speaker
- provider mock
- memory integration tests

所有复杂业务先在 host simulator 跑通，再移植到 ESP32-S3 firmware。

### 5.4 推荐 crates

视硬件兼容情况选择：

```text
serde / serde_json / postcard
heapless
thiserror
log / esp-idf-svc logging
sha2
hmac
chacha20poly1305 or aes-gcm where supported
ed25519-dalek or ed25519-compact for manifest signature verification
reqwest only for host simulator
esp-idf-svc HTTP client for firmware
embedded-svc
anyhow only for host-side tools, not core firmware
```

核心原则：`indwell-core`、`indwell-protocol`、`indwell-memory` 的关键类型尽量避免绑定具体平台。

---

## 6. Agent Kernel 设计

### 6.1 核心状态机

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    Booting,
    Provisioning,
    Idle,
    Listening,
    Authenticating,
    Thinking,
    Speaking,
    Observing,
    Updating,
    Error,
    Sleep,
}
```

状态含义：

- `Idle`：低功耗待命，等待唤醒词、按钮、手机指令、传感器事件。
- `Listening`：正在采集语音。
- `Authenticating`：进行口令、声纹、手机配对验证。
- `Thinking`：正在调用模型或本地规划。
- `Speaking`：正在播放 TTS。
- `Observing`：正在拍照或读取传感器。
- `Updating`：OTA 更新中。

### 6.2 事件模型

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    BootCompleted,
    ButtonPressed { duration_ms: u32 },
    WakeWordDetected { score: f32 },
    VadSpeechStarted,
    VadSpeechEnded,
    AudioCaptured { path: String, duration_ms: u32 },
    ImageCaptured { path: String, width: u16, height: u16 },
    MobileCommand { session_id: String, command: MobileCommand },
    SensorChanged { sensor: String, value: SensorValue },
    AuthPassed { subject_id: String, method: AuthMethod },
    AuthFailed { reason: String },
    ProviderResponse { run_id: String },
    ToolCallRequested { run_id: String, tool: String },
    ToolCallCompleted { run_id: String, tool: String },
    MemoryWriteRequested { record_id: String },
    OtaUpdateAvailable { version: String },
    Error { code: String, message: String },
}
```

设计原则：

- Indwell OS 是事件驱动系统，不是无限 LLM loop。
- 绝大多数事件不应该触发大模型。
- 只有高价值事件才进入 `CognitionPipeline`。
- 所有事件都应可记录、可审计、可回放。

### 6.3 Agent Run

每一次模型调用称为一个 `AgentRun`。

```rust
pub struct AgentRun {
    pub id: String,
    pub trigger: Event,
    pub user_intent: Option<String>,
    pub auth_context: AuthContext,
    pub context_pack: ContextPack,
    pub allowed_tools: Vec<ToolDescriptor>,
    pub provider: ProviderSelection,
    pub status: RunStatus,
    pub created_at_ms: u64,
}
```

每个 `AgentRun` 必须保存：

- 触发原因。
- 输入摘要。
- 使用了哪些记忆。
- 暴露给模型的工具列表。
- 模型输出。
- 工具调用结果。
- 写入了哪些记忆。
- 是否触发安全拦截。

---

## 7. Tool System

### 7.1 工具设计原则

工具必须：

- 命名清晰。
- 带 namespace。
- 输入输出 schema 固定。
- 返回高信号、低 token 内容。
- 有风险等级。
- 有权限要求。
- 可被禁用。
- 可审计。

### 7.2 工具风险等级

```rust
pub enum RiskLevel {
    Safe,       // LED、状态查询、普通记忆检索
    Low,        // 播放声音、拍照、保存非敏感记忆
    Medium,     // 修改人格、删除记忆、远程查看摄像头
    High,       // 开门、联网发送消息、执行更新、暴露 API Key 相关操作
    Forbidden,  // shell、任意脚本、未签名插件、未知二进制
}
```

规则：

- `Safe` 可自动执行。
- `Low` 默认可执行，但需要审计。
- `Medium` 需要主人身份通过。
- `High` 需要主人身份 + 口令或手机确认/物理按钮确认。
- `Forbidden` 永远不执行。

### 7.3 首版工具列表

```text
device.led.set
  设置状态灯。

device.speaker.speak
  播放 TTS 或预设音效。

device.camera.capture
  拍一张照片并返回本地路径或 provider 可上传句柄。

device.sensor.read
  读取温度、光线、IMU、压力等传感器。

memory.search
  按 room/kind/query 搜索本地记忆。

memory.write_candidate
  写入待确认/待合并记忆。

memory.delete
  删除或归档记忆。Medium risk。

identity.whoami
  返回当前身份识别结果。

auth.request_confirmation
  请求手机或物理按钮确认。

system.status
  返回设备状态、网络、模型配置、电量可选。

system.update.check
  检查 GitHub Releases 是否有新版本。

system.update.apply
  执行 OTA。High risk。
```

---

## 8. Provider Layer

### 8.1 不做统一后端代理

Provider 调用必须从设备或用户手机/本地网关直接发往用户选择的模型 API。不得把请求转发到项目方服务器。

### 8.2 Provider traits

```rust
#[async_trait]
pub trait LlmProvider {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, ProviderError>;
    fn capabilities(&self) -> ProviderCapabilities;
}

#[async_trait]
pub trait VisionProvider {
    async fn analyze_image(&self, req: VisionRequest) -> Result<VisionResponse, ProviderError>;
}

#[async_trait]
pub trait AsrProvider {
    async fn transcribe(&self, audio: AudioBlob) -> Result<Transcript, ProviderError>;
}

#[async_trait]
pub trait TtsProvider {
    async fn synthesize(&self, text: &str, voice: VoiceProfile) -> Result<AudioBlob, ProviderError>;
}

#[async_trait]
pub trait EmbeddingProvider {
    async fn embed(&self, input: &str) -> Result<Vec<f32>, ProviderError>;
}
```

在 ESP32-S3 固件中，如果 `async_trait` 不适合，使用同步接口 + task/channel 适配；traits 结构仍保持一致。

### 8.3 Provider 配置

```json
{
  "providers": {
    "llm": {
      "kind": "openai_compatible",
      "base_url": "https://api.example.com/v1",
      "api_key_ref": "key_llm_main",
      "model": "model-name",
      "max_input_tokens": 4000,
      "max_output_tokens": 600
    },
    "vision": {
      "kind": "same_as_llm",
      "model": "vision-model-name"
    },
    "asr": {
      "kind": "provider_or_local",
      "model": "asr-model-name"
    },
    "tts": {
      "kind": "provider_or_local",
      "voice": "warm_indwell"
    }
  }
}
```

### 8.4 Token 节省策略

必须实现 `ContextAssembler`：

```text
System Contract         300-800 tokens
Persona Snapshot        100-300 tokens
Current Device State     50-150 tokens
Recent Turns            200-800 tokens
Retrieved Memories      200-800 tokens
Relevant Tools          100-500 tokens
User Request             variable
```

禁止：

- 每次发送完整历史。
- 每次发送全部工具定义。
- 每次发送全部人格和长期记忆。
- 每次视觉事件都调用大模型。

必须：

- 对工具按需暴露。
- 对记忆按房间、时间、重要性、敏感级别检索。
- 对视觉按需拍照。
- 对反思设置预算。
- 对失败重试设置上限。

---

## 9. Memory OS：Reality Memory Palace

### 9.1 核心理念

Indwell OS 的记忆不是“聊天记录数据库”，而是“长期关系系统”。

记忆目标：

- 让设备记得用户是谁。
- 让设备记得用户偏好、习惯、情绪模式。
- 让设备记得家庭和环境。
- 让设备记得过往互动。
- 让设备能成长，但用户可审计、可删除、可导出。

### 9.2 Palace 结构

```text
memory_palace/
  wings/
    user_<id>/
      rooms/
        identity/
        preferences/
        relationships/
        emotions/
        episodes/
        reflections/
        learning/
        health_notes/
        safety/
    device_<id>/
      rooms/
        hardware/
        sensors/
        network/
        ota/
    home_<id>/
      rooms/
        locations/
        routines/
        environment/
  drawers/
    YYYY-MM-DD.jsonl
  snapshots/
    persona_snapshot.json
    relationship_snapshot.json
    memory_index.json
```

### 9.3 MemoryRecord schema

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub id: String,
    pub kind: MemoryKind,
    pub wing: String,
    pub room: String,
    pub content: String,
    pub source: MemorySource,
    pub verbatim_ref: Option<String>,
    pub confidence: f32,
    pub importance: f32,
    pub sensitivity: Sensitivity,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub last_used_at_ms: Option<u64>,
    pub ttl_policy: TtlPolicy,
    pub tags: Vec<String>,
    pub hash: String,
}

pub enum MemoryKind {
    Identity,
    Preference,
    Relationship,
    Episodic,
    Emotional,
    Reflection,
    Skill,
    Environment,
    Safety,
}

pub enum Sensitivity {
    Public,
    Personal,
    Private,
    Sensitive,
    Critical,
}
```

### 9.4 存储后端

必须实现 trait：

```rust
pub trait MemoryStore {
    fn append(&mut self, record: MemoryRecord) -> Result<(), MemoryError>;
    fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryRecord>, MemoryError>;
    fn get(&self, id: &str) -> Result<Option<MemoryRecord>, MemoryError>;
    fn delete(&mut self, id: &str) -> Result<(), MemoryError>;
    fn compact(&mut self) -> Result<(), MemoryError>;
    fn export(&self) -> Result<MemoryExport, MemoryError>;
}
```

首版后端：

1. `JsonlMemoryStore`：ESP32-S3 默认。
2. `SnapshotMemoryStore`：定期生成 persona / relationship 快照。
3. `SqliteMemoryStore`：host simulator / Linux SBC / 后续强硬件。
4. `VectorMemoryIndex`：可选，不作为 ESP32-S3 首版必需能力。

### 9.5 记忆写入流程

```text
interaction/event
  ↓
MemoryInbox
  ↓
Local filters: sensitivity, duplicate, quota
  ↓
CandidateMemory
  ↓
PolicyEngine
  ↓
写入 JSONL
  ↓
定期 compact 成 snapshot
```

### 9.6 记忆代谢

必须支持：

- `consolidate`：把多条 episodic memory 压缩成稳定偏好或关系记忆。
- `decay`：低重要性、长期未使用、低置信度记忆逐步降权。
- `forget`：用户删除、过期、敏感信息清除。
- `audit`：手机端查看为什么记住这件事。
- `pin`：用户固定重要记忆。

### 9.7 反思机制

借鉴 Hermes，但针对实体陪伴场景。

反思触发条件：

- 每日一次，用户允许时。
- 重要对话后。
- 情绪连续变化后。
- 学习任务结束后。
- 用户手动要求“记住/总结”。

反思输出：

```json
{
  "new_memories": [],
  "updated_preferences": [],
  "relationship_notes": [],
  "behavior_adjustments": [],
  "skills": [],
  "warnings": []
}
```

限制：

- 不能凭空编造记忆。
- 每条反思必须关联 source event。
- 敏感记忆默认需要用户确认或标记为 private。
- 儿童/老人场景必须启用更严格 retention 和 guardian view。

---

## 10. 语音系统

### 10.1 语音链路

```text
Mic
  ↓
VAD / Wake Word
  ↓
Audio Capture
  ↓
Auth: voiceprint/passphrase/phone proximity
  ↓
ASR
  ↓
Agent Kernel
  ↓
LLM
  ↓
TTS
  ↓
Speaker
```

### 10.2 唤醒词

首版支持三种模式：

1. Button-to-talk：最稳定，开发期必需。
2. Wake word：本地唤醒词，如“Indwell / Hey Indwell”。
3. Mobile trigger：手机端点击唤醒。

ESP32-S3 上可以优先评估：

- ESP-SR / microWakeWord 类方案，Rust 通过 FFI 包装。
- 简单能量 VAD + 云端确认唤醒词作为降级方案，但不能持续上传音频。

### 10.3 声纹识别

必须架构支持，但要分层落地：

| 层级 | 实现 |
|---|---|
| P0 | 口令 + 手机配对，声纹关闭 |
| P1 | 手机端或 Linux host 使用 sherpa-onnx / ONNX speaker verification 做声纹 |
| P2 | ESP32-S3 尝试轻量声纹模型，但只作为辅助信号 |
| P3 | 高配硬件本地完整 speaker verification |

重要原则：

- 声纹不是唯一认证因素。
- 可被录音重放攻击，因此必须支持随机口令挑战。
- 高风险操作必须叠加手机确认或物理按钮确认。

### 10.4 口令校验

支持两类：

1. 静态口令：例如“进入主人模式，暗号是星海”。
2. 动态挑战：设备说“请说今天的确认词：蓝色月亮”，用户复述。

动态挑战可以防录音重放。

---

## 11. 视觉与物理世界感知

### 11.1 视觉策略

首版只做按需拍照。

触发场景：

- 用户问“你看到了什么？”
- 用户要求学习辅导，例如“帮我看看这道题”。
- 情绪/陪伴场景，例如用户主动要求“看看我的桌面”。
- 远程 Channel 要求拍照，必须通过权限。

禁止首版默认持续视频上传。

### 11.2 传感器策略

传感器作为 Tool 接入：

```text
device.sensor.temperature.read
device.sensor.light.read
device.sensor.imu.read
device.sensor.pressure.read
device.sensor.distance.read
```

Agent 不需要知道底层驱动细节，只看到结构化结果。

### 11.3 环境记忆

物理世界相关记忆写入 `home_<id>/rooms/environment`：

- 房间常见温度。
- 用户学习桌位置。
- 老人常用活动时间。
- 设备经常被移动的时间。
- 光照习惯。

---

## 12. Channel Layer 与用户入口

### 12.1 核心原则

Indwell OS 的用户入口不应局限于官方手机 App 或 PWA。

它应该像 OpenClaw / Hermes Agent 一类 Personal Agent 一样，可以栖居在用户已经使用的通信工具中：

- 本地 PWA。
- BLE / USB / LAN WebSocket。
- 飞书。
- 钉钉。
- 企业微信。
- Telegram。
- WhatsApp。
- Discord。
- Matrix。
- MQTT。
- Home Assistant。
- 用户自托管 Webhook / Bot Gateway。

但必须保持核心约束：

- Indwell 项目方不得提供必需中心服务器。
- 第三方聊天工具只是用户自选通道，不是系统必需依赖。
- 复杂平台适配优先运行在用户自托管 `Indwell Gateway`、手机网关、NAS、家用服务器、树莓派或 host simulator 上。
- ESP32-S3 首版不应直接承担所有聊天平台 SDK、Webhook、媒体下载、签名验证和复杂 TLS 适配。
- 所有通道消息必须归一化为内部事件，并经过统一认证、权限和审计。

### 12.2 Channel Layer 架构

```text
User Channels
  PWA / BLE / USB / LAN WebSocket
  飞书 / 钉钉 / 企业微信 / Telegram / WhatsApp / Discord / Matrix
  MQTT / Home Assistant / 用户自托管 Webhook
        ↓
Channel Adapter
        ↓
Indwell Gateway / Device Local Gateway
        ↓
Indwell Agent Kernel
        ↓
Tool Bus / Memory OS / Provider Layer / Policy Engine
```

模块定位：

```text
Indwell OS
  Agent Kernel + Memory OS + Tool Runtime + Policy Engine

Indwell Console
  默认本地 PWA 控制台

Indwell Gateway
  可选的用户自托管多渠道接入层

Channel Adapters
  飞书 / 钉钉 / 企业微信 / Telegram / WhatsApp / MQTT / Home Assistant 等适配器
```

PWA 是 Proto v1 的默认本地控制台；聊天工具和自动化平台是可选 Channel Adapter。

### 12.3 Channel Adapter 抽象

Channel Adapter 的职责是把外部平台消息转换为 Indwell 内部事件，并把 Indwell 输出渲染回对应平台。

```rust
pub trait ChannelAdapter {
    fn channel_kind(&self) -> ChannelKind;
    fn capabilities(&self) -> ChannelCapabilities;
    fn normalize_inbound(&self, input: ChannelInbound) -> Result<IndwellEvent, ChannelError>;
    fn render_outbound(&self, msg: IndwellMessage) -> Result<ChannelOutbound, ChannelError>;
}

pub enum ChannelKind {
    LocalPwa,
    Ble,
    UsbSerial,
    LanWebSocket,
    Telegram,
    Feishu,
    Dingtalk,
    WeCom,
    WhatsApp,
    Discord,
    Matrix,
    Mqtt,
    HomeAssistant,
    CustomWebhook,
}
```

所有入口最终应转为统一内部事件：

```rust
pub enum ChannelEvent {
    UserText {
        channel: ChannelKind,
        session_id: String,
        subject_hint: Option<String>,
        text: String,
    },
    UserImage {
        channel: ChannelKind,
        session_id: String,
        image_ref: MediaRef,
        caption: Option<String>,
    },
    UserAudio {
        channel: ChannelKind,
        session_id: String,
        audio_ref: MediaRef,
    },
    Confirmation {
        channel: ChannelKind,
        session_id: String,
        challenge_id: String,
        approved: bool,
    },
    RemoteCommand {
        channel: ChannelKind,
        session_id: String,
        command: MobileCommand,
    },
}
```

Agent Kernel 不应关心消息来自飞书还是 PWA；它只关心：

- 当前 subject 是否可信。
- 当前 channel 是否已绑定。
- 当前 channel 有哪些能力。
- 当前 channel 是否允许请求相应工具。
- 回复应该发回哪个 channel。

### 12.4 Channel 权限模型

不同通道必须有不同权限，不得把公网聊天工具默认等同于本地配对手机。

```rust
pub struct ChannelPolicy {
    pub channel: ChannelKind,
    pub allow_chat: bool,
    pub allow_memory_read: bool,
    pub allow_memory_write: bool,
    pub allow_camera: bool,
    pub allow_sensor_read: bool,
    pub allow_system_status: bool,
    pub allow_system_config: bool,
    pub allow_ota: bool,
    pub requires_owner_auth_for_medium: bool,
    pub requires_confirmation_for_high: bool,
}
```

推荐默认策略：

| 通道 | 默认权限 |
|---|---|
| PWA + 本地配对手机 | 聊天、记忆管理、模型配置、OTA 确认 |
| BLE / USB | 配网、恢复、开发调试、物理近场确认 |
| LAN WebSocket | 实时状态、聊天、本地控制，敏感操作需主人认证 |
| Telegram / 飞书 / 钉钉 / 企业微信 | 聊天、任务、通知、低风险确认 |
| WhatsApp / Discord / Matrix | 聊天、通知、轻量远程任务 |
| MQTT / Home Assistant | 传感器事件、家庭自动化、状态通知 |
| 未强认证公网通道 | 不允许远程拍照、删除记忆、修改 API Key、OTA |

远程拍照、记忆删除、API Key 修改、OTA、系统配置修改等操作必须叠加主人认证和确认流程。

### 12.5 部署模式

无自有服务器时，必须明确远程能力边界：

| 模式 | 是否需要项目方服务器 | 说明 |
|---|---:|---|
| BLE 配网/配对 | 否 | 首次配置 Wi-Fi、设备密钥 |
| 局域网 Web UI | 否 | 手机访问设备 IP，本地控制 |
| 局域网 WebSocket | 否 | 实时状态、聊天、远程拍照、记忆管理 |
| USB Serial | 否 | 开发、恢复、日志 |
| GitHub OTA | 否 | 检查开源版本 |
| 设备直连第三方 Bot/API | 否 | 可选；例如 Telegram Bot API、MQTT Broker，但受 ESP32-S3 资源限制 |
| 用户自托管 Indwell Gateway | 否 | 用户在 NAS、电脑、树莓派、云主机、Tailscale 网络中运行 |
| 手机作为 Gateway | 否 | 手机接收/转发部分通道消息，再通过 LAN/BLE/USB 连接设备 |
| 互联网远程控制 | 不使用项目方服务器 | 需要用户自配 Tailscale/WireGuard/Cloudflare Tunnel/MQTT/Telegram/飞书/钉钉等 |

不能承诺“无任何中继且任意公网都可远程控制”。NAT 穿透通常需要 STUN/TURN/中继；如果不自建服务器，就必须使用用户自选第三方或用户自建网络。

### 12.6 Proto v1 默认入口

优先实现一个由设备本地服务提供的 PWA：

```text
http://indwell.local/
```

功能：

- 设备状态。
- 模型配置。
- API Key 输入与本地加密存储。
- 语音/文本聊天。
- 拍照测试。
- 记忆查看、编辑、删除。
- 人格配置。
- 口令配置。
- 声纹录入。
- OTA 检查与确认。
- 日志查看。

PWA 的好处：

- 不需要上架 App Store。
- 不需要服务器。
- 手机浏览器即可控制。
- 静态资源可以烧录到固件或存在 microSD。

后续可做 native app，但不作为首版必需。

聊天工具接入不作为 ESP32-S3 Proto v1 的硬性前置要求，但 `indwell-channel` 的内部事件与权限模型应在 Phase 0 就设计好，避免后续重构 Agent Kernel。

### 12.7 配网流程

首次启动：

```text
1. 设备进入 Provisioning。
2. LED 黄灯慢闪。
3. 手机通过 BLE / USB / 临时 AP 连接设备。
4. 用户输入 Wi-Fi。
5. 用户输入或选择模型 Provider。
6. 用户输入 API Key。
7. 用户设置主人口令。
8. 可选录入声纹。
9. 设备生成本地配对密钥。
10. 进入 Idle。
```

### 12.8 日常体验

```text
用户：“Indwell”
设备：LED 蓝色，短音效
用户：“帮我看一下桌子上这本书是什么？”
设备：拍照 -> Vision Provider -> 回答
设备：如有必要写入 episodic memory
```

学习机器人：

```text
用户：“今天我们继续学 Rust。”
设备：检索 learning room，知道上次学到 ownership。
设备：继续上次学习计划。
```

情感陪伴：

```text
用户沉默很久，或者主动说“我有点累”。
设备：低频主动关心，但不得过度打扰。
```

老人陪伴：

```text
传感器异常 / 长时间无互动 / 用户主动呼救
设备：本地提醒 + 如配置了用户自选通知渠道，则发送通知。
```

儿童学习：

```text
默认启用 guardian mode，限制敏感内容、限制记忆保存类型、限制远程拍照。
```

---

## 13. 安全设计

### 13.1 威胁模型

必须考虑：

- 局域网攻击者。
- 蓝牙配对攻击。
- Prompt injection。
- 恶意工具/插件。
- 伪造 OTA。
- API Key 泄露。
- SD 卡被拔走读取。
- 设备被物理获取。
- 声音重放攻击。
- 儿童/老人隐私风险。
- 模型输出诱导危险动作。

### 13.2 API Key 存储

要求：

- API Key 不得明文写入日志。
- API Key 不得暴露给 LLM。
- API Key 不得出现在 Tool output。
- API Key 应加密存储。
- 生产版本启用 ESP32-S3 Secure Boot + Flash Encryption。
- microSD 上敏感数据应加密或至少按敏感级别避免存储。

### 13.3 认证方式

支持多因素：

```text
owner_auth = voiceprint_score + passphrase + paired_phone + physical_button
```

策略：

- 普通聊天：无需强认证，但要知道当前 speaker 是否可信。
- 查看敏感记忆：主人认证。
- 删除记忆：主人认证。
- 远程拍照：主人认证 + 手机确认。
- OTA：主人认证 + 手机确认。
- 修改 API Key：主人认证 + 物理按钮或 USB 本地确认。

### 13.4 Prompt Injection 防御

规则：

- 从音频、视觉、网页、图片、传感器、手机文本来的内容都视为 untrusted input。
- 不允许用户输入覆盖系统安全策略。
- 不允许模型自己修改安全策略。
- 不允许模型请求 API Key。
- 不允许模型把工具权限提升到自己没有的级别。

### 13.5 OTA 安全

OTA 来源：GitHub Releases。

但必须：

- 使用 release manifest。
- 校验 sha256。
- 校验签名。
- 支持 A/B 分区或至少回滚。
- 用户确认后更新。
- 更新前备份 memory index。
- Runtime 与 Memory 分离，更新不清空人格与记忆。

manifest 示例：

```json
{
  "version": "0.1.3",
  "channel": "stable",
  "target": "esp32s3-n16r8",
  "firmware_url": "https://github.com/<org>/<repo>/releases/download/v0.1.3/indwell-fw.bin",
  "sha256": "...",
  "signature": "base64-ed25519-signature",
  "min_bootloader": "0.1.0",
  "memory_schema": "1",
  "notes": "Fix audio playback and add memory audit UI."
}
```

### 13.6 插件/技能安全

首版不支持任意第三方插件执行。

允许：

- 静态技能模板。
- YAML/JSON persona 配置。
- 用户可读的 tool policy。

禁止：

- 下载脚本执行。
- LLM 生成代码后直接执行。
- 插件访问 API Key。
- 插件绕过 PolicyEngine。

---

## 14. OTA 与版本更新

### 14.1 无服务器更新机制

```text
Device / Mobile PWA
  ↓
GitHub Releases latest manifest
  ↓
展示 changelog
  ↓
用户确认
  ↓
下载 firmware
  ↓
校验 sha256 + signature
  ↓
写入 OTA partition
  ↓
重启
  ↓
健康检查
  ↓
确认新版本或回滚
```

### 14.2 更新模式

- `manual`：默认，只提示不安装。
- `security_only`：只自动提示安全更新，仍需确认。
- `auto_download_manual_apply`：自动下载，用户确认安装。
- `pinned`：锁定版本。

不得默认强制自动更新。

### 14.3 版本兼容

必须区分：

- Firmware version。
- Agent runtime version。
- Memory schema version。
- Provider schema version。
- Channel protocol version。

---

## 15. 端侧硬件运行策略

### 15.1 ESP32-S3 资源预算

首版必须克制：

- 不做持续视频。
- 不做本地大模型。
- 不做本地复杂向量搜索。
- 不常驻大型 JSON。
- 不把全部记忆加载进内存。
- 不同时开启太多音频/蓝牙/摄像头组件。

### 15.2 任务分解

推荐 FreeRTOS tasks：

```text
main_task
  负责状态机、事件分发。

audio_task
  负责 VAD/wake/capture/playback。

network_task
  负责 HTTP/WebSocket/provider calls。

storage_task
  负责 microSD log append / snapshot。

channel_task
  负责本地 WebSocket 控制与 ChannelEvent 分发。

sensor_task
  负责传感器采样。
```

用 bounded channel 连接，避免内存暴涨。

### 15.3 文件布局

microSD：

```text
/indwell/
  config/
    device.json
    providers.enc
    persona.json
    policy.json
  memory/
    drawers/
      2026-05-18.jsonl
    snapshots/
      persona_snapshot.json
      relationship_snapshot.json
      index.json
  cache/
    audio/
    image/
  logs/
    runs/
    system/
  ota/
    manifest.json
    pending.bin
```

---

## 16. 情绪与人格系统

### 16.1 Emotion State

```rust
pub enum EmotionState {
    Calm,
    Curious,
    Caring,
    Excited,
    Sleepy,
    Concerned,
    Playful,
}
```

Emotion 影响：

- 回复语气。
- TTS voice profile。
- LED 动画。
- 主动性。
- 回复长度。
- 学习模式严肃程度。

### 16.2 主动性控制

主动性必须有节制。

配置：

```json
{
  "proactivity": {
    "enabled": true,
    "max_interruptions_per_day": 3,
    "quiet_hours": ["22:30", "08:00"],
    "modes": ["care", "learning", "safety"]
  }
}
```

主动事件必须遵守：

- 不在安静时段打扰，除非安全事件。
- 不频繁情绪劝导。
- 不对儿童进行不适当心理判断。
- 不把低置信度观察当作事实。

---

## 17. 运行模式

### 17.1 Companion Mode

情感陪伴，长期关系。

### 17.2 Learning Mode

学习机器人：

- 记住学习目标。
- 生成复习计划。
- 看图识别题目。
- 陪练语言/数学/编程。

### 17.3 Elder Care Mode

老人陪伴：

- 低频问候。
- 传感器异常提醒。
- 用户自选通知渠道。
- 记忆更谨慎。

### 17.4 Child Mode

儿童模式：

- Guardian policy。
- 限制内容。
- 限制记忆保存。
- 限制远程摄像头。
- 默认不开启主动情绪分析。

### 17.5 Developer Mode

开发调试：

- USB logs。
- Provider mock。
- Memory export。
- Tool trace。
- 更详细错误信息。

---

## 18. Codex 实施路线

### Phase 0：Host Simulator

目标：先把 Rust 核心跑通。

交付：

- `indwell-core`：事件、状态机、AgentRun。
- `indwell-memory`：JSONL memory store + snapshot。
- `indwell-provider`：mock provider + openai-compatible skeleton。
- `indwell-security`：policy engine + auth context。
- `indwell-protocol`：mobile/channel command schema。
- `indwell-channel`：ChannelKind、ChannelEvent、ChannelPolicy、LocalPwa adapter skeleton。
- `indwell-gateway`：可选用户自托管 gateway skeleton，先支持 mock/custom webhook。
- `indwell-host-sim`：本地 HTTP/WebSocket 控制。
- PWA 简单页面。

验收：

- 可以在电脑上模拟设备。
- 手机浏览器连接电脑本地服务。
- 文本对话可调用 mock provider。
- 记忆可写入、检索、删除。
- 工具调用经过 policy。
- PWA 输入、mock webhook 输入能归一化为同一种 ChannelEvent。
- 不同 channel 权限策略能阻止未授权高风险工具。

### Phase 1：ESP32-S3 基础固件

交付：

- Wi-Fi 配网。
- microSD 挂载。
- LED 状态。
- 按钮。
- 本地 Web UI / WebSocket。
- API Key 加密存储雏形。
- GitHub manifest 检查。

验收：

- 刷机后手机 / PWA 可配网。
- 设备可进入本地控制台。
- 能写入 memory JSONL。
- 能检查更新但不自动安装。

### Phase 2：语音与模型调用

交付：

- INMP441 音频采集。
- MAX98357A 播放。
- button-to-talk。
- provider ASR。
- provider LLM。
- provider TTS。
- 本地播放回复。

验收：

- 按按钮说话，设备回答。
- 不保存原始音频，除非调试开关开启。
- 对话写入 episodic memory。

### Phase 3：唤醒词、视觉、身份

交付：

- Wake word。
- 口令识别。
- 可选手机端声纹验证。
- OV2640 拍照。
- vision provider。
- 已授权 Channel 的远程拍照权限。

验收：

- 语音唤醒。
- 高风险命令要求口令/手机确认。
- 可问“你看到了什么”。

### Phase 4：Memory Palace + Reflection

交付：

- Palace wing/room/drawer。
- persona snapshot。
- relationship snapshot。
- reflection engine。
- memory audit UI。
- retention policy。

验收：

- 设备能记住用户偏好。
- 用户可查看“为什么记住”。
- 用户可删除记忆。
- context pack 不超过配置 token budget。

### Phase 5：安全强化与 OTA

交付：

- signed manifest。
- sha256 校验。
- rollback。
- secure boot / flash encryption 指南。
- memory schema migration。

验收：

- 伪造 manifest 被拒绝。
- 更新失败可回滚。
- 更新不丢记忆。

---

## 19. 验收标准

### 19.1 核心功能验收

- [ ] 设备可刷机启动。
- [ ] 手机 / PWA 可配网。
- [ ] 用户可配置模型 Provider。
- [ ] API Key 不明文出现在日志。
- [ ] button-to-talk 可用。
- [ ] wake word 可用或有明确实验开关。
- [ ] ASR -> LLM -> TTS 链路可用。
- [ ] 摄像头按需拍照可用。
- [ ] 本地记忆写入 microSD。
- [ ] 手机 / PWA / 已授权 Channel 可管理记忆。
- [ ] Channel Layer 架构可用，至少支持 PWA 与 mock/custom webhook。
- [ ] 口令认证可用。
- [ ] 声纹认证架构可用，至少能通过手机/host 端实现。
- [ ] 高风险工具必须确认。
- [ ] OTA 检查可用。
- [ ] OTA 不强制自动更新。

### 19.2 安全验收

- [ ] LLM 无法读取 API Key。
- [ ] Prompt injection 无法提升工具权限。
- [ ] 未授权公网 channel 无法提升为本地配对手机权限。
- [ ] 未认证用户无法删除记忆。
- [ ] 未认证用户无法远程拍照。
- [ ] 伪造 OTA 无法安装。
- [ ] Memory export 需要主人认证。
- [ ] 儿童模式默认更严格。

### 19.3 成本与端侧验收

- [ ] 主线硬件为 ESP32-S3 + microSD。
- [ ] 不要求 Linux SBC。
- [ ] 不要求本地大模型。
- [ ] 不要求自建服务器。
- [ ] 不要求屏幕、外壳、电池。

---

## 20. 设计禁令

Codex 实现时禁止：

1. 新增任何必须的中心服务器。
2. 把用户 API Key 上传到项目方。
3. 把长期记忆默认上传云端。
4. 默认持续上传音频流。
5. 默认持续上传视频流。
6. 让 LLM 任意执行 shell。
7. 让第三方插件绕过 PolicyEngine。
8. 强制自动更新。
9. 把所有工具定义每次都塞进模型上下文。
10. 把所有聊天历史每次都塞进模型上下文。
11. 宣称 ESP32-S3 声纹识别具备强安全性。
12. 承诺无中继公网远控。
13. 把飞书、钉钉、企业微信、Telegram、WhatsApp 等第三方通道做成系统必需依赖。
14. 让公网聊天通道默认拥有本地配对 PWA / BLE / USB 的权限。

---

## 21. 参考资料与需要借鉴的项目

Codex 可优先阅读这些项目/文档的设计思想：

- OpenClaw：Personal AI Assistant / own devices / multi-channel / assistant as product。  
  https://github.com/openclaw/openclaw

- Hermes Agent：self-improving agent / learning loop / skills / cross-session memory / provider flexibility。  
  https://github.com/nousresearch/hermes-agent

- MemPalace：local-first memory / verbatim storage / palace structure / pluggable backend。  
  https://github.com/mempalace/mempalace

- Claude Tool Use：client-side tools / structured tool calls / tool execution outside model。  
  https://platform.claude.com/docs/en/agents-and-tools/tool-use/overview

- Anthropic tool design：namespacing tools / token-efficient tool responses / on-demand tool discovery。  
  https://www.anthropic.com/engineering/writing-tools-for-agents  
  https://www.anthropic.com/engineering/advanced-tool-use

- OpenAI Agents SDK：agents / handoffs / guardrails / voice agents / state。  
  https://developers.openai.com/api/docs/guides/agents

- LangGraph：durable execution / checkpoint / human-in-the-loop。  
  https://docs.langchain.com/oss/python/langgraph/durable-execution

- smolagents：minimal abstractions / model-agnostic / tool-agnostic / code agents。  
  https://huggingface.co/docs/smolagents/en/index

- ESP Rust：Rust on Espressif。  
  https://docs.espressif.com/projects/rust/

- ESPHome Voice Assistant：ESP voice endpoint resource constraints and voice pipeline patterns。  
  https://esphome.io/components/voice_assistant/

- ESP Web Tools：browser-based ESP flashing and provisioning pattern。  
  https://esphome.github.io/esp-web-tools/

- ESP32-S3 Secure Boot / Flash Encryption：production security baseline。  
  https://docs.espressif.com/projects/esp-idf/en/stable/esp32s3/security/secure-boot-v2.html  
  https://docs.espressif.com/projects/esp-idf/en/stable/esp32s3/security/flash-encryption.html

- sherpa-onnx：local ASR/TTS/VAD/speaker verification for stronger edge/mobile tiers。  
  https://github.com/k2-fsa/sherpa-onnx

- openWakeWord：open wake word concept and tooling, mainly as design reference。  
  https://github.com/dscripka/openWakeWord

---

## 22. 最终工程判断

本项目最重要的不是硬件，而是：

```text
Local-first Memory OS
+ Safe Tool Runtime
+ Provider-agnostic Cognition
+ Channel / Mobile Control
+ Voice/Identity Layer
+ Emotion/Relationship Engine
+ Cheap Embodied Hardware
```

如果首版能在约 50-100 元人民币级别硬件上完成：

- 能听。
- 能说。
- 能看。
- 能记住用户。
- 能通过手机控制。
- 能让用户选择自己的模型。
- 能无服务器运行。
- 能安全更新。

那么它就不是一个玩具项目，而是一个可扩展到 AI 玩偶、桌面 AI、老人陪护、儿童学习、家庭 Agent 终端的 **端侧 AI Agent OS**。
