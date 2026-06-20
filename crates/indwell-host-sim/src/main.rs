mod context;
mod ota;
mod planner;
mod provider_config;
mod tools;

use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use chrono::Utc;
use context::{apply_context_pack, contextual_chat_request, ContextAssembler};
use indwell_channel::{
    BasicChannelAdapter, ChannelAdapter, ChannelEvent, ChannelInbound, ChannelKind, ChannelPolicy,
    ChannelPrincipal,
};
use indwell_core::{
    AgentRun, AuthContext, AuthMethod, DeviceState, Event, ProviderSelection, RunStatus,
    ToolAuditOutcome,
};
use indwell_memory::{
    JsonlMemoryStore, MemoryExport, MemoryKind, MemoryMetabolismReport, MemoryQuery, MemoryRecord,
    MemorySource, MemoryStore, Sensitivity, TtlPolicy,
};
use indwell_ota::{OtaManifest, OtaSlot, OtaTrustStore, OtaVerificationReport};
use indwell_protocol::MobileCommand;
use indwell_protocol::{
    ApiEnvelope, CustomWebhookInputRequest, ProviderConfig, ProviderConfigSet, ProvisioningRequest,
    ProvisioningResponse,
};
use indwell_provider::{
    AsrProvider, AudioBlob, ChatMessage, ChatRequest, EmbeddingProvider, LlmProvider,
    MockAsrProvider, MockEmbeddingProvider, MockLlmProvider, MockTtsProvider, MockVisionProvider,
    OpenAiCompatibleConfig, OpenAiCompatibleProvider, ToolCall, Transcript, TtsProvider,
    VisionProvider, VisionRequest, VoiceProfile,
};
use indwell_reflection::{ReflectionBudget, ReflectionEngine, ReflectionReport};
use indwell_runs::{JsonlRunStore, RunLedgerEntry, RunStore};
use indwell_security::{
    AuthSession, ConfirmationGrant, ConfirmationGrantManager, FileSealedSecretStore,
    JsonConfirmationGrantStore, JsonPairedDeviceStore, PairedDevice, PairingChallenge,
    PairingManager, PassphraseChallenge, PassphraseChallengeManager, PolicyDecision, PolicyEngine,
    SessionTokenManager, SignedRequest, StoredSecret,
};
use ota::JsonOtaManifestStore;
use planner::plan_tool_calls;
use provider_config::JsonProviderConfigStore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::Mutex;
use tools::{lookup_tool, tool_catalog, HostToolCatalogItem};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    memory: Arc<Mutex<JsonlMemoryStore>>,
    runs: Arc<Mutex<JsonlRunStore>>,
    providers: Arc<Mutex<JsonProviderConfigStore>>,
    provisioning: Arc<Mutex<Option<ProvisioningRequest>>>,
    secrets: Arc<Mutex<FileSealedSecretStore>>,
    pairing: Arc<Mutex<PairingManager>>,
    paired_devices: Arc<JsonPairedDeviceStore>,
    sessions: Arc<Mutex<SessionTokenManager>>,
    grants: Arc<Mutex<ConfirmationGrantManager>>,
    confirmation_grants: Arc<JsonConfirmationGrantStore>,
    passphrases: Arc<Mutex<PassphraseChallengeManager>>,
    ota: Arc<Mutex<JsonOtaManifestStore>>,
    context: Arc<ContextAssembler>,
    policy: Arc<PolicyEngine>,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    service: &'static str,
    status: &'static str,
}

#[derive(Debug, Deserialize)]
struct ChannelInputRequest {
    channel: ChannelKind,
    session_id: String,
    subject_hint: Option<String>,
    session_token: Option<String>,
    text: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChannelInputResponse {
    event: ChannelEvent,
    run_id: String,
    reply: String,
    memory_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateMemoryRequest {
    kind: MemoryKind,
    wing: String,
    room: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AcceptMemoryRequest {
    wing: Option<String>,
    room: Option<String>,
    kind: Option<MemoryKind>,
    sensitivity: Option<Sensitivity>,
    confidence: Option<f32>,
    importance: Option<f32>,
}

#[derive(Debug, Serialize)]
struct MemoryAuditResponse {
    record: MemoryRecord,
    status: String,
    recommendation: String,
    related_run_id: Option<String>,
    evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
struct AcceptMemoryResponse {
    accepted: bool,
    record: MemoryRecord,
}

#[derive(Debug, Serialize)]
struct ToolDecisionResponse {
    tool: String,
    decision: PolicyDecision,
}

#[derive(Debug, Deserialize)]
struct ToolExecuteRequest {
    channel: ChannelKind,
    session_token: Option<String>,
    confirmation_grant_id: Option<String>,
    input: Option<Value>,
}

#[derive(Debug, Serialize)]
struct ToolExecuteResponse {
    run_id: String,
    tool: String,
    decision: PolicyDecision,
    output: Option<Value>,
    memory_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChannelPolicyResponse {
    policies: Vec<ChannelPolicy>,
}

#[derive(Debug, Deserialize)]
struct VoiceTurnRequest {
    text_hint: String,
    voice: Option<String>,
}

#[derive(Debug, Serialize)]
struct VoiceTurnResponse {
    run_id: String,
    transcript: Transcript,
    reply: String,
    audio: AudioBlob,
}

#[derive(Debug, Deserialize)]
struct PutSecretRequest {
    secret: String,
}

#[derive(Debug, Deserialize)]
struct CompletePairingRequest {
    session_id: String,
    code: String,
    label: String,
    public_key: String,
    signature: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IssueSessionRequest {
    device_id: String,
    subject_id: Option<String>,
    timestamp_ms: u64,
    nonce: String,
    method: String,
    path: String,
    body_sha256: String,
    signature: String,
}

#[derive(Debug, Serialize)]
struct IssueSessionResponse {
    session: AuthSession,
    token: String,
}

#[derive(Debug, Deserialize)]
struct VerifyPassphraseRequest {
    challenge_id: String,
    spoken_phrase: String,
    subject_id: String,
    allowed_tool: String,
}

#[derive(Debug, Serialize)]
struct VerifyPassphraseResponse {
    verified: bool,
    grant: ConfirmationGrant,
}

#[derive(Debug, Deserialize)]
struct ReflectionRunRequest {
    limit: Option<usize>,
    allow_sensitive: Option<bool>,
    allow_skill_generation: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ToolCatalogResponse {
    tools: Vec<HostToolCatalogItem>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "indwell_host_sim=info,tower_http=info".to_string()),
        )
        .init();

    let data_root = std::env::var("INDWELL_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data/host-sim"));
    let state = init_state(data_root)?;
    let app = build_router(state);

    let addr: SocketAddr = std::env::var("INDWELL_HOST_SIM_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3030".to_string())
        .parse()?;
    tracing::info!("Indwell host simulator listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn init_state(data_root: PathBuf) -> anyhow::Result<AppState> {
    let memory = JsonlMemoryStore::new(data_root.join("memory"))?;
    let runs = JsonlRunStore::new(data_root.join("runs"))?;
    let providers = JsonProviderConfigStore::new(data_root.join("config"))?;
    let ota = JsonOtaManifestStore::new(data_root.join("ota"))?;
    let secrets = FileSealedSecretStore::new(
        data_root.join("secrets"),
        host_secret_store_key(std::env::var("INDWELL_HOST_SIM_SECRET_KEY").ok().as_deref()),
    )?;
    let paired_devices_store = JsonPairedDeviceStore::new(data_root.join("pairing/devices.json"))?;
    let pairing = PairingManager::from_paired_devices(paired_devices_store.load()?);
    let confirmation_grants_store =
        JsonConfirmationGrantStore::new(data_root.join("auth/confirmation_grants.json"))?;
    let grants = ConfirmationGrantManager::from_grants(confirmation_grants_store.load()?);
    let sessions = SessionTokenManager::new(host_secret_store_key(
        std::env::var("INDWELL_HOST_SIM_SESSION_KEY")
            .ok()
            .as_deref(),
    ));

    Ok(AppState {
        memory: Arc::new(Mutex::new(memory)),
        runs: Arc::new(Mutex::new(runs)),
        providers: Arc::new(Mutex::new(providers)),
        provisioning: Arc::new(Mutex::new(None)),
        secrets: Arc::new(Mutex::new(secrets)),
        pairing: Arc::new(Mutex::new(pairing)),
        paired_devices: Arc::new(paired_devices_store),
        sessions: Arc::new(Mutex::new(sessions)),
        grants: Arc::new(Mutex::new(grants)),
        confirmation_grants: Arc::new(confirmation_grants_store),
        passphrases: Arc::new(Mutex::new(PassphraseChallengeManager::default())),
        ota: Arc::new(Mutex::new(ota)),
        context: Arc::new(ContextAssembler::default()),
        policy: Arc::new(PolicyEngine),
    })
}

fn build_router(state: AppState) -> Router {
    let public_routes = Router::new()
        .route("/health", get(health))
        .route("/v1/channel/input", post(channel_input))
        .route("/v1/voice/mock-turn", post(mock_voice_turn))
        .route("/v1/gateway/custom-webhook", post(custom_webhook_input))
        .route("/v1/channels/policies", get(channel_policies))
        .route("/v1/pairing/challenge", post(issue_pairing_challenge))
        .route("/v1/pairing/complete", post(complete_pairing))
        .route("/v1/auth/session", post(issue_auth_session))
        .route(
            "/v1/auth/passphrase/challenge",
            post(issue_passphrase_challenge),
        )
        .route(
            "/v1/auth/passphrase/verify",
            post(verify_passphrase_challenge),
        );

    let protected_routes = Router::new()
        .route("/v1/memory", post(create_memory))
        .route("/v1/memory/:id/accept", post(accept_memory))
        .route("/v1/memory/:id/audit", get(audit_memory))
        .route("/v1/memory/export", get(export_memory))
        .route("/v1/memory/metabolize", post(metabolize_memory))
        .route("/v1/memory/search", post(search_memory))
        .route("/v1/reflection/run", post(run_reflection))
        .route(
            "/v1/provisioning",
            get(get_provisioning).post(save_provisioning),
        )
        .route("/v1/providers", get(get_providers).put(save_providers))
        .route("/v1/providers/test", post(test_provider))
        .route(
            "/v1/secrets/:key_ref",
            get(get_secret).put(put_secret).delete(delete_secret),
        )
        .route("/v1/pairing/devices", get(list_paired_devices))
        .route("/v1/pairing/devices/:id", delete(revoke_paired_device))
        .route("/v1/ota/manifest", get(get_ota_manifest))
        .route("/v1/ota/check", post(check_ota_manifest))
        .route("/v1/ota/trust", get(get_ota_trust).put(save_ota_trust))
        .route("/v1/runs", get(list_runs))
        .route("/v1/runs/:id", get(get_run))
        .route("/v1/runs/:id/entries", get(get_run_entries))
        .route("/v1/tools", get(list_tools))
        .route("/v1/tools/:tool/check", post(check_tool))
        .route("/v1/tools/:tool/execute", post(execute_tool))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_session_auth,
        ));

    let app = public_routes
        .merge(protected_routes)
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());
    app
}

async fn health() -> Json<ApiEnvelope<HealthResponse>> {
    Json(ApiEnvelope::ok(HealthResponse {
        service: "indwell-host-sim",
        status: "ok",
    }))
}

async fn require_session_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: axum::extract::Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = session_token_from_headers(&headers).ok_or(AppError::MissingSessionToken)?;
    let session = state
        .sessions
        .lock()
        .await
        .verify(&token, Utc::now().timestamp_millis() as u64)?;
    request.extensions_mut().insert(session);
    Ok(next.run(request).await)
}

fn session_token_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            headers
                .get("x-indwell-session-token")
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
        })
}

async fn channel_input(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ChannelInputRequest>,
) -> Result<Json<ApiEnvelope<ChannelInputResponse>>, AppError> {
    let header_token = session_token_from_headers(&headers);
    let token = req.session_token.as_deref().or(header_token.as_deref());
    let principal = match token {
        Some(token) => Some(principal_from_session_token(&state, token).await?),
        None => None,
    };
    handle_channel_event(
        state,
        ChannelInbound {
            channel: req.channel,
            session_id: req.session_id.clone(),
            subject_hint: req.subject_hint,
            principal,
            text: req.text,
            command: None,
        },
    )
    .await
}

async fn custom_webhook_input(
    State(state): State<AppState>,
    Json(req): Json<CustomWebhookInputRequest>,
) -> Result<Json<ApiEnvelope<ChannelInputResponse>>, AppError> {
    handle_channel_event(state, req.into()).await
}

async fn mock_voice_turn(
    State(state): State<AppState>,
    Json(req): Json<VoiceTurnRequest>,
) -> Result<Json<ApiEnvelope<VoiceTurnResponse>>, AppError> {
    let providers = state.providers.lock().await.load()?;
    let auth_context = AuthContext::anonymous();
    let (providers, provider_policy_note) = provider_config_for_ingress(&providers, &auth_context);
    let transcript = transcribe_with_config(
        &state,
        &providers,
        AudioBlob {
            bytes: req.text_hint.as_bytes().to_vec(),
            mime_type: "audio/mock-text-hint".to_string(),
            duration_ms: Some((req.text_hint.len() as u32).saturating_mul(40)),
        },
    )
    .await?;
    let mut run = AgentRun::new(
        Event::AudioCaptured {
            path: "host-sim://voice/mock-turn".to_string(),
            duration_ms: req
                .text_hint
                .len()
                .saturating_mul(40)
                .try_into()
                .unwrap_or(u32::MAX),
        },
        Some(transcript.text.clone()),
        auth_context,
        DeviceState::Thinking,
        ProviderSelection {
            llm: format!("{}:{}", providers.llm.kind, providers.llm.model),
            vision: providers
                .vision
                .as_ref()
                .map(|provider| format!("{}:{}", provider.kind, provider.model)),
            asr: providers
                .asr
                .as_ref()
                .map(|provider| format!("{}:{}", provider.kind, provider.model)),
            tts: providers
                .tts
                .as_ref()
                .map(|provider| format!("{}:{}", provider.kind, provider.model)),
            embedding: providers
                .embedding
                .as_ref()
                .map(|provider| format!("{}:{}", provider.kind, provider.model)),
        },
        Utc::now().timestamp_millis() as u64,
    );
    run.audit.input_summary = Some(transcript.text.clone());
    checkpoint_run(&state, &run, "created").await?;

    run.status = RunStatus::AssemblingContext;
    let memory_query = state.context.memory_query(&transcript.text);
    let retrieved_memories = state.memory.lock().await.search(memory_query)?;
    let assembly = state.context.assemble(
        &transcript.text,
        ChannelKind::LocalPwa,
        retrieved_memories,
        &state.policy,
        &run.auth_context,
    );
    apply_context_pack(&mut run, DeviceState::Thinking, assembly);
    if let Some(note) = provider_policy_note {
        run.record_policy_block(note);
    }
    checkpoint_run(&state, &run, "context").await?;

    run.status = RunStatus::WaitingForProvider;
    let api_key = match providers.llm.api_key_ref.as_deref() {
        Some(key_ref) => match resolve_api_key(&state, key_ref).await {
            Ok(api_key) => api_key,
            Err(error) => {
                fail_run(
                    &state,
                    &mut run,
                    "failed",
                    format!("llm api key: {error:?}"),
                )
                .await?;
                return Err(error);
            }
        },
        None => None,
    };
    let reply = match chat_with_config(
        &providers,
        api_key,
        contextual_chat_request(&run, &transcript.text),
    )
    .await
    {
        Ok(response) => response.text,
        Err(error) => {
            fail_run(
                &state,
                &mut run,
                "failed",
                format!("llm provider: {error:?}"),
            )
            .await?;
            return Err(error);
        }
    };
    checkpoint_run(&state, &run, "provider").await?;
    let audio = match synthesize_with_config(
        &state,
        &providers,
        &reply,
        VoiceProfile {
            voice: req.voice.unwrap_or_else(|| "warm_indwell".to_string()),
            language: Some("en".to_string()),
        },
    )
    .await
    {
        Ok(audio) => audio,
        Err(error) => {
            fail_run(
                &state,
                &mut run,
                "failed",
                format!("tts provider: {error:?}"),
            )
            .await?;
            return Err(error);
        }
    };
    run.finish_with_summary(reply.clone(), Utc::now().timestamp_millis() as u64);
    state.runs.lock().await.append(&run)?;

    Ok(Json(ApiEnvelope::ok(VoiceTurnResponse {
        run_id: run.id.to_string(),
        transcript,
        reply,
        audio,
    })))
}

async fn handle_channel_event(
    state: AppState,
    inbound: ChannelInbound,
) -> Result<Json<ApiEnvelope<ChannelInputResponse>>, AppError> {
    let auth_context = auth_context_for_inbound(&inbound);
    let adapter = BasicChannelAdapter::new(inbound.channel);
    let event = adapter.normalize_inbound(inbound)?;

    let text = match &event {
        ChannelEvent::UserText { text, .. } => text.clone(),
        ChannelEvent::RemoteCommand { command, .. } => command_prompt(command),
        _ => String::new(),
    };
    let mut run = AgentRun::new(
        Event::ChannelMessage {
            channel: format!("{:?}", event.channel()),
            session_id: match &event {
                ChannelEvent::UserText { session_id, .. }
                | ChannelEvent::UserImage { session_id, .. }
                | ChannelEvent::UserAudio { session_id, .. }
                | ChannelEvent::Confirmation { session_id, .. }
                | ChannelEvent::RemoteCommand { session_id, .. } => session_id.clone(),
            },
        },
        Some(text.clone()),
        auth_context,
        DeviceState::Thinking,
        ProviderSelection {
            llm: "mock:phase0".to_string(),
            vision: None,
            asr: None,
            tts: None,
            embedding: None,
        },
        Utc::now().timestamp_millis() as u64,
    );
    run.audit.input_summary = Some(text.clone());
    checkpoint_run(&state, &run, "created").await?;

    run.status = RunStatus::AssemblingContext;
    let memory_query = state.context.memory_query(&text);
    let retrieved_memories = state.memory.lock().await.search(memory_query)?;
    let assembly = state.context.assemble(
        &text,
        event.channel(),
        retrieved_memories,
        &state.policy,
        &run.auth_context,
    );
    apply_context_pack(&mut run, DeviceState::Thinking, assembly);
    checkpoint_run(&state, &run, "context").await?;

    run.status = RunStatus::WaitingForProvider;
    let providers = state.providers.lock().await.load()?;
    let (providers, provider_policy_note) =
        provider_config_for_ingress(&providers, &run.auth_context);
    run.provider.llm = format!("{}:{}", providers.llm.kind, providers.llm.model);
    if let Some(note) = provider_policy_note {
        run.record_policy_block(note);
    }
    let api_key = match providers.llm.api_key_ref.as_deref() {
        Some(key_ref) => match resolve_api_key(&state, key_ref).await {
            Ok(api_key) => api_key,
            Err(error) => {
                fail_run(
                    &state,
                    &mut run,
                    "failed",
                    format!("llm api key: {error:?}"),
                )
                .await?;
                return Err(error);
            }
        },
        None => None,
    };
    let chat_response =
        match chat_with_config(&providers, api_key, contextual_chat_request(&run, &text)).await {
            Ok(response) => response,
            Err(error) => {
                fail_run(
                    &state,
                    &mut run,
                    "failed",
                    format!("llm provider: {error:?}"),
                )
                .await?;
                return Err(error);
            }
        };
    let reply = chat_response.text;
    checkpoint_run(&state, &run, "provider").await?;

    run.status = RunStatus::WaitingForTool;
    if chat_response.tool_calls.is_empty() {
        execute_planned_tool_calls(&state, &mut run, &text).await;
    } else {
        execute_provider_tool_calls(&state, &mut run, chat_response.tool_calls).await;
    }
    checkpoint_run(&state, &run, "tool").await?;

    let memory_id = if !text.trim().is_empty() {
        let owner_authenticated = run.auth_context.owner_authenticated;
        let (wing, room) = memory_target_for_ingress(&run.auth_context);
        let mut record = MemoryRecord::new(
            MemoryKind::Episodic,
            wing,
            room,
            format!("{} -> {}", text, reply),
            MemorySource::AgentRun {
                run_id: run.id.to_string(),
            },
            Utc::now().timestamp_millis() as u64,
        );
        record.tags.push(format!("channel:{:?}", event.channel()));
        if !owner_authenticated {
            record.tags.push("unverified_ingress".to_string());
            record.confidence = 0.35;
            record.importance = 0.2;
            record.sensitivity = Sensitivity::Personal;
            record.ttl_policy = TtlPolicy::Review;
        }
        let id = record.id.clone();
        state.memory.lock().await.append(record)?;
        run.record_written_memory(id.clone());
        Some(id)
    } else {
        None
    };
    run.finish_with_summary(reply.clone(), Utc::now().timestamp_millis() as u64);
    state.runs.lock().await.append(&run)?;

    Ok(Json(ApiEnvelope::ok(ChannelInputResponse {
        event,
        run_id: run.id.to_string(),
        reply,
        memory_id,
    })))
}

fn auth_context_for_inbound(inbound: &ChannelInbound) -> AuthContext {
    if let Some(principal) = &inbound.principal {
        if principal.owner_authenticated {
            return AuthContext::owner(&principal.subject_id, vec![AuthMethod::PairedDevice]);
        }
        AuthContext {
            subject_id: Some(principal.subject_id.clone()),
            owner_authenticated: false,
            methods: vec![],
        }
    } else {
        AuthContext::anonymous()
    }
}

fn provider_config_for_ingress(
    providers: &ProviderConfigSet,
    auth: &AuthContext,
) -> (ProviderConfigSet, Option<String>) {
    if auth.owner_authenticated || !provider_set_uses_external_provider(providers) {
        return (providers.clone(), None);
    }

    (
        mock_provider_config(),
        Some(
            "unauthenticated ingress forced to mock provider to protect user-owned API keys"
                .to_string(),
        ),
    )
}

fn provider_set_uses_external_provider(providers: &ProviderConfigSet) -> bool {
    provider_kind_is_external(&providers.llm.kind)
        || providers
            .vision
            .as_ref()
            .is_some_and(|provider| provider_kind_is_external(&provider.kind))
        || providers
            .asr
            .as_ref()
            .is_some_and(|provider| provider_kind_is_external(&provider.kind))
        || providers
            .tts
            .as_ref()
            .is_some_and(|provider| provider_kind_is_external(&provider.kind))
        || providers
            .embedding
            .as_ref()
            .is_some_and(|provider| provider_kind_is_external(&provider.kind))
}

fn provider_kind_is_external(kind: &str) -> bool {
    !matches!(kind, "mock")
}

fn mock_provider_config() -> ProviderConfigSet {
    ProviderConfigSet {
        llm: ProviderConfig {
            kind: "mock".to_string(),
            base_url: None,
            api_key_ref: None,
            model: "mock:phase0".to_string(),
            max_input_tokens: Some(4000),
            max_output_tokens: Some(600),
        },
        vision: None,
        asr: None,
        tts: None,
        embedding: None,
    }
}

fn memory_target_for_ingress(auth: &AuthContext) -> (&'static str, &'static str) {
    if auth.owner_authenticated {
        ("user_unknown", "episodes")
    } else {
        ("inbox", "unverified")
    }
}

async fn principal_from_session_token(
    state: &AppState,
    token: &str,
) -> Result<ChannelPrincipal, AppError> {
    let session = state
        .sessions
        .lock()
        .await
        .verify(token, Utc::now().timestamp_millis() as u64)?;
    Ok(ChannelPrincipal {
        subject_id: session.subject_id,
        owner_authenticated: true,
    })
}

fn command_prompt(command: &MobileCommand) -> String {
    match command {
        MobileCommand::SendText { text } => text.clone(),
        MobileCommand::CaptureImage => "capture camera image".to_string(),
        MobileCommand::SearchMemory { query } => format!("search memory {query}"),
        MobileCommand::DeleteMemory { id } => format!("delete memory {id}"),
        MobileCommand::SystemStatus => "system status".to_string(),
        MobileCommand::CheckUpdate => "check update".to_string(),
        MobileCommand::ApplyUpdate { version } => format!("apply update {version}"),
    }
}

async fn execute_planned_tool_calls(state: &AppState, run: &mut AgentRun, text: &str) {
    let planned_tools = plan_tool_calls(text, &run.allowed_tools);
    for planned in planned_tools {
        if tool_requires_runtime_confirmation(run, &planned.tool) {
            record_tool_confirmation_block(run, &planned.tool, "planned tool");
            continue;
        }

        match execute_mock_tool(state, &planned.tool, planned.input).await {
            Ok(execution) => {
                run.record_tool_call(
                    &planned.tool,
                    ToolAuditOutcome::Completed,
                    format!("planned tool: {}", execution.summary),
                );
                if let Some(memory_id) = execution.memory_id {
                    run.record_written_memory(memory_id);
                }
            }
            Err(error) => {
                run.record_tool_call(
                    &planned.tool,
                    ToolAuditOutcome::Failed,
                    format!("planned tool failed: {error:?}"),
                );
            }
        }
    }
}

async fn execute_provider_tool_calls(
    state: &AppState,
    run: &mut AgentRun,
    tool_calls: Vec<ToolCall>,
) {
    for call in tool_calls {
        if !run.allowed_tools.iter().any(|tool| tool.name == call.name) {
            let summary = format!("provider requested unavailable tool {}", call.name);
            run.record_tool_call(&call.name, ToolAuditOutcome::Blocked, summary.clone());
            run.record_policy_block(summary);
            continue;
        }
        if tool_requires_runtime_confirmation(run, &call.name) {
            record_tool_confirmation_block(run, &call.name, "provider tool");
            continue;
        }

        match execute_mock_tool(state, &call.name, call.arguments).await {
            Ok(execution) => {
                run.record_tool_call(
                    &call.name,
                    ToolAuditOutcome::Completed,
                    format!("provider tool {}: {}", call.id, execution.summary),
                );
                if let Some(memory_id) = execution.memory_id {
                    run.record_written_memory(memory_id);
                }
            }
            Err(error) => {
                run.record_tool_call(
                    &call.name,
                    ToolAuditOutcome::Failed,
                    format!("provider tool {} failed: {error:?}", call.id),
                );
            }
        }
    }
}

fn tool_requires_runtime_confirmation(run: &AgentRun, tool_name: &str) -> bool {
    run.allowed_tools
        .iter()
        .find(|tool| tool.name == tool_name)
        .is_some_and(|tool| tool.requires_confirmation)
}

fn record_tool_confirmation_block(run: &mut AgentRun, tool_name: &str, source: &str) {
    let summary = format!(
        "{source} requested {tool_name}, but this high-risk tool requires explicit confirmation"
    );
    run.record_tool_call(tool_name, ToolAuditOutcome::Blocked, summary.clone());
    run.record_policy_block(summary);
}

async fn chat_with_config(
    providers: &ProviderConfigSet,
    api_key: Option<String>,
    req: ChatRequest,
) -> Result<indwell_provider::ChatResponse, AppError> {
    match providers.llm.kind.as_str() {
        "mock" => Ok(MockLlmProvider.chat(req).await?),
        "openai_compatible" => {
            let api_key_ref = providers
                .llm
                .api_key_ref
                .clone()
                .unwrap_or_else(|| "key_llm_main".to_string());
            let provider = OpenAiCompatibleProvider::new(OpenAiCompatibleConfig {
                base_url: providers.llm.base_url.clone().unwrap_or_default(),
                api_key: api_key.or_else(|| resolve_api_key_from_env(&api_key_ref)),
                api_key_ref,
                model: providers.llm.model.clone(),
                max_input_tokens: providers.llm.max_input_tokens,
                max_output_tokens: providers.llm.max_output_tokens,
            });
            Ok(provider.chat(req).await?)
        }
        other => Err(AppError::UnsupportedProviderKind(other.to_string())),
    }
}

async fn transcribe_with_config(
    state: &AppState,
    providers: &ProviderConfigSet,
    audio: AudioBlob,
) -> Result<Transcript, AppError> {
    let Some(config) = providers.asr.as_ref() else {
        return Ok(MockAsrProvider.transcribe(audio).await?);
    };
    match config.kind.as_str() {
        "mock" => Ok(MockAsrProvider.transcribe(audio).await?),
        "same_as_llm" | "openai_compatible" => {
            let resolved = provider_config_with_llm_fallback(config, &providers.llm);
            let api_key = match resolved.api_key_ref.as_deref() {
                Some(key_ref) => resolve_api_key(state, key_ref).await?,
                None => None,
            };
            let provider = openai_provider_from_config(&resolved, api_key);
            Ok(provider.transcribe(audio).await?)
        }
        other => Err(AppError::UnsupportedProviderKind(other.to_string())),
    }
}

async fn synthesize_with_config(
    state: &AppState,
    providers: &ProviderConfigSet,
    text: &str,
    voice: VoiceProfile,
) -> Result<AudioBlob, AppError> {
    let Some(config) = providers.tts.as_ref() else {
        return Ok(MockTtsProvider.synthesize(text, voice).await?);
    };
    match config.kind.as_str() {
        "mock" => Ok(MockTtsProvider.synthesize(text, voice).await?),
        "same_as_llm" | "openai_compatible" => {
            let resolved = provider_config_with_llm_fallback(config, &providers.llm);
            let api_key = match resolved.api_key_ref.as_deref() {
                Some(key_ref) => resolve_api_key(state, key_ref).await?,
                None => None,
            };
            let provider = openai_provider_from_config(&resolved, api_key);
            Ok(provider.synthesize(text, voice).await?)
        }
        other => Err(AppError::UnsupportedProviderKind(other.to_string())),
    }
}

async fn analyze_image_with_config(
    state: &AppState,
    providers: &ProviderConfigSet,
    req: VisionRequest,
) -> Result<indwell_provider::VisionResponse, AppError> {
    let Some(config) = providers.vision.as_ref() else {
        return Ok(MockVisionProvider.analyze_image(req).await?);
    };
    match config.kind.as_str() {
        "mock" => Ok(MockVisionProvider.analyze_image(req).await?),
        "same_as_llm" | "openai_compatible" => {
            let resolved = provider_config_with_llm_fallback(config, &providers.llm);
            let api_key = match resolved.api_key_ref.as_deref() {
                Some(key_ref) => resolve_api_key(state, key_ref).await?,
                None => None,
            };
            let provider = openai_provider_from_config(&resolved, api_key);
            Ok(provider.analyze_image(req).await?)
        }
        other => Err(AppError::UnsupportedProviderKind(other.to_string())),
    }
}

async fn embed_with_config(
    state: &AppState,
    providers: &ProviderConfigSet,
    input: &str,
) -> Result<Vec<f32>, AppError> {
    let Some(config) = providers.embedding.as_ref() else {
        return Ok(MockEmbeddingProvider.embed(input).await?);
    };
    match config.kind.as_str() {
        "mock" => Ok(MockEmbeddingProvider.embed(input).await?),
        "same_as_llm" | "openai_compatible" => {
            let resolved = provider_config_with_llm_fallback(config, &providers.llm);
            let api_key = match resolved.api_key_ref.as_deref() {
                Some(key_ref) => resolve_api_key(state, key_ref).await?,
                None => None,
            };
            let provider = openai_provider_from_config(&resolved, api_key);
            Ok(provider.embed(input).await?)
        }
        other => Err(AppError::UnsupportedProviderKind(other.to_string())),
    }
}

fn provider_config_with_llm_fallback(
    config: &ProviderConfig,
    llm: &ProviderConfig,
) -> ProviderConfig {
    ProviderConfig {
        kind: if config.kind == "same_as_llm" {
            llm.kind.clone()
        } else {
            config.kind.clone()
        },
        base_url: config.base_url.clone().or_else(|| llm.base_url.clone()),
        api_key_ref: config
            .api_key_ref
            .clone()
            .or_else(|| llm.api_key_ref.clone()),
        model: if config.model.trim().is_empty() {
            llm.model.clone()
        } else {
            config.model.clone()
        },
        max_input_tokens: config.max_input_tokens.or(llm.max_input_tokens),
        max_output_tokens: config.max_output_tokens.or(llm.max_output_tokens),
    }
}

fn openai_provider_from_config(
    config: &ProviderConfig,
    api_key: Option<String>,
) -> OpenAiCompatibleProvider {
    OpenAiCompatibleProvider::new(OpenAiCompatibleConfig {
        base_url: config.base_url.clone().unwrap_or_default(),
        api_key: api_key.or_else(|| {
            config
                .api_key_ref
                .as_deref()
                .and_then(resolve_api_key_from_env)
        }),
        api_key_ref: config
            .api_key_ref
            .clone()
            .unwrap_or_else(|| "key_llm_main".to_string()),
        model: config.model.clone(),
        max_input_tokens: config.max_input_tokens,
        max_output_tokens: config.max_output_tokens,
    })
}

async fn resolve_api_key(state: &AppState, api_key_ref: &str) -> Result<Option<String>, AppError> {
    let stored = state.secrets.lock().await.get(api_key_ref)?;
    Ok(stored
        .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
        .or_else(|| resolve_api_key_from_env(api_key_ref)))
}

fn host_secret_store_key(seed: Option<&str>) -> [u8; 32] {
    use sha2::{Digest, Sha256};

    let seed = seed.unwrap_or("indwell-host-sim-local-development-key");
    Sha256::digest(seed.as_bytes()).into()
}

fn resolve_api_key_from_env(api_key_ref: &str) -> Option<String> {
    let env_name = api_key_env_name(api_key_ref);
    std::env::var(env_name).ok()
}

fn api_key_env_name(api_key_ref: &str) -> String {
    let suffix = api_key_ref
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("INDWELL_SECRET_{suffix}")
}

fn memory_audit_response(record: MemoryRecord) -> MemoryAuditResponse {
    let is_unverified = record.tags.iter().any(|tag| tag == "unverified_ingress");
    let related_run_id = match &record.source {
        MemorySource::AgentRun { run_id } => Some(run_id.clone()),
        _ => None,
    };
    let mut evidence = vec![
        format!("source={}", memory_source_label(&record.source)),
        format!("wing_room={}/{}", record.wing, record.room),
        format!("confidence={:.2}", record.confidence),
        format!("importance={:.2}", record.importance),
        format!("sensitivity={:?}", record.sensitivity),
    ];
    if !record.tags.is_empty() {
        evidence.push(format!("tags={}", record.tags.join(",")));
    }
    if let Some(verbatim_ref) = record.verbatim_ref.as_deref() {
        evidence.push(format!("verbatim_ref={verbatim_ref}"));
    }

    let (status, recommendation) = if is_unverified {
        (
            "unverified".to_string(),
            "Review this inbox memory before it can influence long-term persona or relationship snapshots."
                .to_string(),
        )
    } else if matches!(record.ttl_policy, TtlPolicy::Review) {
        (
            "review".to_string(),
            "Memory is available but still marked for future review or metabolism.".to_string(),
        )
    } else {
        (
            "accepted".to_string(),
            "Memory is accepted for normal local-first retrieval and snapshots.".to_string(),
        )
    };

    MemoryAuditResponse {
        record,
        status,
        recommendation,
        related_run_id,
        evidence,
    }
}

fn memory_source_label(source: &MemorySource) -> String {
    match source {
        MemorySource::UserSaid => "user_said".to_string(),
        MemorySource::DeviceEvent => "device_event".to_string(),
        MemorySource::AgentRun { run_id } => format!("agent_run:{run_id}"),
        MemorySource::Reflection => "reflection".to_string(),
        MemorySource::Imported => "imported".to_string(),
        MemorySource::Manual => "manual".to_string(),
    }
}

fn push_unique_tag(tags: &mut Vec<String>, tag: &str) {
    if !tags.iter().any(|existing| existing == tag) {
        tags.push(tag.to_string());
    }
}

fn host_sim_camera_jpeg() -> &'static [u8] {
    // Minimal JPEG-like fixture: enough for provider request plumbing without storing media.
    &[
        0xff, 0xd8, 0xff, 0xe0, b'I', b'N', b'D', b'W', b'E', b'L', b'L', 0xff, 0xd9,
    ]
}

fn decode_hex_or_utf8(value: &str) -> Result<Vec<u8>, AppError> {
    let trimmed = value.trim();
    if trimmed.len() % 2 == 0 && trimmed.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        let mut out = Vec::with_capacity(trimmed.len() / 2);
        for chunk in trimmed.as_bytes().chunks_exact(2) {
            let hi = hex_value(chunk[0])?;
            let lo = hex_value(chunk[1])?;
            out.push((hi << 4) | lo);
        }
        return Ok(out);
    }
    Ok(trimmed.as_bytes().to_vec())
}

fn hex_value(byte: u8) -> Result<u8, AppError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(AppError::InvalidHex),
    }
}

async fn create_memory(
    State(state): State<AppState>,
    Json(req): Json<CreateMemoryRequest>,
) -> Result<Json<ApiEnvelope<MemoryRecord>>, AppError> {
    let record = MemoryRecord::new(
        req.kind,
        req.wing,
        req.room,
        req.content,
        MemorySource::Manual,
        Utc::now().timestamp_millis() as u64,
    );
    state.memory.lock().await.append(record.clone())?;
    Ok(Json(ApiEnvelope::ok(record)))
}

async fn audit_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiEnvelope<MemoryAuditResponse>>, AppError> {
    let record = state
        .memory
        .lock()
        .await
        .get(&id)?
        .ok_or_else(|| AppError::MemoryNotFound(id.clone()))?;
    Ok(Json(ApiEnvelope::ok(memory_audit_response(record))))
}

async fn accept_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<AcceptMemoryRequest>,
) -> Result<Json<ApiEnvelope<AcceptMemoryResponse>>, AppError> {
    let now_ms = Utc::now().timestamp_millis() as u64;
    let mut memory = state.memory.lock().await;
    let mut record = memory
        .get(&id)?
        .ok_or_else(|| AppError::MemoryNotFound(id.clone()))?;
    record.wing = req.wing.unwrap_or_else(|| "user_unknown".to_string());
    record.room = req.room.unwrap_or_else(|| "episodes".to_string());
    if let Some(kind) = req.kind {
        record.kind = kind;
    }
    if let Some(sensitivity) = req.sensitivity {
        record.sensitivity = sensitivity;
    }
    if let Some(confidence) = req.confidence {
        record.confidence = confidence.clamp(0.0, 1.0);
    } else if record.confidence < 0.65 {
        record.confidence = 0.65;
    }
    if let Some(importance) = req.importance {
        record.importance = importance.clamp(0.0, 1.0);
    } else if record.importance < 0.4 {
        record.importance = 0.4;
    }
    record.updated_at_ms = now_ms;
    record.tags.retain(|tag| tag != "unverified_ingress");
    push_unique_tag(&mut record.tags, "reviewed_by_owner");
    push_unique_tag(&mut record.tags, "accepted_from_inbox");
    memory.append(record.clone())?;
    Ok(Json(ApiEnvelope::ok(AcceptMemoryResponse {
        accepted: true,
        record,
    })))
}

async fn export_memory(
    State(state): State<AppState>,
) -> Result<Json<ApiEnvelope<MemoryExport>>, AppError> {
    let export = state.memory.lock().await.export()?;
    Ok(Json(ApiEnvelope::ok(export)))
}

async fn search_memory(
    State(state): State<AppState>,
    Json(query): Json<MemoryQuery>,
) -> Result<Json<ApiEnvelope<Vec<MemoryRecord>>>, AppError> {
    let records = state.memory.lock().await.search(query)?;
    Ok(Json(ApiEnvelope::ok(records)))
}

async fn metabolize_memory(
    State(state): State<AppState>,
) -> Result<Json<ApiEnvelope<MemoryMetabolismReport>>, AppError> {
    let report = state
        .memory
        .lock()
        .await
        .metabolize(Utc::now().timestamp_millis() as u64)?;
    Ok(Json(ApiEnvelope::ok(report)))
}

async fn run_reflection(
    State(state): State<AppState>,
    Json(req): Json<ReflectionRunRequest>,
) -> Result<Json<ApiEnvelope<ReflectionReport>>, AppError> {
    let source_records = state.memory.lock().await.search(MemoryQuery {
        wing: None,
        room: Some("episodes".to_string()),
        text: None,
        limit: Some(req.limit.unwrap_or(20)),
    })?;
    let report = ReflectionEngine.reflect(indwell_reflection::ReflectionInput {
        source_records,
        now_ms: Utc::now().timestamp_millis() as u64,
        budget: ReflectionBudget {
            max_new_memories: 8,
            allow_sensitive: req.allow_sensitive.unwrap_or(false),
            allow_skill_generation: req.allow_skill_generation.unwrap_or(true),
        },
    })?;

    {
        let mut memory = state.memory.lock().await;
        for record in &report.new_memories {
            memory.append(record.clone())?;
        }
        for skill in &report.skills {
            let record = MemoryRecord::new(
                MemoryKind::Skill,
                "user_unknown",
                "learning",
                serde_json::to_string(skill)?,
                MemorySource::Reflection,
                Utc::now().timestamp_millis() as u64,
            );
            memory.append(record)?;
        }
    }

    Ok(Json(ApiEnvelope::ok(report)))
}

async fn save_provisioning(
    State(state): State<AppState>,
    Json(req): Json<ProvisioningRequest>,
) -> Result<Json<ApiEnvelope<ProvisioningResponse>>, AppError> {
    state.providers.lock().await.save(&req.providers)?;
    *state.provisioning.lock().await = Some(req.clone());
    Ok(Json(ApiEnvelope::ok(ProvisioningResponse {
        accepted: true,
        next_state: DeviceState::Idle,
        message: format!("provisioning accepted for {}", req.device_id),
    })))
}

async fn get_provisioning(
    State(state): State<AppState>,
) -> Json<ApiEnvelope<Option<ProvisioningRequest>>> {
    Json(ApiEnvelope::ok(state.provisioning.lock().await.clone()))
}

async fn get_providers(
    State(state): State<AppState>,
) -> Result<Json<ApiEnvelope<ProviderConfigSet>>, AppError> {
    let providers = state.providers.lock().await.load()?;
    Ok(Json(ApiEnvelope::ok(providers)))
}

async fn save_providers(
    State(state): State<AppState>,
    Json(config): Json<ProviderConfigSet>,
) -> Result<Json<ApiEnvelope<ProviderConfigSet>>, AppError> {
    state.providers.lock().await.save(&config)?;
    Ok(Json(ApiEnvelope::ok(config)))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ProviderTestTarget {
    Llm,
    Vision,
    Asr,
    Tts,
    Embedding,
}

#[derive(Debug, Deserialize)]
struct ProviderTestRequest {
    target: ProviderTestTarget,
}

#[derive(Debug, Serialize)]
struct ProviderTestResponse {
    target: ProviderTestTarget,
    provider: Option<ProviderConfig>,
    ok: bool,
    summary: String,
    details: Value,
}

async fn test_provider(
    State(state): State<AppState>,
    Json(req): Json<ProviderTestRequest>,
) -> Result<Json<ApiEnvelope<ProviderTestResponse>>, AppError> {
    let providers = state.providers.lock().await.load()?;
    let response = match req.target {
        ProviderTestTarget::Llm => test_llm_provider(&state, &providers).await,
        ProviderTestTarget::Vision => test_vision_provider(&state, &providers).await,
        ProviderTestTarget::Asr => test_asr_provider(&state, &providers).await,
        ProviderTestTarget::Tts => test_tts_provider(&state, &providers).await,
        ProviderTestTarget::Embedding => test_embedding_provider(&state, &providers).await,
    };
    Ok(Json(ApiEnvelope::ok(response)))
}

async fn test_llm_provider(
    state: &AppState,
    providers: &ProviderConfigSet,
) -> ProviderTestResponse {
    let api_key = match providers.llm.api_key_ref.as_deref() {
        Some(key_ref) => match resolve_api_key(state, key_ref).await {
            Ok(api_key) => api_key,
            Err(error) => {
                return provider_test_failure(
                    ProviderTestTarget::Llm,
                    Some(providers.llm.clone()),
                    error,
                );
            }
        },
        None => None,
    };
    let request = ChatRequest {
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "Reply with a short Indwell provider diagnostic acknowledgement.".to_string(),
        }],
        tools: Vec::new(),
    };

    match chat_with_config(providers, api_key, request).await {
        Ok(response) => ProviderTestResponse {
            target: ProviderTestTarget::Llm,
            provider: Some(providers.llm.clone()),
            ok: true,
            summary: "LLM provider responded".to_string(),
            details: json!({
                "text": response.text,
                "tool_call_count": response.tool_calls.len(),
            }),
        },
        Err(error) => {
            provider_test_failure(ProviderTestTarget::Llm, Some(providers.llm.clone()), error)
        }
    }
}

async fn test_vision_provider(
    state: &AppState,
    providers: &ProviderConfigSet,
) -> ProviderTestResponse {
    let request = VisionRequest {
        image_bytes: vec![0xff, 0xd8, 0xff, 0xd9],
        mime_type: "image/jpeg".to_string(),
        prompt: Some("Indwell provider diagnostic image.".to_string()),
    };

    match analyze_image_with_config(state, providers, request).await {
        Ok(response) => ProviderTestResponse {
            target: ProviderTestTarget::Vision,
            provider: providers.vision.clone(),
            ok: true,
            summary: "Vision provider responded".to_string(),
            details: json!({
                "description": response.description,
            }),
        },
        Err(error) => {
            provider_test_failure(ProviderTestTarget::Vision, providers.vision.clone(), error)
        }
    }
}

async fn test_asr_provider(
    state: &AppState,
    providers: &ProviderConfigSet,
) -> ProviderTestResponse {
    let audio = AudioBlob {
        bytes: b"indwell provider diagnostic".to_vec(),
        mime_type: "audio/mock-text".to_string(),
        duration_ms: Some(600),
    };

    match transcribe_with_config(state, providers, audio).await {
        Ok(transcript) => ProviderTestResponse {
            target: ProviderTestTarget::Asr,
            provider: providers.asr.clone(),
            ok: true,
            summary: "ASR provider responded".to_string(),
            details: json!({
                "text": transcript.text,
                "language": transcript.language,
            }),
        },
        Err(error) => provider_test_failure(ProviderTestTarget::Asr, providers.asr.clone(), error),
    }
}

async fn test_tts_provider(
    state: &AppState,
    providers: &ProviderConfigSet,
) -> ProviderTestResponse {
    let voice = VoiceProfile {
        voice: "warm_indwell".to_string(),
        language: Some("en".to_string()),
    };

    match synthesize_with_config(state, providers, "Indwell provider diagnostic.", voice).await {
        Ok(audio) => ProviderTestResponse {
            target: ProviderTestTarget::Tts,
            provider: providers.tts.clone(),
            ok: true,
            summary: "TTS provider responded".to_string(),
            details: json!({
                "byte_len": audio.bytes.len(),
                "mime_type": audio.mime_type,
                "duration_ms": audio.duration_ms,
            }),
        },
        Err(error) => provider_test_failure(ProviderTestTarget::Tts, providers.tts.clone(), error),
    }
}

async fn test_embedding_provider(
    state: &AppState,
    providers: &ProviderConfigSet,
) -> ProviderTestResponse {
    match embed_with_config(state, providers, "Indwell provider diagnostic.").await {
        Ok(vector) => ProviderTestResponse {
            target: ProviderTestTarget::Embedding,
            provider: providers.embedding.clone(),
            ok: true,
            summary: "Embedding provider responded".to_string(),
            details: json!({
                "dimensions": vector.len(),
                "preview": vector.iter().take(4).copied().collect::<Vec<_>>(),
            }),
        },
        Err(error) => provider_test_failure(
            ProviderTestTarget::Embedding,
            providers.embedding.clone(),
            error,
        ),
    }
}

fn provider_test_failure(
    target: ProviderTestTarget,
    provider: Option<ProviderConfig>,
    error: impl std::fmt::Debug,
) -> ProviderTestResponse {
    ProviderTestResponse {
        target,
        provider,
        ok: false,
        summary: "Provider diagnostic failed".to_string(),
        details: json!({
            "error": format!("{error:?}"),
        }),
    }
}

async fn put_secret(
    State(state): State<AppState>,
    Path(key_ref): Path<String>,
    Json(req): Json<PutSecretRequest>,
) -> Result<Json<ApiEnvelope<StoredSecret>>, AppError> {
    let stored = state.secrets.lock().await.put(
        key_ref,
        req.secret.as_bytes(),
        Utc::now().timestamp_millis() as u64,
    )?;
    Ok(Json(ApiEnvelope::ok(stored)))
}

async fn get_secret(
    State(state): State<AppState>,
    Path(key_ref): Path<String>,
) -> Result<Json<ApiEnvelope<Option<StoredSecret>>>, AppError> {
    let stored = state.secrets.lock().await.describe(&key_ref)?;
    Ok(Json(ApiEnvelope::ok(stored)))
}

async fn delete_secret(
    State(state): State<AppState>,
    Path(key_ref): Path<String>,
) -> Result<Json<ApiEnvelope<bool>>, AppError> {
    let deleted = state.secrets.lock().await.delete(&key_ref)?;
    Ok(Json(ApiEnvelope::ok(deleted)))
}

async fn issue_pairing_challenge(
    State(state): State<AppState>,
) -> Json<ApiEnvelope<PairingChallenge>> {
    let challenge = state
        .pairing
        .lock()
        .await
        .issue_challenge(Utc::now().timestamp_millis() as u64, 120_000);
    Json(ApiEnvelope::ok(challenge))
}

async fn complete_pairing(
    State(state): State<AppState>,
    Json(req): Json<CompletePairingRequest>,
) -> Result<Json<ApiEnvelope<PairedDevice>>, AppError> {
    let public_key = decode_hex_or_utf8(&req.public_key)?;
    let paired = if let Some(signature) = req.signature {
        let signature = decode_hex_or_utf8(&signature)?;
        state.pairing.lock().await.complete_pairing_signed(
            &req.session_id,
            &req.code,
            req.label,
            &public_key,
            &signature,
            Utc::now().timestamp_millis() as u64,
        )?
    } else {
        state.pairing.lock().await.complete_pairing(
            &req.session_id,
            &req.code,
            req.label,
            &public_key,
            Utc::now().timestamp_millis() as u64,
        )?
    };
    persist_paired_devices(&state).await?;
    Ok(Json(ApiEnvelope::ok(paired)))
}

async fn list_paired_devices(
    State(state): State<AppState>,
) -> Json<ApiEnvelope<Vec<PairedDevice>>> {
    Json(ApiEnvelope::ok(state.pairing.lock().await.paired_devices()))
}

async fn revoke_paired_device(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiEnvelope<PairedDevice>>, AppError> {
    let paired = state
        .pairing
        .lock()
        .await
        .revoke(&id, Utc::now().timestamp_millis() as u64)?;
    persist_paired_devices(&state).await?;
    Ok(Json(ApiEnvelope::ok(paired)))
}

async fn issue_auth_session(
    State(state): State<AppState>,
    Json(req): Json<IssueSessionRequest>,
) -> Result<Json<ApiEnvelope<IssueSessionResponse>>, AppError> {
    let now_ms = Utc::now().timestamp_millis() as u64;
    let signature = decode_hex_or_utf8(&req.signature)?;
    let signed = SignedRequest {
        device_id: req.device_id.clone(),
        timestamp_ms: req.timestamp_ms,
        nonce: req.nonce,
        method: req.method,
        path: req.path,
        body_sha256: req.body_sha256,
    };
    let mut pairing = state.pairing.lock().await;
    let device = pairing
        .paired_devices()
        .into_iter()
        .find(|device| device.device_id == req.device_id)
        .ok_or(AppError::UnknownPairedDevice)?;
    indwell_security::verify_signed_request(&device, &signed, &signature, now_ms, 5 * 60 * 1000)?;
    let device = pairing.mark_seen(&req.device_id, now_ms)?;
    drop(pairing);
    persist_paired_devices(&state).await?;
    let (session, token) = state.sessions.lock().await.issue(
        &device,
        req.subject_id.unwrap_or_else(|| "owner".to_string()),
        now_ms,
        24 * 60 * 60 * 1000,
    )?;
    Ok(Json(ApiEnvelope::ok(IssueSessionResponse {
        session,
        token,
    })))
}

async fn persist_paired_devices(state: &AppState) -> Result<(), AppError> {
    let devices = state.pairing.lock().await.paired_devices();
    state.paired_devices.save(&devices)?;
    Ok(())
}

async fn persist_confirmation_grants(state: &AppState) -> Result<(), AppError> {
    let grants = state.grants.lock().await.grants();
    state.confirmation_grants.save(&grants)?;
    Ok(())
}

async fn issue_passphrase_challenge(
    State(state): State<AppState>,
) -> Json<ApiEnvelope<PassphraseChallenge>> {
    let challenge = state
        .passphrases
        .lock()
        .await
        .issue(Utc::now().timestamp_millis() as u64, 120_000);
    Json(ApiEnvelope::ok(challenge))
}

async fn verify_passphrase_challenge(
    State(state): State<AppState>,
    Json(req): Json<VerifyPassphraseRequest>,
) -> Result<Json<ApiEnvelope<VerifyPassphraseResponse>>, AppError> {
    let now_ms = Utc::now().timestamp_millis() as u64;
    state
        .passphrases
        .lock()
        .await
        .verify(&req.challenge_id, &req.spoken_phrase, now_ms)?;
    let grant =
        state
            .grants
            .lock()
            .await
            .issue(req.subject_id, req.allowed_tool, now_ms, 5 * 60 * 1000);
    persist_confirmation_grants(&state).await?;
    Ok(Json(ApiEnvelope::ok(VerifyPassphraseResponse {
        verified: true,
        grant,
    })))
}

async fn get_ota_manifest(
    State(state): State<AppState>,
) -> Result<Json<ApiEnvelope<OtaManifest>>, AppError> {
    let manifest = state.ota.lock().await.load()?;
    Ok(Json(ApiEnvelope::ok(manifest)))
}

async fn check_ota_manifest(
    State(state): State<AppState>,
) -> Result<Json<ApiEnvelope<OtaVerificationReport>>, AppError> {
    let report = state.ota.lock().await.verify("host-sim")?;
    Ok(Json(ApiEnvelope::ok(report)))
}

async fn get_ota_trust(
    State(state): State<AppState>,
) -> Result<Json<ApiEnvelope<OtaTrustStore>>, AppError> {
    let trust = state.ota.lock().await.load_trust_store()?;
    Ok(Json(ApiEnvelope::ok(trust)))
}

async fn save_ota_trust(
    State(state): State<AppState>,
    Json(trust): Json<OtaTrustStore>,
) -> Result<Json<ApiEnvelope<OtaTrustStore>>, AppError> {
    state.ota.lock().await.save_trust_store(&trust)?;
    Ok(Json(ApiEnvelope::ok(trust)))
}

async fn list_runs(
    State(state): State<AppState>,
) -> Result<Json<ApiEnvelope<Vec<AgentRun>>>, AppError> {
    let runs = state.runs.lock().await.list()?;
    Ok(Json(ApiEnvelope::ok(runs)))
}

async fn get_run(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiEnvelope<Option<AgentRun>>>, AppError> {
    let run = state.runs.lock().await.get(id)?;
    Ok(Json(ApiEnvelope::ok(run)))
}

async fn get_run_entries(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiEnvelope<Vec<RunLedgerEntry>>>, AppError> {
    let entries = state.runs.lock().await.entries_for_run(id)?;
    Ok(Json(ApiEnvelope::ok(entries)))
}

async fn checkpoint_run(state: &AppState, run: &AgentRun, stage: &str) -> Result<(), AppError> {
    state
        .runs
        .lock()
        .await
        .append_checkpoint(run, stage, Utc::now().timestamp_millis() as u64)?;
    Ok(())
}

async fn fail_run(
    state: &AppState,
    run: &mut AgentRun,
    stage: &str,
    reason: impl Into<String>,
) -> Result<(), AppError> {
    run.mark_failed(reason, Utc::now().timestamp_millis() as u64);
    checkpoint_run(state, run, stage).await
}

async fn list_tools() -> Json<ApiEnvelope<ToolCatalogResponse>> {
    Json(ApiEnvelope::ok(ToolCatalogResponse {
        tools: tool_catalog(),
    }))
}

async fn check_tool(
    State(state): State<AppState>,
    Extension(session): Extension<AuthSession>,
    Path(tool): Path<String>,
    Json(channel): Json<ChannelKind>,
) -> Json<ApiEnvelope<ToolDecisionResponse>> {
    let descriptor = lookup_tool(&tool);
    let policy = ChannelPolicy::default_for(channel);
    let auth = AuthContext::owner(
        session.subject_id,
        vec![indwell_core::AuthMethod::PairedDevice],
    );
    let decision = state.policy.evaluate_tool(&descriptor, &auth, &policy);

    Json(ApiEnvelope::ok(ToolDecisionResponse { tool, decision }))
}

async fn execute_tool(
    State(state): State<AppState>,
    Extension(session): Extension<AuthSession>,
    Path(tool): Path<String>,
    Json(req): Json<ToolExecuteRequest>,
) -> Result<Json<ApiEnvelope<ToolExecuteResponse>>, AppError> {
    let descriptor = lookup_tool(&tool);
    let channel_policy = ChannelPolicy::default_for(req.channel);
    let auth = if let Some(token) = req.session_token.as_deref() {
        let session = state
            .sessions
            .lock()
            .await
            .verify(token, Utc::now().timestamp_millis() as u64)?;
        AuthContext::owner(
            session.subject_id,
            vec![indwell_core::AuthMethod::PairedDevice],
        )
    } else {
        AuthContext::owner(
            session.subject_id,
            vec![indwell_core::AuthMethod::PairedDevice],
        )
    };
    let decision = state
        .policy
        .evaluate_tool(&descriptor, &auth, &channel_policy);

    let now = Utc::now().timestamp_millis() as u64;
    let mut run = AgentRun::new(
        Event::ToolCallRequested {
            run_id: "host-sim-direct-tool".to_string(),
            tool: tool.clone(),
        },
        Some(format!("execute tool {tool}")),
        auth,
        DeviceState::Thinking,
        ProviderSelection {
            llm: "mock:phase0".to_string(),
            vision: None,
            asr: None,
            tts: None,
            embedding: None,
        },
        now,
    );
    run.allowed_tools.push(descriptor.clone());
    checkpoint_run(&state, &run, "created").await?;

    if decision != PolicyDecision::Allow {
        let summary = format!("blocked by policy: {decision:?}");
        run.record_tool_call(&tool, ToolAuditOutcome::Blocked, summary.clone());
        run.record_policy_block(summary);
        state.runs.lock().await.append(&run)?;
        return Ok(Json(ApiEnvelope::ok(ToolExecuteResponse {
            run_id: run.id.to_string(),
            tool,
            decision,
            output: None,
            memory_id: None,
        })));
    }

    if descriptor.requires_confirmation {
        let subject_id = run
            .auth_context
            .subject_id
            .as_deref()
            .unwrap_or("anonymous");
        let Some(grant_id) = req.confirmation_grant_id.as_deref() else {
            let summary = "missing confirmation grant for high-risk tool".to_string();
            run.record_tool_call(&tool, ToolAuditOutcome::Blocked, summary.clone());
            run.record_policy_block(summary);
            state.runs.lock().await.append(&run)?;
            return Ok(Json(ApiEnvelope::ok(ToolExecuteResponse {
                run_id: run.id.to_string(),
                tool,
                decision: PolicyDecision::RequireConfirmation,
                output: None,
                memory_id: None,
            })));
        };
        if let Err(err) = state.grants.lock().await.consume(
            grant_id,
            subject_id,
            &tool,
            Utc::now().timestamp_millis() as u64,
        ) {
            let summary = format!("confirmation grant rejected: {err}");
            run.record_tool_call(&tool, ToolAuditOutcome::Blocked, summary.clone());
            run.record_policy_block(summary);
            state.runs.lock().await.append(&run)?;
            return Ok(Json(ApiEnvelope::ok(ToolExecuteResponse {
                run_id: run.id.to_string(),
                tool,
                decision: PolicyDecision::RequireConfirmation,
                output: None,
                memory_id: None,
            })));
        }
        persist_confirmation_grants(&state).await?;
    }

    run.status = RunStatus::WaitingForTool;
    checkpoint_run(&state, &run, "tool").await?;
    let execution =
        match execute_mock_tool(&state, &tool, req.input.unwrap_or_else(|| json!({}))).await {
            Ok(execution) => execution,
            Err(error) => {
                run.record_tool_call(&tool, ToolAuditOutcome::Failed, format!("{error:?}"));
                fail_run(
                    &state,
                    &mut run,
                    "failed",
                    format!("tool execution: {error:?}"),
                )
                .await?;
                return Err(error);
            }
        };
    run.record_tool_call(
        &tool,
        ToolAuditOutcome::Completed,
        execution.summary.clone(),
    );
    if let Some(memory_id) = &execution.memory_id {
        run.record_written_memory(memory_id.clone());
    }
    run.finish_with_summary(execution.summary, Utc::now().timestamp_millis() as u64);
    state.runs.lock().await.append(&run)?;

    Ok(Json(ApiEnvelope::ok(ToolExecuteResponse {
        run_id: run.id.to_string(),
        tool,
        decision,
        output: Some(execution.output),
        memory_id: execution.memory_id,
    })))
}

#[derive(Debug)]
struct ToolExecution {
    output: Value,
    memory_id: Option<String>,
    summary: String,
}

async fn execute_mock_tool(
    state: &AppState,
    tool: &str,
    input: Value,
) -> Result<ToolExecution, AppError> {
    match tool {
        "system.status" => {
            let providers = state.providers.lock().await.load()?;
            Ok(ToolExecution {
                output: json!({
                    "state": "idle",
                    "network": "host-sim",
                    "provider": {
                        "kind": providers.llm.kind,
                        "model": providers.llm.model,
                    },
                    "memory_backend": "jsonl",
                }),
                memory_id: None,
                summary: "returned host simulator status".to_string(),
            })
        }
        "device.led.set" => {
            let color = input
                .get("color")
                .and_then(Value::as_str)
                .unwrap_or("green");
            Ok(ToolExecution {
                output: json!({
                    "accepted": true,
                    "simulated": true,
                    "color": color,
                }),
                memory_id: None,
                summary: format!("simulated LED color {color}"),
            })
        }
        "device.speaker.speak" => {
            let text = input.get("text").and_then(Value::as_str).unwrap_or("");
            Ok(ToolExecution {
                output: json!({
                    "accepted": true,
                    "simulated": true,
                    "text": text,
                }),
                memory_id: None,
                summary: "simulated speaker output".to_string(),
            })
        }
        "device.camera.capture" => {
            let analyze = input
                .get("analyze")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let prompt = input
                .get("prompt")
                .and_then(Value::as_str)
                .unwrap_or("Describe the captured scene for the Indwell user.")
                .to_string();
            let mut output = json!({
                "accepted": true,
                "simulated": true,
                "path": "data/host-sim/camera/latest.jpg",
                "width": 640,
                "height": 480,
                "mime_type": "image/jpeg",
                "byte_len": host_sim_camera_jpeg().len(),
                "retention": "temporary",
                "analyzed": analyze,
            });
            let mut summary = "simulated still image capture".to_string();
            if analyze {
                let providers = state.providers.lock().await.load()?;
                let vision = analyze_image_with_config(
                    state,
                    &providers,
                    VisionRequest {
                        image_bytes: host_sim_camera_jpeg().to_vec(),
                        mime_type: "image/jpeg".to_string(),
                        prompt: Some(prompt),
                    },
                )
                .await?;
                output["vision"] = json!({
                    "description": vision.description,
                    "provider": providers.vision.as_ref().map(|provider| json!({
                        "kind": provider.kind,
                        "model": provider.model,
                    })),
                });
                summary = "captured still image and analyzed it with vision provider".to_string();
            }
            Ok(ToolExecution {
                output,
                memory_id: None,
                summary,
            })
        }
        "device.sensor.read" => {
            let sensor = input
                .get("sensor")
                .and_then(Value::as_str)
                .unwrap_or("ambient_light");
            let value = match sensor {
                "temperature" => json!({ "celsius": 24.2 }),
                "imu" => json!({ "motion": "stable" }),
                "pressure" => json!({ "pressed": false }),
                _ => json!({ "lux": 320 }),
            };
            Ok(ToolExecution {
                output: json!({
                    "sensor": sensor,
                    "value": value,
                    "simulated": true,
                }),
                memory_id: None,
                summary: format!("read simulated sensor {sensor}"),
            })
        }
        "memory.search" => {
            let query = MemoryQuery {
                wing: input
                    .get("wing")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                room: input
                    .get("room")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                text: input
                    .get("text")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                limit: input
                    .get("limit")
                    .and_then(Value::as_u64)
                    .map(|limit| limit as usize),
            };
            let records = state.memory.lock().await.search(query)?;
            Ok(ToolExecution {
                output: json!({ "records": records }),
                memory_id: None,
                summary: "searched local memory".to_string(),
            })
        }
        "memory.write_candidate" => {
            let content = input
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or("empty candidate memory");
            let wing = input
                .get("wing")
                .and_then(Value::as_str)
                .unwrap_or("user_unknown");
            let room = input
                .get("room")
                .and_then(Value::as_str)
                .unwrap_or("episodes");
            let record = MemoryRecord::new(
                MemoryKind::Episodic,
                wing,
                room,
                content,
                MemorySource::DeviceEvent,
                Utc::now().timestamp_millis() as u64,
            );
            let memory_id = record.id.clone();
            state.memory.lock().await.append(record)?;
            Ok(ToolExecution {
                output: json!({
                    "accepted": true,
                    "memory_id": memory_id,
                }),
                memory_id: Some(memory_id),
                summary: "wrote candidate memory".to_string(),
            })
        }
        "memory.delete" => {
            let id = input.get("id").and_then(Value::as_str).unwrap_or("");
            if id.is_empty() {
                return Ok(ToolExecution {
                    output: json!({
                        "deleted": false,
                        "reason": "missing memory id",
                    }),
                    memory_id: None,
                    summary: "memory delete missing id".to_string(),
                });
            }
            state.memory.lock().await.delete(id)?;
            Ok(ToolExecution {
                output: json!({
                    "deleted": true,
                    "memory_id": id,
                }),
                memory_id: None,
                summary: format!("deleted memory {id}"),
            })
        }
        "identity.whoami" => Ok(ToolExecution {
            output: json!({
                "subject_id": "unknown",
                "owner_authenticated": false,
                "method": "none",
            }),
            memory_id: None,
            summary: "returned anonymous identity context".to_string(),
        }),
        "auth.request_confirmation" => Ok(ToolExecution {
            output: json!({
                "requested": true,
                "channels": ["local_pwa", "physical_button"],
                "challenge": "host-sim-confirm",
            }),
            memory_id: None,
            summary: "simulated human confirmation request".to_string(),
        }),
        "system.update.check" => Ok(ToolExecution {
            output: {
                let manifest = state.ota.lock().await.load()?;
                let report = state.ota.lock().await.verify("host-sim")?;
                json!({
                    "update_available": false,
                    "current_version": "0.1.0-host-sim",
                    "manifest": manifest,
                    "verification": report,
                })
            },
            memory_id: None,
            summary: "checked mock update manifest".to_string(),
        }),
        "system.update.apply" => {
            let ota = state.ota.lock().await;
            let manifest = ota.load()?;
            let trust = ota.load_trust_store()?;
            drop(ota);
            match indwell_ota::plan_trusted_ota_apply(
                &manifest,
                &trust,
                "host-sim",
                "0.1.0-host-sim",
                OtaSlot::Ota0,
            ) {
                Ok(plan) => Ok(ToolExecution {
                    output: json!({
                        "accepted": true,
                        "simulated": true,
                        "plan": plan,
                        "next_state": "updating",
                    }),
                    memory_id: None,
                    summary: "simulated verified OTA apply plan".to_string(),
                }),
                Err(error) => Ok(ToolExecution {
                    output: json!({
                        "accepted": false,
                        "simulated": true,
                        "reason": error.to_string(),
                        "manifest": manifest,
                        "next_state": "idle",
                    }),
                    memory_id: None,
                    summary: format!("OTA apply refused: {error}"),
                }),
            }
        }
        _ => Ok(ToolExecution {
            output: json!({
                "accepted": false,
                "simulated": true,
                "reason": "no mock executor implemented for this tool",
            }),
            memory_id: None,
            summary: "no mock executor implemented".to_string(),
        }),
    }
}

async fn channel_policies() -> Json<ApiEnvelope<ChannelPolicyResponse>> {
    let channels = [
        ChannelKind::LocalPwa,
        ChannelKind::Ble,
        ChannelKind::UsbSerial,
        ChannelKind::LanWebSocket,
        ChannelKind::Telegram,
        ChannelKind::Feishu,
        ChannelKind::Dingtalk,
        ChannelKind::WeCom,
        ChannelKind::WhatsApp,
        ChannelKind::Discord,
        ChannelKind::Matrix,
        ChannelKind::Mqtt,
        ChannelKind::HomeAssistant,
        ChannelKind::CustomWebhook,
    ];

    Json(ApiEnvelope::ok(ChannelPolicyResponse {
        policies: channels
            .into_iter()
            .map(ChannelPolicy::default_for)
            .collect(),
    }))
}

#[derive(Debug)]
enum AppError {
    Channel(indwell_channel::ChannelError),
    Memory(indwell_memory::MemoryError),
    Provider(indwell_provider::ProviderError),
    ProviderConfig(provider_config::ProviderConfigStoreError),
    Ota(ota::OtaManifestStoreError),
    RunStore(indwell_runs::RunStoreError),
    Security(indwell_security::SecurityError),
    Reflection(indwell_reflection::ReflectionError),
    Json(serde_json::Error),
    UnsupportedProviderKind(String),
    MemoryNotFound(String),
    InvalidHex,
    UnknownPairedDevice,
    MissingSessionToken,
}

impl From<indwell_channel::ChannelError> for AppError {
    fn from(err: indwell_channel::ChannelError) -> Self {
        Self::Channel(err)
    }
}

impl From<indwell_memory::MemoryError> for AppError {
    fn from(err: indwell_memory::MemoryError) -> Self {
        Self::Memory(err)
    }
}

impl From<indwell_provider::ProviderError> for AppError {
    fn from(err: indwell_provider::ProviderError) -> Self {
        Self::Provider(err)
    }
}

impl From<provider_config::ProviderConfigStoreError> for AppError {
    fn from(err: provider_config::ProviderConfigStoreError) -> Self {
        Self::ProviderConfig(err)
    }
}

impl From<ota::OtaManifestStoreError> for AppError {
    fn from(err: ota::OtaManifestStoreError) -> Self {
        Self::Ota(err)
    }
}

impl From<indwell_runs::RunStoreError> for AppError {
    fn from(err: indwell_runs::RunStoreError) -> Self {
        Self::RunStore(err)
    }
}

impl From<indwell_security::SecurityError> for AppError {
    fn from(err: indwell_security::SecurityError) -> Self {
        Self::Security(err)
    }
}

impl From<indwell_reflection::ReflectionError> for AppError {
    fn from(err: indwell_reflection::ReflectionError) -> Self {
        Self::Reflection(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let message = match self {
            Self::Channel(err) => err.to_string(),
            Self::Memory(err) => err.to_string(),
            Self::Provider(err) => err.to_string(),
            Self::ProviderConfig(err) => err.to_string(),
            Self::Ota(err) => err.to_string(),
            Self::RunStore(err) => err.to_string(),
            Self::Security(err) => err.to_string(),
            Self::Reflection(err) => err.to_string(),
            Self::Json(err) => err.to_string(),
            Self::UnsupportedProviderKind(kind) => format!("unsupported provider kind: {kind}"),
            Self::MemoryNotFound(id) => format!("memory not found: {id}"),
            Self::InvalidHex => "invalid hex input".to_string(),
            Self::UnknownPairedDevice => "paired device not found".to_string(),
            Self::MissingSessionToken => "missing session token".to_string(),
        };
        (
            StatusCode::BAD_REQUEST,
            Json(ApiEnvelope::<()>::err(message)),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{to_bytes, Body},
        http::{header, HeaderMap, HeaderValue, Request, StatusCode},
    };
    use ed25519_dalek::{Signer, SigningKey};
    use indwell_channel::{ChannelInbound, ChannelKind, ChannelPrincipal};
    use indwell_memory::{MemoryQuery, MemoryRecord, MemoryStore};
    use indwell_ota::{manifest_signature_payload, OtaManifest, OtaTrustStore};
    use indwell_protocol::{MobileCommand, ProviderConfig, ProviderConfigSet};
    use indwell_runs::RunStore;
    use indwell_security::{
        pairing_signature_payload, signed_request_payload, PairedDevice, SignedRequest,
    };
    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};
    use tower::ServiceExt;

    use super::{
        api_key_env_name, auth_context_for_inbound, build_router, command_prompt,
        decode_hex_or_utf8, execute_mock_tool, execute_provider_tool_calls, init_state,
        persist_confirmation_grants, provider_config_with_llm_fallback, session_token_from_headers,
    };

    #[test]
    fn api_key_ref_maps_to_stable_secret_env_name() {
        assert_eq!(
            api_key_env_name("key_llm-main"),
            "INDWELL_SECRET_KEY_LLM_MAIN"
        );
    }

    #[test]
    fn mobile_command_maps_to_tool_planning_prompt() {
        assert_eq!(
            command_prompt(&MobileCommand::CaptureImage),
            "capture camera image"
        );
        assert_eq!(
            command_prompt(&MobileCommand::SystemStatus),
            "system status"
        );
    }

    #[test]
    fn voice_provider_config_can_inherit_llm_connection_details() {
        let llm = ProviderConfig {
            kind: "openai_compatible".to_string(),
            base_url: Some("https://api.example.com/v1".to_string()),
            api_key_ref: Some("key_llm_main".to_string()),
            model: "llm-model".to_string(),
            max_input_tokens: Some(4000),
            max_output_tokens: Some(600),
        };
        let asr = ProviderConfig {
            kind: "same_as_llm".to_string(),
            base_url: None,
            api_key_ref: None,
            model: "whisper-compatible".to_string(),
            max_input_tokens: None,
            max_output_tokens: None,
        };

        let resolved = provider_config_with_llm_fallback(&asr, &llm);

        assert_eq!(resolved.kind, "openai_compatible");
        assert_eq!(
            resolved.base_url.as_deref(),
            Some("https://api.example.com/v1")
        );
        assert_eq!(resolved.api_key_ref.as_deref(), Some("key_llm_main"));
        assert_eq!(resolved.model, "whisper-compatible");
    }

    #[tokio::test]
    async fn provider_diagnostics_require_session() {
        let root = temp_root("provider-diagnostics-auth");
        let _ = std::fs::remove_dir_all(&root);
        let state = init_state(root.clone()).unwrap();
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/providers/test")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({ "target": "llm" }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn provider_diagnostics_cover_mock_provider_slots() {
        let root = temp_root("provider-diagnostics-mock");
        let _ = std::fs::remove_dir_all(&root);
        let state = init_state(root.clone()).unwrap();
        state
            .providers
            .lock()
            .await
            .save(&mock_all_provider_config())
            .unwrap();
        let token = issue_test_session(&state).await;
        let app = build_router(state);

        for target in ["llm", "vision", "asr", "tts", "embedding"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/providers/test")
                        .header(header::CONTENT_TYPE, "application/json")
                        .header(header::AUTHORIZATION, format!("Bearer {token}"))
                        .body(Body::from(json!({ "target": target }).to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);
            let data = response_data(response).await;
            assert_eq!(data["target"], target);
            assert_eq!(data["ok"], true);
            assert!(data["summary"].as_str().unwrap().contains("responded"));
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn provider_diagnostics_return_structured_failures() {
        let root = temp_root("provider-diagnostics-failure");
        let _ = std::fs::remove_dir_all(&root);
        let state = init_state(root.clone()).unwrap();
        let mut config = external_provider_config();
        config.llm.api_key_ref = Some("missing_provider_diagnostic_key".to_string());
        state.providers.lock().await.save(&config).unwrap();
        let token = issue_test_session(&state).await;
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/providers/test")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::from(json!({ "target": "llm" }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let data = response_data(response).await;
        assert_eq!(data["target"], "llm");
        assert_eq!(data["ok"], false);
        assert!(data["details"]["error"]
            .as_str()
            .unwrap()
            .contains("MissingApiKey"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn pairing_key_inputs_accept_hex_or_utf8_for_host_sim() {
        assert_eq!(decode_hex_or_utf8("01020a").unwrap(), vec![1, 2, 10]);
        assert_eq!(
            decode_hex_or_utf8("phone-public-key").unwrap(),
            b"phone-public-key"
        );
    }

    #[test]
    fn subject_hint_no_longer_grants_owner_auth() {
        let inbound = ChannelInbound {
            channel: ChannelKind::LocalPwa,
            session_id: "session-1".to_string(),
            subject_hint: Some("owner".to_string()),
            principal: None,
            text: Some("delete memory".to_string()),
            command: None,
        };

        let auth = auth_context_for_inbound(&inbound);

        assert!(!auth.owner_authenticated);
        assert!(auth.subject_id.is_none());
    }

    #[test]
    fn verified_channel_principal_grants_owner_auth() {
        let inbound = ChannelInbound {
            channel: ChannelKind::LocalPwa,
            session_id: "session-1".to_string(),
            subject_hint: None,
            principal: Some(ChannelPrincipal {
                subject_id: "owner".to_string(),
                owner_authenticated: true,
            }),
            text: Some("delete memory".to_string()),
            command: None,
        };

        let auth = auth_context_for_inbound(&inbound);

        assert!(auth.owner_authenticated);
        assert_eq!(auth.subject_id.as_deref(), Some("owner"));
    }

    #[test]
    fn session_token_can_be_read_from_authorization_or_custom_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer session-token"),
        );
        assert_eq!(
            session_token_from_headers(&headers).as_deref(),
            Some("session-token")
        );

        let mut headers = HeaderMap::new();
        headers.insert(
            "x-indwell-session-token",
            HeaderValue::from_static("session-token-2"),
        );
        assert_eq!(
            session_token_from_headers(&headers).as_deref(),
            Some("session-token-2")
        );
    }

    #[tokio::test]
    async fn public_health_route_does_not_require_session() {
        let root = temp_root("health");
        let _ = std::fs::remove_dir_all(&root);
        let app = build_router(init_state(root.clone()).unwrap());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn protected_memory_route_requires_session() {
        let root = temp_root("protected-memory");
        let _ = std::fs::remove_dir_all(&root);
        let app = build_router(init_state(root.clone()).unwrap());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/memory/export")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn signed_pairing_can_issue_session_and_call_protected_route() {
        let root = temp_root("signed-session");
        let _ = std::fs::remove_dir_all(&root);
        let app = build_router(init_state(root.clone()).unwrap());
        let signing_key = SigningKey::from_bytes(&[42_u8; 32]);
        let public_key = signing_key.verifying_key().to_bytes();

        let challenge_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/pairing/challenge")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(challenge_response.status(), StatusCode::OK);
        let challenge = response_data(challenge_response).await;
        let session_id = challenge["session_id"].as_str().unwrap();
        let code = challenge["code"].as_str().unwrap();
        let label = "Test browser";
        let pairing_signature = signing_key
            .sign(pairing_signature_payload(session_id, code, label, &public_key).as_bytes())
            .to_bytes();

        let paired_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/pairing/complete")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "session_id": session_id,
                            "code": code,
                            "label": label,
                            "public_key": hex_bytes(&public_key),
                            "signature": hex_bytes(&pairing_signature),
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(paired_response.status(), StatusCode::OK);
        let paired = response_data(paired_response).await;
        let device_id = paired["device_id"].as_str().unwrap();

        let signed = SignedRequest {
            device_id: device_id.to_string(),
            timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
            nonce: "nonce-1".to_string(),
            method: "POST".to_string(),
            path: "/v1/auth/session".to_string(),
            body_sha256: hex_bytes(&Sha256::digest([])),
        };
        let request_signature = signing_key
            .sign(signed_request_payload(&signed).as_bytes())
            .to_bytes();

        let session_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/auth/session")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "device_id": signed.device_id,
                            "subject_id": "owner",
                            "timestamp_ms": signed.timestamp_ms,
                            "nonce": signed.nonce,
                            "method": signed.method,
                            "path": signed.path,
                            "body_sha256": signed.body_sha256,
                            "signature": hex_bytes(&request_signature),
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(session_response.status(), StatusCode::OK);
        let session = response_data(session_response).await;
        let token = session["token"].as_str().unwrap();

        let providers_response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/providers")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(providers_response.status(), StatusCode::OK);
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn unauthenticated_channel_input_uses_mock_provider_and_quarantines_memory() {
        let root = temp_root("public-ingress");
        let _ = std::fs::remove_dir_all(&root);
        let state = init_state(root.clone()).unwrap();
        state
            .providers
            .lock()
            .await
            .save(&external_provider_config())
            .unwrap();
        let app = build_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/channel/input")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "channel": "local_pwa",
                            "session_id": "public-test",
                            "subject_hint": "owner",
                            "text": "remember my quiet morning preference",
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let data = response_data(response).await;
        assert!(data["reply"]
            .as_str()
            .unwrap()
            .starts_with("Indwell mock response"));

        let records = state
            .memory
            .lock()
            .await
            .search(MemoryQuery {
                wing: Some("inbox".to_string()),
                room: Some("unverified".to_string()),
                text: Some("quiet morning".to_string()),
                limit: Some(10),
            })
            .unwrap();
        assert_eq!(records.len(), 1);
        assert!(records[0]
            .tags
            .iter()
            .any(|tag| tag == "unverified_ingress"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn provider_failure_is_persisted_as_failed_run_checkpoint() {
        let root = temp_root("provider-failed-run");
        let _ = std::fs::remove_dir_all(&root);
        let state = init_state(root.clone()).unwrap();
        let mut config = external_provider_config();
        config.llm.api_key_ref = Some("missing_failed_run_key".to_string());
        state.providers.lock().await.save(&config).unwrap();
        let token = issue_test_session(&state).await;
        let app = build_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/channel/input")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "channel": "local_pwa",
                            "session_id": "provider-failed-run",
                            "text": "hello with broken provider",
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let runs = state.runs.lock().await.list().unwrap();
        let run = runs
            .iter()
            .find(|run| run.user_intent.as_deref() == Some("hello with broken provider"))
            .expect("failed run should be persisted");
        assert_eq!(run.status, indwell_core::RunStatus::Failed);
        assert!(run
            .audit
            .failure_reason
            .as_deref()
            .unwrap()
            .contains("llm "));
        let entries = state.runs.lock().await.entries_for_run(run.id).unwrap();
        assert_eq!(
            entries.last().map(|entry| entry.stage.as_str()),
            Some("failed")
        );
        assert_eq!(
            entries.last().map(|entry| entry.status.clone()),
            Some(indwell_core::RunStatus::Failed)
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn provider_tool_call_executes_and_is_audited() {
        let root = temp_root("provider-tool-call");
        let _ = std::fs::remove_dir_all(&root);
        let state = init_state(root.clone()).unwrap();
        let app = build_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/channel/input")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "channel": "local_pwa",
                            "session_id": "provider-tool-test",
                            "subject_hint": null,
                            "text": "remember that I like quiet blue lights",
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let data = response_data(response).await;
        let run_id = data["run_id"].as_str().unwrap();
        let run = state
            .runs
            .lock()
            .await
            .get(uuid::Uuid::parse_str(run_id).unwrap())
            .unwrap()
            .expect("run should be stored");
        assert!(run
            .audit
            .tool_calls
            .iter()
            .any(|call| call.tool == "memory.write_candidate"
                && call.summary.starts_with("provider tool mock-memory-write")));
        assert!(!run.audit.written_memory_ids.is_empty());
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn provider_tool_call_cannot_bypass_high_risk_confirmation() {
        let root = temp_root("provider-high-risk-block");
        let _ = std::fs::remove_dir_all(&root);
        let state = init_state(root.clone()).unwrap();
        let mut run = indwell_core::AgentRun::new(
            indwell_core::Event::ChannelMessage {
                channel: "local_pwa".to_string(),
                session_id: "provider-high-risk-test".to_string(),
            },
            Some("apply update".to_string()),
            indwell_core::AuthContext::owner("owner", vec![indwell_core::AuthMethod::PairedDevice]),
            indwell_core::DeviceState::Thinking,
            indwell_core::ProviderSelection {
                llm: "mock:phase0".to_string(),
                vision: None,
                asr: None,
                tts: None,
                embedding: None,
            },
            chrono::Utc::now().timestamp_millis() as u64,
        );
        run.allowed_tools
            .push(crate::tools::lookup_tool("system.update.apply"));

        execute_provider_tool_calls(
            &state,
            &mut run,
            vec![indwell_provider::ToolCall {
                id: "provider-ota-apply".to_string(),
                name: "system.update.apply".to_string(),
                arguments: json!({}),
            }],
        )
        .await;

        assert!(run.audit.tool_calls.iter().any(|call| {
            call.tool == "system.update.apply"
                && call.outcome == indwell_core::ToolAuditOutcome::Blocked
                && call.summary.contains("requires explicit confirmation")
        }));
        assert!(run.audit.policy_blocks.iter().any(|block| {
            block.contains("system.update.apply")
                && block.contains("requires explicit confirmation")
        }));
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn manual_memory_can_be_deleted_through_tool_runtime() {
        let root = temp_root("memory-delete-tool");
        let _ = std::fs::remove_dir_all(&root);
        let state = init_state(root.clone()).unwrap();
        let record = MemoryRecord::new(
            indwell_memory::MemoryKind::Preference,
            "user_unknown",
            "preferences",
            "User likes deliberate memory review.",
            indwell_memory::MemorySource::Manual,
            chrono::Utc::now().timestamp_millis() as u64,
        );
        let memory_id = record.id.clone();
        state.memory.lock().await.append(record).unwrap();

        let execution = execute_mock_tool(
            &state,
            "memory.delete",
            json!({
                "id": memory_id,
            }),
        )
        .await
        .unwrap();

        assert_eq!(execution.output["deleted"], true);
        let records = state
            .memory
            .lock()
            .await
            .search(MemoryQuery {
                wing: Some("user_unknown".to_string()),
                room: Some("preferences".to_string()),
                text: Some("deliberate memory review".to_string()),
                limit: Some(10),
            })
            .unwrap();
        assert!(records.is_empty());
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn protected_tool_execute_uses_authorization_session_without_body_token() {
        let root = temp_root("tool-execute-header-session");
        let _ = std::fs::remove_dir_all(&root);
        let state = init_state(root.clone()).unwrap();
        let token = issue_test_session(&state).await;
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tools/memory.delete/execute")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "channel": "local_pwa",
                            "input": {}
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let data = response_data(response).await;
        assert_eq!(data["decision"], "allow");
        assert_eq!(data["output"]["deleted"], false);
        assert_eq!(data["output"]["reason"], "missing memory id");
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn camera_capture_can_analyze_with_vision_provider() {
        let root = temp_root("camera-vision-tool");
        let _ = std::fs::remove_dir_all(&root);
        let state = init_state(root.clone()).unwrap();
        state
            .providers
            .lock()
            .await
            .save(&mock_all_provider_config())
            .unwrap();

        let execution = execute_mock_tool(
            &state,
            "device.camera.capture",
            json!({
                "analyze": true,
                "prompt": "Describe the host simulator camera fixture."
            }),
        )
        .await
        .unwrap();

        assert_eq!(execution.output["accepted"], true);
        assert_eq!(execution.output["analyzed"], true);
        assert!(execution.output["vision"]["description"]
            .as_str()
            .unwrap()
            .contains("Mock vision saw"));
        assert_eq!(execution.output["vision"]["provider"]["kind"], "mock");
        assert!(execution.summary.contains("vision provider"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn ota_apply_requires_confirmation_and_returns_structured_result() {
        let root = temp_root("ota-apply-confirmation");
        let _ = std::fs::remove_dir_all(&root);
        let state = init_state(root.clone()).unwrap();
        let token = issue_test_session(&state).await;
        let app = build_router(state.clone());

        let blocked_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tools/system.update.apply/execute")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::from(
                        json!({
                            "channel": "local_pwa",
                            "session_token": token,
                            "input": {}
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(blocked_response.status(), StatusCode::OK);
        let blocked = response_data(blocked_response).await;
        assert_eq!(blocked["decision"], "require_confirmation");

        let grant = state.grants.lock().await.issue(
            "owner",
            "system.update.apply",
            chrono::Utc::now().timestamp_millis() as u64,
            60_000,
        );
        let token = issue_test_session(&state).await;
        let applied_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tools/system.update.apply/execute")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::from(
                        json!({
                            "channel": "local_pwa",
                            "session_token": token,
                            "confirmation_grant_id": grant.grant_id,
                            "input": {}
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(applied_response.status(), StatusCode::OK);
        let applied = response_data(applied_response).await;
        assert_eq!(applied["decision"], "allow");
        assert_eq!(applied["output"]["accepted"], false);
        assert!(applied["output"]["reason"]
            .as_str()
            .unwrap()
            .contains("trusted_signature"));
        let run_id = applied["run_id"].as_str().unwrap();
        let run = state
            .runs
            .lock()
            .await
            .get(uuid::Uuid::parse_str(run_id).unwrap())
            .unwrap()
            .expect("run should be stored");
        assert!(run.audit.tool_calls.iter().any(|call| {
            call.tool == "system.update.apply" && call.summary.contains("OTA apply refused")
        }));
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn ota_apply_accepts_signed_manifest_from_trust_store() {
        let root = temp_root("ota-apply-trusted");
        let _ = std::fs::remove_dir_all(&root);
        let state = init_state(root.clone()).unwrap();
        let signing_key = SigningKey::from_bytes(&[31_u8; 32]);
        let verifying_key = signing_key.verifying_key();
        let mut manifest = OtaManifest::host_sim_default();
        manifest.version = "0.1.1-host-sim".to_string();
        manifest.signature = hex_bytes(
            signing_key
                .sign(manifest_signature_payload(&manifest).as_bytes())
                .to_bytes(),
        );
        {
            let ota = state.ota.lock().await;
            ota.save(&manifest).unwrap();
            ota.save_trust_store(&OtaTrustStore::with_keys([hex_bytes(
                verifying_key.as_bytes(),
            )]))
            .unwrap();
        }
        let grant = state.grants.lock().await.issue(
            "owner",
            "system.update.apply",
            chrono::Utc::now().timestamp_millis() as u64,
            60_000,
        );
        persist_confirmation_grants(&state).await.unwrap();
        let token = issue_test_session(&state).await;
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tools/system.update.apply/execute")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::from(
                        json!({
                            "channel": "local_pwa",
                            "confirmation_grant_id": grant.grant_id,
                            "input": {}
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let data = response_data(response).await;
        assert_eq!(data["decision"], "allow");
        assert_eq!(data["output"]["accepted"], true);
        assert_eq!(data["output"]["plan"]["version"], "0.1.1-host-sim");
        assert_eq!(data["output"]["plan"]["to_slot"], "ota1");

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn consumed_confirmation_grant_survives_host_restart() {
        let root = temp_root("confirmation-grant-restart");
        let _ = std::fs::remove_dir_all(&root);
        let state = init_state(root.clone()).unwrap();
        let token = issue_test_session(&state).await;
        let app = build_router(state.clone());

        let challenge_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/auth/passphrase/challenge")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(challenge_response.status(), StatusCode::OK);
        let challenge = response_data(challenge_response).await;

        let verify_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/auth/passphrase/verify")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "challenge_id": challenge["challenge_id"],
                            "spoken_phrase": challenge["phrase"],
                            "subject_id": "owner",
                            "allowed_tool": "system.update.apply"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(verify_response.status(), StatusCode::OK);
        let grant_id = response_data(verify_response).await["grant"]["grant_id"]
            .as_str()
            .unwrap()
            .to_string();

        let first_apply = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tools/system.update.apply/execute")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::from(
                        json!({
                            "channel": "local_pwa",
                            "confirmation_grant_id": grant_id.clone(),
                            "input": {}
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(first_apply.status(), StatusCode::OK);
        assert_eq!(response_data(first_apply).await["decision"], "allow");

        let restarted = init_state(root.clone()).unwrap();
        let restarted_token = issue_test_session(&restarted).await;
        let restarted_app = build_router(restarted);
        let replay_response = restarted_app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tools/system.update.apply/execute")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {restarted_token}"))
                    .body(Body::from(
                        json!({
                            "channel": "local_pwa",
                            "confirmation_grant_id": grant_id,
                            "input": {}
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(replay_response.status(), StatusCode::OK);
        let replay = response_data(replay_response).await;
        assert_eq!(replay["decision"], "require_confirmation");

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn inbox_memory_can_be_audited_and_accepted() {
        let root = temp_root("memory-inbox-review");
        let _ = std::fs::remove_dir_all(&root);
        let state = init_state(root.clone()).unwrap();
        let mut record = MemoryRecord::new(
            indwell_memory::MemoryKind::Episodic,
            "inbox",
            "unverified",
            "public channel said the owner likes quiet evenings",
            indwell_memory::MemorySource::AgentRun {
                run_id: "run-inbox-review".to_string(),
            },
            chrono::Utc::now().timestamp_millis() as u64,
        );
        record.tags.push("unverified_ingress".to_string());
        record.confidence = 0.35;
        let memory_id = record.id.clone();
        state.memory.lock().await.append(record).unwrap();
        let token = issue_test_session(&state).await;
        let app = build_router(state.clone());

        let audit_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/memory/{memory_id}/audit"))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(audit_response.status(), StatusCode::OK);
        let audit = response_data(audit_response).await;
        assert_eq!(audit["status"], "unverified");
        assert_eq!(audit["related_run_id"], "run-inbox-review");

        let accept_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/memory/{memory_id}/accept"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::from(
                        json!({
                            "wing": "user_unknown",
                            "room": "episodes",
                            "confidence": 0.8,
                            "importance": 0.5
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(accept_response.status(), StatusCode::OK);
        let accepted = response_data(accept_response).await;
        assert_eq!(accepted["accepted"], true);
        assert_eq!(accepted["record"]["wing"], "user_unknown");
        assert_eq!(accepted["record"]["room"], "episodes");

        let inbox_records = state
            .memory
            .lock()
            .await
            .search(MemoryQuery {
                wing: Some("inbox".to_string()),
                room: Some("unverified".to_string()),
                text: Some("quiet evenings".to_string()),
                limit: Some(10),
            })
            .unwrap();
        assert!(inbox_records.is_empty());

        let accepted_records = state
            .memory
            .lock()
            .await
            .search(MemoryQuery {
                wing: Some("user_unknown".to_string()),
                room: Some("episodes".to_string()),
                text: Some("quiet evenings".to_string()),
                limit: Some(10),
            })
            .unwrap();
        assert_eq!(accepted_records.len(), 1);
        assert!(!accepted_records[0]
            .tags
            .iter()
            .any(|tag| tag == "unverified_ingress"));
        assert!(accepted_records[0]
            .tags
            .iter()
            .any(|tag| tag == "reviewed_by_owner"));
        assert!(accepted_records[0]
            .tags
            .iter()
            .any(|tag| tag == "accepted_from_inbox"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn channel_input_accepts_bearer_session_for_owner_memory() {
        let root = temp_root("authenticated-channel");
        let _ = std::fs::remove_dir_all(&root);
        let state = init_state(root.clone()).unwrap();
        let device = test_paired_device();
        let (_session, token) = state
            .sessions
            .lock()
            .await
            .issue(
                &device,
                "owner",
                chrono::Utc::now().timestamp_millis() as u64,
                60_000,
            )
            .unwrap();
        let app = build_router(state.clone());

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/channel/input")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "channel": "local_pwa",
                            "session_id": "authenticated-test",
                            "subject_hint": "owner",
                            "text": "remember I like verified mornings",
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let data = response_data(response).await;
        let run_id = data["run_id"].as_str().unwrap();
        let entries_response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/runs/{run_id}/entries"))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(entries_response.status(), StatusCode::OK);
        let entries = response_data(entries_response).await;
        let stages = entries
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["stage"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert!(stages.starts_with(&["created", "context", "provider", "tool"]));
        assert_eq!(stages.last().copied(), Some("completed"));

        let records = state
            .memory
            .lock()
            .await
            .search(MemoryQuery {
                wing: Some("user_unknown".to_string()),
                room: Some("episodes".to_string()),
                text: Some("verified mornings".to_string()),
                limit: Some(10),
            })
            .unwrap();
        assert!(!records.is_empty());
        assert!(records
            .iter()
            .all(|record| !record.tags.iter().any(|tag| tag == "unverified_ingress")));
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn owner_look_request_runs_camera_vision_and_audits_tool_call() {
        let root = temp_root("owner-look-camera-vision");
        let _ = std::fs::remove_dir_all(&root);
        let state = init_state(root.clone()).unwrap();
        state
            .providers
            .lock()
            .await
            .save(&mock_all_provider_config())
            .unwrap();
        let token = issue_test_session(&state).await;
        let app = build_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/channel/input")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::from(
                        json!({
                            "channel": "local_pwa",
                            "session_id": "owner-look-test",
                            "subject_hint": "owner",
                            "text": "看看桌面上有什么",
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let data = response_data(response).await;
        let run_id = data["run_id"].as_str().unwrap();
        let run = state
            .runs
            .lock()
            .await
            .get(uuid::Uuid::parse_str(run_id).unwrap())
            .unwrap()
            .expect("run should be stored");
        assert!(run.audit.tool_calls.iter().any(|call| {
            call.tool == "device.camera.capture"
                && call.summary.contains("analyzed it with vision provider")
        }));
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn public_voice_mock_turn_does_not_call_external_provider() {
        let root = temp_root("public-voice");
        let _ = std::fs::remove_dir_all(&root);
        let state = init_state(root.clone()).unwrap();
        state
            .providers
            .lock()
            .await
            .save(&external_provider_config())
            .unwrap();
        let app = build_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/voice/mock-turn")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "text_hint": "hello from public voice",
                            "voice": "warm_indwell",
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let data = response_data(response).await;
        assert!(data["reply"]
            .as_str()
            .unwrap()
            .starts_with("Indwell mock response"));
        assert_eq!(data["audio"]["mime_type"].as_str(), Some("audio/wav"));
        let run_id = data["run_id"].as_str().unwrap();
        let run = state
            .runs
            .lock()
            .await
            .get(uuid::Uuid::parse_str(run_id).unwrap())
            .unwrap()
            .expect("voice turn run should be stored");
        assert!(run
            .audit
            .input_summary
            .as_deref()
            .unwrap()
            .starts_with("mock transcript for"));
        assert!(run.context_pack.persona_snapshot.is_some());
        assert_eq!(run.provider.llm, "mock:mock:phase0");
        let _ = std::fs::remove_dir_all(root);
    }

    fn temp_root(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("indwell-host-sim-{name}-{}", uuid::Uuid::new_v4()))
    }

    async fn response_data(response: axum::response::Response) -> Value {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice::<Value>(&bytes).unwrap()["data"].clone()
    }

    fn hex_bytes(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn external_provider_config() -> ProviderConfigSet {
        ProviderConfigSet {
            llm: ProviderConfig {
                kind: "openai_compatible".to_string(),
                base_url: Some("https://api.example.invalid/v1".to_string()),
                api_key_ref: Some("key_llm_main".to_string()),
                model: "external-model".to_string(),
                max_input_tokens: Some(4000),
                max_output_tokens: Some(600),
            },
            vision: None,
            asr: None,
            tts: None,
            embedding: None,
        }
    }

    fn mock_all_provider_config() -> ProviderConfigSet {
        ProviderConfigSet {
            llm: ProviderConfig {
                kind: "mock".to_string(),
                base_url: None,
                api_key_ref: None,
                model: "mock:phase0".to_string(),
                max_input_tokens: Some(4000),
                max_output_tokens: Some(600),
            },
            vision: Some(mock_optional_provider("mock:vision")),
            asr: Some(mock_optional_provider("mock:asr")),
            tts: Some(mock_optional_provider("mock:tts")),
            embedding: Some(mock_optional_provider("mock:embedding")),
        }
    }

    fn mock_optional_provider(model: &str) -> ProviderConfig {
        ProviderConfig {
            kind: "mock".to_string(),
            base_url: None,
            api_key_ref: None,
            model: model.to_string(),
            max_input_tokens: None,
            max_output_tokens: None,
        }
    }

    async fn issue_test_session(state: &super::AppState) -> String {
        let device = test_paired_device();
        let (_session, token) = state
            .sessions
            .lock()
            .await
            .issue(
                &device,
                "owner",
                chrono::Utc::now().timestamp_millis() as u64,
                60_000,
            )
            .unwrap();
        token
    }

    fn test_paired_device() -> PairedDevice {
        PairedDevice {
            device_id: "test-device".to_string(),
            label: "Test device".to_string(),
            public_key_hash: "test-key".to_string(),
            public_key: vec![9_u8; 32],
            paired_at_ms: 1,
            last_seen_at_ms: Some(1),
            revoked_at_ms: None,
        }
    }
}
