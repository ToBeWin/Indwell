use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: DeviceState,
    pub to: DeviceState,
}

impl DeviceState {
    pub fn can_transition_to(self, next: DeviceState) -> bool {
        use DeviceState::{
            Authenticating, Booting, Error, Idle, Listening, Observing, Provisioning, Sleep,
            Speaking, Thinking, Updating,
        };

        matches!(
            (self, next),
            (Booting, Provisioning)
                | (Booting, Idle)
                | (Provisioning, Idle)
                | (Idle, Listening)
                | (Idle, Observing)
                | (Idle, Updating)
                | (Idle, Sleep)
                | (Listening, Authenticating)
                | (Listening, Thinking)
                | (Authenticating, Thinking)
                | (Authenticating, Idle)
                | (Thinking, Speaking)
                | (Thinking, Observing)
                | (Thinking, Idle)
                | (Observing, Thinking)
                | (Observing, Idle)
                | (Speaking, Idle)
                | (Updating, Idle)
                | (Sleep, Idle)
                | (_, Error)
                | (Error, Idle)
                | (Error, Provisioning)
        )
    }

    pub fn transition_to(self, next: DeviceState) -> Result<StateTransition, StateTransitionError> {
        if self.can_transition_to(next) {
            Ok(StateTransition {
                from: self,
                to: next,
            })
        } else {
            Err(StateTransitionError {
                from: self,
                to: next,
            })
        }
    }

    pub fn next_for_event(self, event: &Event) -> Option<DeviceState> {
        let next = match event {
            Event::BootCompleted => DeviceState::Idle,
            Event::ButtonPressed { .. } | Event::WakeWordDetected { .. } => DeviceState::Listening,
            Event::VadSpeechStarted => DeviceState::Listening,
            Event::VadSpeechEnded | Event::AudioCaptured { .. } => DeviceState::Thinking,
            Event::ImageCaptured { .. } => DeviceState::Thinking,
            Event::ChannelMessage { .. } => DeviceState::Thinking,
            Event::AuthPassed { .. } => DeviceState::Thinking,
            Event::AuthFailed { .. } => DeviceState::Idle,
            Event::ProviderResponse { .. } => DeviceState::Speaking,
            Event::ToolCallRequested { tool, .. } if tool == "device.camera.capture" => {
                DeviceState::Observing
            }
            Event::ToolCallRequested { .. } => DeviceState::Thinking,
            Event::ToolCallCompleted { .. } => DeviceState::Thinking,
            Event::MemoryWriteRequested { .. } => DeviceState::Thinking,
            Event::OtaUpdateAvailable { .. } => DeviceState::Updating,
            Event::Error { .. } => DeviceState::Error,
        };

        self.can_transition_to(next).then_some(next)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateTransitionError {
    pub from: DeviceState,
    pub to: DeviceState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Safe,
    Low,
    Medium,
    High,
    Forbidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    None,
    Passphrase,
    PairedDevice,
    PhysicalButton,
    Voiceprint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthContext {
    pub subject_id: Option<String>,
    pub methods: Vec<AuthMethod>,
    pub owner_authenticated: bool,
}

impl AuthContext {
    pub fn anonymous() -> Self {
        Self {
            subject_id: None,
            methods: vec![AuthMethod::None],
            owner_authenticated: false,
        }
    }

    pub fn owner(subject_id: impl Into<String>, methods: Vec<AuthMethod>) -> Self {
        Self {
            subject_id: Some(subject_id.into()),
            methods,
            owner_authenticated: true,
        }
    }

    pub fn has_strong_factor(&self) -> bool {
        self.methods.iter().any(|method| {
            matches!(
                method,
                AuthMethod::Passphrase | AuthMethod::PairedDevice | AuthMethod::PhysicalButton
            )
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    pub risk: RiskLevel,
    pub requires_owner: bool,
    pub requires_confirmation: bool,
}

impl ToolDescriptor {
    pub fn new(name: impl Into<String>, description: impl Into<String>, risk: RiskLevel) -> Self {
        let risk = risk;
        Self {
            name: name.into(),
            description: description.into(),
            risk,
            requires_owner: risk >= RiskLevel::Medium,
            requires_confirmation: risk >= RiskLevel::High,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Created,
    AssemblingContext,
    WaitingForProvider,
    WaitingForTool,
    Completed,
    BlockedByPolicy,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPack {
    pub persona_snapshot: Option<String>,
    pub current_device_state: DeviceState,
    pub recent_turns: Vec<String>,
    pub retrieved_memories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSelection {
    pub llm: String,
    pub vision: Option<String>,
    pub asr: Option<String>,
    pub tts: Option<String>,
    pub embedding: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentRunAudit {
    pub input_summary: Option<String>,
    pub retrieved_memory_ids: Vec<String>,
    pub written_memory_ids: Vec<String>,
    pub tool_calls: Vec<ToolAuditRecord>,
    pub provider_output_summary: Option<String>,
    pub policy_blocks: Vec<String>,
    pub failure_reason: Option<String>,
    pub completed_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolAuditRecord {
    pub tool: String,
    pub outcome: ToolAuditOutcome,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolAuditOutcome {
    Requested,
    Completed,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    BootCompleted,
    ButtonPressed {
        duration_ms: u32,
    },
    WakeWordDetected {
        score: f32,
    },
    VadSpeechStarted,
    VadSpeechEnded,
    AudioCaptured {
        path: String,
        duration_ms: u32,
    },
    ImageCaptured {
        path: String,
        width: u16,
        height: u16,
    },
    ChannelMessage {
        channel: String,
        session_id: String,
    },
    AuthPassed {
        subject_id: String,
        method: AuthMethod,
    },
    AuthFailed {
        reason: String,
    },
    ProviderResponse {
        run_id: String,
    },
    ToolCallRequested {
        run_id: String,
        tool: String,
    },
    ToolCallCompleted {
        run_id: String,
        tool: String,
    },
    MemoryWriteRequested {
        record_id: String,
    },
    OtaUpdateAvailable {
        version: String,
    },
    Error {
        code: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRun {
    pub id: Uuid,
    pub trigger: Event,
    pub user_intent: Option<String>,
    pub auth_context: AuthContext,
    pub context_pack: ContextPack,
    pub allowed_tools: Vec<ToolDescriptor>,
    pub provider: ProviderSelection,
    pub status: RunStatus,
    pub created_at_ms: u64,
    pub audit: AgentRunAudit,
}

impl AgentRun {
    pub fn new(
        trigger: Event,
        user_intent: Option<String>,
        auth_context: AuthContext,
        device_state: DeviceState,
        provider: ProviderSelection,
        created_at_ms: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            trigger,
            user_intent,
            auth_context,
            context_pack: ContextPack {
                persona_snapshot: None,
                current_device_state: device_state,
                recent_turns: Vec::new(),
                retrieved_memories: Vec::new(),
            },
            allowed_tools: Vec::new(),
            provider,
            status: RunStatus::Created,
            created_at_ms,
            audit: AgentRunAudit::default(),
        }
    }

    pub fn record_retrieved_memory(&mut self, record_id: impl Into<String>) {
        self.audit.retrieved_memory_ids.push(record_id.into());
    }

    pub fn record_written_memory(&mut self, record_id: impl Into<String>) {
        self.audit.written_memory_ids.push(record_id.into());
    }

    pub fn record_tool_call(
        &mut self,
        tool: impl Into<String>,
        outcome: ToolAuditOutcome,
        summary: impl Into<String>,
    ) {
        self.audit.tool_calls.push(ToolAuditRecord {
            tool: tool.into(),
            outcome,
            summary: summary.into(),
        });
    }

    pub fn record_policy_block(&mut self, reason: impl Into<String>) {
        self.audit.policy_blocks.push(reason.into());
        self.status = RunStatus::BlockedByPolicy;
    }

    pub fn finish_with_summary(&mut self, summary: impl Into<String>, completed_at_ms: u64) {
        self.audit.provider_output_summary = Some(summary.into());
        self.audit.completed_at_ms = Some(completed_at_ms);
        self.status = RunStatus::Completed;
    }

    pub fn mark_failed(&mut self, reason: impl Into<String>, completed_at_ms: u64) {
        self.audit.failure_reason = Some(reason.into());
        self.audit.completed_at_ms = Some(completed_at_ms);
        self.status = RunStatus::Failed;
    }
}

pub fn default_tools() -> Vec<ToolDescriptor> {
    vec![
        ToolDescriptor::new("device.led.set", "Set device status LED.", RiskLevel::Safe),
        ToolDescriptor::new(
            "device.speaker.speak",
            "Play TTS or a preset sound.",
            RiskLevel::Low,
        ),
        ToolDescriptor::new(
            "device.camera.capture",
            "Capture one still image.",
            RiskLevel::Medium,
        ),
        ToolDescriptor::new(
            "device.sensor.read",
            "Read a simulated sensor value.",
            RiskLevel::Safe,
        ),
        ToolDescriptor::new("memory.search", "Search local memories.", RiskLevel::Safe),
        ToolDescriptor::new(
            "memory.write_candidate",
            "Write a candidate memory.",
            RiskLevel::Low,
        ),
        ToolDescriptor::new(
            "memory.delete",
            "Delete or archive a memory.",
            RiskLevel::Medium,
        ),
        ToolDescriptor::new(
            "identity.whoami",
            "Return current identity context.",
            RiskLevel::Safe,
        ),
        ToolDescriptor::new(
            "auth.request_confirmation",
            "Request human confirmation.",
            RiskLevel::Safe,
        ),
        ToolDescriptor::new("system.status", "Return device status.", RiskLevel::Safe),
        ToolDescriptor::new(
            "system.update.check",
            "Check for an update manifest.",
            RiskLevel::Low,
        ),
        ToolDescriptor::new(
            "system.update.apply",
            "Apply a verified OTA update.",
            RiskLevel::High,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        AgentRun, AuthContext, DeviceState, Event, ProviderSelection, RunStatus, ToolAuditOutcome,
    };

    fn provider() -> ProviderSelection {
        ProviderSelection {
            llm: "mock:phase0".to_string(),
            vision: None,
            asr: None,
            tts: None,
            embedding: None,
        }
    }

    #[test]
    fn agent_run_records_lightweight_audit_trail() {
        let mut run = AgentRun::new(
            Event::BootCompleted,
            Some("remember my preference".to_string()),
            AuthContext::anonymous(),
            DeviceState::Thinking,
            provider(),
            10,
        );

        run.audit.input_summary = Some("user asked to remember a preference".to_string());
        run.record_retrieved_memory("mem_previous");
        run.record_tool_call(
            "memory.write_candidate",
            ToolAuditOutcome::Completed,
            "candidate accepted",
        );
        run.record_written_memory("mem_new");
        run.finish_with_summary("preference stored", 20);

        assert_eq!(run.status, RunStatus::Completed);
        assert_eq!(run.audit.retrieved_memory_ids, ["mem_previous"]);
        assert_eq!(run.audit.written_memory_ids, ["mem_new"]);
        assert_eq!(run.audit.tool_calls[0].tool, "memory.write_candidate");
        assert_eq!(run.audit.completed_at_ms, Some(20));
    }

    #[test]
    fn policy_block_updates_status_and_audit() {
        let mut run = AgentRun::new(
            Event::BootCompleted,
            None,
            AuthContext::anonymous(),
            DeviceState::Thinking,
            provider(),
            10,
        );

        run.record_policy_block("owner confirmation required");

        assert_eq!(run.status, RunStatus::BlockedByPolicy);
        assert_eq!(run.audit.policy_blocks, ["owner confirmation required"]);
    }

    #[test]
    fn failed_run_records_reason_and_completion_time() {
        let mut run = AgentRun::new(
            Event::BootCompleted,
            None,
            AuthContext::anonymous(),
            DeviceState::Thinking,
            provider(),
            10,
        );

        run.mark_failed("provider timeout", 20);

        assert_eq!(run.status, RunStatus::Failed);
        assert_eq!(
            run.audit.failure_reason.as_deref(),
            Some("provider timeout")
        );
        assert_eq!(run.audit.completed_at_ms, Some(20));
    }

    #[test]
    fn device_state_transitions_allow_proto_v1_flow() {
        assert!(DeviceState::Booting
            .transition_to(DeviceState::Provisioning)
            .is_ok());
        assert!(DeviceState::Provisioning
            .transition_to(DeviceState::Idle)
            .is_ok());
        assert!(DeviceState::Idle
            .transition_to(DeviceState::Listening)
            .is_ok());
        assert!(DeviceState::Listening
            .transition_to(DeviceState::Thinking)
            .is_ok());
        assert!(DeviceState::Thinking
            .transition_to(DeviceState::Speaking)
            .is_ok());
        assert!(DeviceState::Speaking
            .transition_to(DeviceState::Idle)
            .is_ok());
    }

    #[test]
    fn device_state_rejects_invalid_jump() {
        assert!(DeviceState::Speaking
            .transition_to(DeviceState::Updating)
            .is_err());
    }

    #[test]
    fn event_reducer_maps_wake_and_provider_events() {
        assert_eq!(
            DeviceState::Idle.next_for_event(&Event::WakeWordDetected { score: 0.91 }),
            Some(DeviceState::Listening)
        );
        assert_eq!(
            DeviceState::Thinking.next_for_event(&Event::ProviderResponse {
                run_id: "run".to_string()
            }),
            Some(DeviceState::Speaking)
        );
    }

    #[test]
    fn event_reducer_respects_transition_rules() {
        assert_eq!(
            DeviceState::Speaking.next_for_event(&Event::OtaUpdateAvailable {
                version: "0.1.1".to_string()
            }),
            None
        );
    }
}
