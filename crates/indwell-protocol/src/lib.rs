use indwell_core::DeviceState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MobileCommand {
    SendText { text: String },
    CaptureImage,
    SearchMemory { query: String },
    DeleteMemory { id: String },
    SystemStatus,
    CheckUpdate,
    ApplyUpdate { version: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DeviceMessage {
    State { state: DeviceState },
    Text { text: String },
    ToolBlocked { tool: String, reason: String },
    MemoryWritten { id: String },
    Error { code: String, message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub kind: String,
    pub base_url: Option<String>,
    pub api_key_ref: Option<String>,
    pub model: String,
    pub max_input_tokens: Option<u32>,
    pub max_output_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderConfigSet {
    pub llm: ProviderConfig,
    pub vision: Option<ProviderConfig>,
    pub asr: Option<ProviderConfig>,
    pub tts: Option<ProviderConfig>,
    pub embedding: Option<ProviderConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WifiCredentials {
    pub ssid: String,
    pub password_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvisioningRequest {
    pub device_id: String,
    pub wifi: WifiCredentials,
    pub providers: ProviderConfigSet,
    pub owner_pairing_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvisioningResponse {
    pub accepted: bool,
    pub next_state: DeviceState,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEnvelope<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiEnvelope<T> {
    pub fn ok(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(error: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(error.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomWebhookInputRequest {
    pub session_id: String,
    pub subject_hint: Option<String>,
    pub text: Option<String>,
    pub command: Option<MobileCommand>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomWebhookInputResponse {
    pub accepted: bool,
    pub event_id: Option<String>,
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{
        CustomWebhookInputRequest, MobileCommand, ProviderConfig, ProviderConfigSet,
        ProvisioningRequest, WifiCredentials,
    };

    #[test]
    fn custom_webhook_input_request_round_trips_text_payload() {
        let req = CustomWebhookInputRequest {
            session_id: "gateway-session-1".to_string(),
            subject_hint: Some("owner:local".to_string()),
            text: Some("hello from a user gateway".to_string()),
            command: None,
            source: Some("user-home-gateway".to_string()),
        };

        let json = serde_json::to_string(&req).expect("serialize webhook request");
        let decoded: CustomWebhookInputRequest =
            serde_json::from_str(&json).expect("deserialize webhook request");

        assert_eq!(decoded, req);
    }

    #[test]
    fn custom_webhook_input_request_supports_mobile_commands() {
        let req = CustomWebhookInputRequest {
            session_id: "gateway-session-2".to_string(),
            subject_hint: None,
            text: None,
            command: Some(MobileCommand::SystemStatus),
            source: Some("automation".to_string()),
        };

        let json = serde_json::to_value(&req).expect("serialize webhook command");

        assert_eq!(json["command"]["type"], "system_status");
    }

    #[test]
    fn provisioning_request_keeps_password_as_reference() {
        let req = ProvisioningRequest {
            device_id: "indwell-proto-v1".to_string(),
            wifi: WifiCredentials {
                ssid: "home".to_string(),
                password_ref: Some("wifi_home".to_string()),
            },
            providers: ProviderConfigSet {
                llm: ProviderConfig {
                    kind: "mock".to_string(),
                    base_url: None,
                    api_key_ref: Some("key_llm_main".to_string()),
                    model: "mock:phase0".to_string(),
                    max_input_tokens: Some(4000),
                    max_output_tokens: Some(600),
                },
                vision: None,
                asr: None,
                tts: None,
                embedding: None,
            },
            owner_pairing_label: Some("Bingo phone".to_string()),
        };

        let json = serde_json::to_value(&req).expect("serialize provisioning");

        assert_eq!(json["wifi"]["password_ref"], "wifi_home");
        assert!(json["wifi"].get("password").is_none());
    }
}
