use indwell_protocol::{CustomWebhookInputRequest, MobileCommand};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelCapabilities {
    pub text: bool,
    pub image: bool,
    pub audio: bool,
    pub confirmation: bool,
    pub commands: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl ChannelPolicy {
    /// Returns conservative defaults for each channel class.
    ///
    /// Local, paired channels may manage memory/config/OTA after owner auth and
    /// confirmation checks. Remote and public gateway-style channels default to
    /// chat/status only and cannot read memory, use the camera, alter config, or
    /// apply OTA updates without an explicit policy override.
    pub fn default_for(channel: ChannelKind) -> Self {
        match channel {
            ChannelKind::LocalPwa | ChannelKind::Ble | ChannelKind::UsbSerial => Self {
                channel,
                allow_chat: true,
                allow_memory_read: true,
                allow_memory_write: true,
                allow_camera: true,
                allow_sensor_read: true,
                allow_system_status: true,
                allow_system_config: true,
                allow_ota: true,
                requires_owner_auth_for_medium: true,
                requires_confirmation_for_high: true,
            },
            ChannelKind::LanWebSocket => Self {
                channel,
                allow_chat: true,
                allow_memory_read: true,
                allow_memory_write: true,
                allow_camera: true,
                allow_sensor_read: true,
                allow_system_status: true,
                allow_system_config: false,
                allow_ota: false,
                requires_owner_auth_for_medium: true,
                requires_confirmation_for_high: true,
            },
            ChannelKind::Mqtt | ChannelKind::HomeAssistant => Self {
                channel,
                allow_chat: false,
                allow_memory_read: false,
                allow_memory_write: true,
                allow_camera: false,
                allow_sensor_read: true,
                allow_system_status: true,
                allow_system_config: false,
                allow_ota: false,
                requires_owner_auth_for_medium: true,
                requires_confirmation_for_high: true,
            },
            _ => Self {
                channel,
                allow_chat: true,
                allow_memory_read: false,
                allow_memory_write: true,
                allow_camera: false,
                allow_sensor_read: false,
                allow_system_status: true,
                allow_system_config: false,
                allow_ota: false,
                requires_owner_auth_for_medium: true,
                requires_confirmation_for_high: true,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaRef {
    pub uri: String,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelPrincipal {
    pub subject_id: String,
    pub owner_authenticated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
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

impl ChannelEvent {
    pub fn channel(&self) -> ChannelKind {
        match self {
            Self::UserText { channel, .. }
            | Self::UserImage { channel, .. }
            | Self::UserAudio { channel, .. }
            | Self::Confirmation { channel, .. }
            | Self::RemoteCommand { channel, .. } => *channel,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInbound {
    pub channel: ChannelKind,
    pub session_id: String,
    pub subject_hint: Option<String>,
    pub principal: Option<ChannelPrincipal>,
    pub text: Option<String>,
    pub command: Option<MobileCommand>,
}

impl From<CustomWebhookInputRequest> for ChannelInbound {
    fn from(req: CustomWebhookInputRequest) -> Self {
        Self {
            channel: ChannelKind::CustomWebhook,
            session_id: req.session_id,
            subject_hint: req.subject_hint.or(req.source),
            principal: None,
            text: req.text,
            command: req.command,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndwellMessage {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelOutbound {
    pub channel: ChannelKind,
    pub session_id: String,
    pub text: String,
}

#[derive(Debug, Error)]
pub enum ChannelError {
    #[error("inbound message has no supported payload")]
    EmptyInbound,
}

pub trait ChannelAdapter {
    fn channel_kind(&self) -> ChannelKind;
    fn capabilities(&self) -> ChannelCapabilities;
    fn normalize_inbound(&self, input: ChannelInbound) -> Result<ChannelEvent, ChannelError>;
    fn render_outbound(
        &self,
        session_id: String,
        msg: IndwellMessage,
    ) -> Result<ChannelOutbound, ChannelError>;
}

#[derive(Debug, Clone)]
pub struct BasicChannelAdapter {
    channel: ChannelKind,
}

impl BasicChannelAdapter {
    pub fn new(channel: ChannelKind) -> Self {
        Self { channel }
    }
}

impl ChannelAdapter for BasicChannelAdapter {
    fn channel_kind(&self) -> ChannelKind {
        self.channel
    }

    fn capabilities(&self) -> ChannelCapabilities {
        ChannelCapabilities {
            text: true,
            image: false,
            audio: false,
            confirmation: true,
            commands: true,
        }
    }

    fn normalize_inbound(&self, input: ChannelInbound) -> Result<ChannelEvent, ChannelError> {
        if let Some(command) = input.command {
            return Ok(ChannelEvent::RemoteCommand {
                channel: self.channel,
                session_id: input.session_id,
                command,
            });
        }

        if let Some(text) = input.text {
            return Ok(ChannelEvent::UserText {
                channel: self.channel,
                session_id: input.session_id,
                subject_hint: input.subject_hint,
                text,
            });
        }

        Err(ChannelError::EmptyInbound)
    }

    fn render_outbound(
        &self,
        session_id: String,
        msg: IndwellMessage,
    ) -> Result<ChannelOutbound, ChannelError> {
        Ok(ChannelOutbound {
            channel: self.channel,
            session_id,
            text: msg.text,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BasicChannelAdapter, ChannelAdapter, ChannelEvent, ChannelInbound, ChannelKind,
        ChannelPolicy,
    };
    use indwell_protocol::CustomWebhookInputRequest;

    #[test]
    fn local_pwa_and_custom_webhook_text_normalize_to_equivalent_user_text_shape() {
        let session_id = "session-123".to_string();
        let subject_hint = Some("owner".to_string());
        let text = "remember that I like quiet status lights".to_string();

        let pwa = BasicChannelAdapter::new(ChannelKind::LocalPwa);
        let pwa_event = pwa
            .normalize_inbound(ChannelInbound {
                channel: ChannelKind::LocalPwa,
                session_id: session_id.clone(),
                subject_hint: subject_hint.clone(),
                principal: None,
                text: Some(text.clone()),
                command: None,
            })
            .expect("normalize local pwa text");

        let webhook = BasicChannelAdapter::new(ChannelKind::CustomWebhook);
        let webhook_event = webhook
            .normalize_inbound(
                CustomWebhookInputRequest {
                    session_id: session_id.clone(),
                    subject_hint: subject_hint.clone(),
                    text: Some(text.clone()),
                    command: None,
                    source: Some("home-gateway".to_string()),
                }
                .into(),
            )
            .expect("normalize custom webhook text");

        assert_eq!(pwa_event.channel(), ChannelKind::LocalPwa);
        assert_eq!(webhook_event.channel(), ChannelKind::CustomWebhook);

        let pwa_shape = comparable_user_text_shape(pwa_event);
        let webhook_shape = comparable_user_text_shape(webhook_event);
        assert_eq!(pwa_shape, webhook_shape);
    }

    #[test]
    fn local_pwa_defaults_allow_owner_managed_control_surface() {
        let policy = ChannelPolicy::default_for(ChannelKind::LocalPwa);

        assert!(policy.allow_chat);
        assert!(policy.allow_memory_read);
        assert!(policy.allow_memory_write);
        assert!(policy.allow_camera);
        assert!(policy.allow_sensor_read);
        assert!(policy.allow_system_status);
        assert!(policy.allow_system_config);
        assert!(policy.allow_ota);
        assert!(policy.requires_owner_auth_for_medium);
        assert!(policy.requires_confirmation_for_high);
    }

    #[test]
    fn custom_webhook_defaults_are_gateway_safe() {
        let policy = ChannelPolicy::default_for(ChannelKind::CustomWebhook);

        assert!(policy.allow_chat);
        assert!(!policy.allow_memory_read);
        assert!(policy.allow_memory_write);
        assert!(!policy.allow_camera);
        assert!(!policy.allow_sensor_read);
        assert!(policy.allow_system_status);
        assert!(!policy.allow_system_config);
        assert!(!policy.allow_ota);
        assert!(policy.requires_owner_auth_for_medium);
        assert!(policy.requires_confirmation_for_high);
    }

    #[test]
    fn mqtt_and_home_assistant_defaults_are_event_oriented() {
        for channel in [ChannelKind::Mqtt, ChannelKind::HomeAssistant] {
            let policy = ChannelPolicy::default_for(channel);

            assert!(!policy.allow_chat);
            assert!(!policy.allow_memory_read);
            assert!(policy.allow_memory_write);
            assert!(!policy.allow_camera);
            assert!(policy.allow_sensor_read);
            assert!(policy.allow_system_status);
            assert!(!policy.allow_system_config);
            assert!(!policy.allow_ota);
        }
    }

    fn comparable_user_text_shape(event: ChannelEvent) -> (String, Option<String>, String) {
        match event {
            ChannelEvent::UserText {
                session_id,
                subject_hint,
                text,
                ..
            } => (session_id, subject_hint, text),
            other => panic!("expected user text event, got {other:?}"),
        }
    }
}
