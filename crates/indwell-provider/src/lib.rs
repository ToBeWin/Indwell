use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};
use reqwest::{multipart, Client, Url};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiCompatibleConfig {
    pub base_url: String,
    #[serde(default, skip_serializing)]
    pub api_key: Option<String>,
    pub api_key_ref: String,
    pub model: String,
    pub max_input_tokens: Option<u32>,
    pub max_output_tokens: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleProvider {
    config: OpenAiCompatibleConfig,
    client: Client,
}

impl OpenAiCompatibleProvider {
    pub fn new(config: OpenAiCompatibleConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    pub fn with_client(config: OpenAiCompatibleConfig, client: Client) -> Self {
        Self { config, client }
    }

    pub fn config(&self) -> &OpenAiCompatibleConfig {
        &self.config
    }

    pub fn chat_completions_url(&self) -> String {
        self.endpoint_url_for_path("chat/completions")
    }

    pub fn audio_transcriptions_url(&self) -> String {
        self.endpoint_url_for_path("audio/transcriptions")
    }

    pub fn audio_speech_url(&self) -> String {
        self.endpoint_url_for_path("audio/speech")
    }

    pub fn embeddings_url(&self) -> String {
        self.endpoint_url_for_path("embeddings")
    }

    pub fn build_chat_request(&self, req: ChatRequest) -> OpenAiCompatibleChatRequest {
        OpenAiCompatibleChatRequest {
            model: self.config.model.clone(),
            messages: req
                .messages
                .into_iter()
                .map(OpenAiCompatibleChatMessage::from)
                .collect(),
            max_tokens: self.config.max_output_tokens,
        }
    }

    pub fn build_vision_chat_request(
        &self,
        req: VisionRequest,
    ) -> OpenAiCompatibleVisionChatRequest {
        OpenAiCompatibleVisionChatRequest {
            model: self.config.model.clone(),
            messages: vec![OpenAiCompatibleVisionMessage {
                role: "user".to_string(),
                content: vec![
                    OpenAiCompatibleVisionContent::Text {
                        text: req
                            .prompt
                            .unwrap_or_else(|| "Describe the image for Indwell OS.".to_string()),
                    },
                    OpenAiCompatibleVisionContent::ImageUrl {
                        image_url: OpenAiCompatibleImageUrl {
                            url: data_url(&req.mime_type, &req.image_bytes),
                        },
                    },
                ],
            }],
            max_tokens: self.config.max_output_tokens,
        }
    }

    pub fn build_transcription_request_metadata(
        &self,
        audio: &AudioBlob,
    ) -> OpenAiCompatibleTranscriptionRequest {
        OpenAiCompatibleTranscriptionRequest {
            model: self.config.model.clone(),
            mime_type: audio.mime_type.clone(),
            duration_ms: audio.duration_ms,
            byte_len: audio.bytes.len(),
        }
    }

    pub fn build_speech_request(
        &self,
        text: impl Into<String>,
        voice: VoiceProfile,
    ) -> OpenAiCompatibleSpeechRequest {
        OpenAiCompatibleSpeechRequest {
            model: self.config.model.clone(),
            input: text.into(),
            voice: voice.voice,
            response_format: "wav".to_string(),
        }
    }

    pub fn build_embedding_request(
        &self,
        input: impl Into<String>,
    ) -> OpenAiCompatibleEmbeddingRequest {
        OpenAiCompatibleEmbeddingRequest {
            model: self.config.model.clone(),
            input: input.into(),
        }
    }

    fn endpoint_url_for_path(&self, path: &str) -> String {
        format!("{}/{}", self.config.base_url.trim_end_matches('/'), path)
    }

    fn endpoint_url(&self, path: &str) -> Result<Url, ProviderError> {
        if self.config.base_url.trim().is_empty() {
            return Err(ProviderError::MissingBaseUrl);
        }

        Url::parse(&self.endpoint_url_for_path(path))
            .map_err(|err| ProviderError::InvalidBaseUrl(err.to_string()))
    }

    fn api_key(&self) -> Result<&str, ProviderError> {
        let key = self.config.api_key.as_deref().unwrap_or_default().trim();
        if key.is_empty() {
            return Err(ProviderError::MissingApiKey {
                api_key_ref: self.config.api_key_ref.clone(),
            });
        }
        Ok(key)
    }
}

#[async_trait]
impl LlmProvider for OpenAiCompatibleProvider {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let endpoint = self.endpoint_url("chat/completions")?;
        let api_key = self.api_key()?;
        let body = self.build_chat_request(req);

        let response = self
            .client
            .post(endpoint)
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .map_err(ProviderError::Http)?;

        let status = response.status();
        let response_text = response.text().await.map_err(ProviderError::Http)?;
        if !status.is_success() {
            return Err(ProviderError::Api {
                status: status.as_u16(),
                body: response_text,
            });
        }

        parse_openai_compatible_chat_response(&response_text)
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            text: true,
            vision: true,
            audio_in: true,
            audio_out: true,
            embeddings: true,
        }
    }
}

#[async_trait]
impl VisionProvider for OpenAiCompatibleProvider {
    async fn analyze_image(&self, req: VisionRequest) -> Result<VisionResponse, ProviderError> {
        let endpoint = self.endpoint_url("chat/completions")?;
        let api_key = self.api_key()?;
        let body = self.build_vision_chat_request(req);
        let response = self
            .client
            .post(endpoint)
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .map_err(ProviderError::Http)?;
        let status = response.status();
        let response_text = response.text().await.map_err(ProviderError::Http)?;
        if !status.is_success() {
            return Err(ProviderError::Api {
                status: status.as_u16(),
                body: response_text,
            });
        }
        Ok(VisionResponse {
            description: parse_openai_compatible_chat_response(&response_text)?.text,
        })
    }
}

#[async_trait]
impl AsrProvider for OpenAiCompatibleProvider {
    async fn transcribe(&self, audio: AudioBlob) -> Result<Transcript, ProviderError> {
        let endpoint = self.endpoint_url("audio/transcriptions")?;
        let api_key = self.api_key()?;
        let part = multipart::Part::bytes(audio.bytes)
            .file_name("indwell-audio")
            .mime_str(&audio.mime_type)
            .map_err(|err| ProviderError::Unavailable(err.to_string()))?;
        let form = multipart::Form::new()
            .text("model", self.config.model.clone())
            .part("file", part);
        let response = self
            .client
            .post(endpoint)
            .bearer_auth(api_key)
            .multipart(form)
            .send()
            .await
            .map_err(ProviderError::Http)?;
        let status = response.status();
        let response_text = response.text().await.map_err(ProviderError::Http)?;
        if !status.is_success() {
            return Err(ProviderError::Api {
                status: status.as_u16(),
                body: response_text,
            });
        }
        parse_openai_compatible_transcript_response(&response_text)
    }
}

#[async_trait]
impl TtsProvider for OpenAiCompatibleProvider {
    async fn synthesize(
        &self,
        text: &str,
        voice: VoiceProfile,
    ) -> Result<AudioBlob, ProviderError> {
        let endpoint = self.endpoint_url("audio/speech")?;
        let api_key = self.api_key()?;
        let body = self.build_speech_request(text, voice);
        let response = self
            .client
            .post(endpoint)
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .map_err(ProviderError::Http)?;
        let status = response.status();
        let mime_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("audio/wav")
            .to_string();
        let bytes = response
            .bytes()
            .await
            .map_err(ProviderError::Http)?
            .to_vec();
        if !status.is_success() {
            return Err(ProviderError::Api {
                status: status.as_u16(),
                body: String::from_utf8_lossy(&bytes).to_string(),
            });
        }
        Ok(AudioBlob {
            bytes,
            mime_type,
            duration_ms: None,
        })
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAiCompatibleProvider {
    async fn embed(&self, input: &str) -> Result<Vec<f32>, ProviderError> {
        let endpoint = self.endpoint_url("embeddings")?;
        let api_key = self.api_key()?;
        let body = self.build_embedding_request(input);
        let response = self
            .client
            .post(endpoint)
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .map_err(ProviderError::Http)?;
        let status = response.status();
        let response_text = response.text().await.map_err(ProviderError::Http)?;
        if !status.is_success() {
            return Err(ProviderError::Api {
                status: status.as_u16(),
                body: response_text,
            });
        }
        parse_openai_compatible_embedding_response(&response_text)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenAiCompatibleChatRequest {
    pub model: String,
    pub messages: Vec<OpenAiCompatibleChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenAiCompatibleChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenAiCompatibleVisionChatRequest {
    pub model: String,
    pub messages: Vec<OpenAiCompatibleVisionMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenAiCompatibleVisionMessage {
    pub role: String,
    pub content: Vec<OpenAiCompatibleVisionContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OpenAiCompatibleVisionContent {
    Text { text: String },
    ImageUrl { image_url: OpenAiCompatibleImageUrl },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenAiCompatibleImageUrl {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenAiCompatibleTranscriptionRequest {
    pub model: String,
    pub mime_type: String,
    pub duration_ms: Option<u32>,
    pub byte_len: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenAiCompatibleSpeechRequest {
    pub model: String,
    pub input: String,
    pub voice: String,
    pub response_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenAiCompatibleEmbeddingRequest {
    pub model: String,
    pub input: String,
}

impl From<ChatMessage> for OpenAiCompatibleChatMessage {
    fn from(message: ChatMessage) -> Self {
        Self {
            role: message.role,
            content: message.content,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioBlob {
    pub bytes: Vec<u8>,
    pub mime_type: String,
    pub duration_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transcript {
    pub text: String,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceProfile {
    pub voice: String,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VisionRequest {
    pub image_bytes: Vec<u8>,
    pub mime_type: String,
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VisionResponse {
    pub description: String,
}

fn data_url(mime_type: &str, bytes: &[u8]) -> String {
    format!(
        "data:{};base64,{}",
        mime_type,
        BASE64_STANDARD.encode(bytes)
    )
}

#[derive(Debug, Deserialize)]
struct OpenAiCompatibleChatResponse {
    choices: Vec<OpenAiCompatibleChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiCompatibleChoice {
    message: OpenAiCompatibleAssistantMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiCompatibleAssistantMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiCompatibleTranscriptResponse {
    text: String,
    language: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiCompatibleEmbeddingResponse {
    data: Vec<OpenAiCompatibleEmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct OpenAiCompatibleEmbeddingData {
    embedding: Vec<f32>,
}

pub fn parse_openai_compatible_chat_response(body: &str) -> Result<ChatResponse, ProviderError> {
    let parsed: OpenAiCompatibleChatResponse =
        serde_json::from_str(body).map_err(ProviderError::ResponseParse)?;
    let text = parsed
        .choices
        .into_iter()
        .find_map(|choice| choice.message.content)
        .filter(|content| !content.is_empty())
        .ok_or(ProviderError::MissingAssistantMessage)?;

    Ok(ChatResponse { text })
}

pub fn parse_openai_compatible_transcript_response(
    body: &str,
) -> Result<Transcript, ProviderError> {
    let parsed: OpenAiCompatibleTranscriptResponse =
        serde_json::from_str(body).map_err(ProviderError::ResponseParse)?;
    Ok(Transcript {
        text: parsed.text,
        language: parsed.language,
    })
}

pub fn parse_openai_compatible_embedding_response(body: &str) -> Result<Vec<f32>, ProviderError> {
    let parsed: OpenAiCompatibleEmbeddingResponse =
        serde_json::from_str(body).map_err(ProviderError::ResponseParse)?;
    parsed
        .data
        .into_iter()
        .next()
        .map(|item| item.embedding)
        .filter(|embedding| !embedding.is_empty())
        .ok_or(ProviderError::MissingEmbedding)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub text: bool,
    pub vision: bool,
    pub audio_in: bool,
    pub audio_out: bool,
    pub embeddings: bool,
}

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("provider unavailable: {0}")]
    Unavailable(String),
    #[error("openai-compatible provider base_url is missing")]
    MissingBaseUrl,
    #[error("openai-compatible provider api key is missing for ref {api_key_ref}")]
    MissingApiKey { api_key_ref: String },
    #[error("openai-compatible provider base_url is invalid: {0}")]
    InvalidBaseUrl(String),
    #[error("provider http error: {0}")]
    Http(#[source] reqwest::Error),
    #[error("provider api error: status {status}, body: {body}")]
    Api { status: u16, body: String },
    #[error("provider response parse error: {0}")]
    ResponseParse(#[source] serde_json::Error),
    #[error("provider response did not include an assistant message")]
    MissingAssistantMessage,
    #[error("provider response did not include an embedding")]
    MissingEmbedding,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, ProviderError>;
    fn capabilities(&self) -> ProviderCapabilities;
}

#[async_trait]
pub trait VisionProvider: Send + Sync {
    async fn analyze_image(&self, req: VisionRequest) -> Result<VisionResponse, ProviderError>;
}

#[async_trait]
pub trait AsrProvider: Send + Sync {
    async fn transcribe(&self, audio: AudioBlob) -> Result<Transcript, ProviderError>;
}

#[async_trait]
pub trait TtsProvider: Send + Sync {
    async fn synthesize(&self, text: &str, voice: VoiceProfile)
        -> Result<AudioBlob, ProviderError>;
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, input: &str) -> Result<Vec<f32>, ProviderError>;
}

#[derive(Debug, Clone)]
pub struct MockLlmProvider;

#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let last = req
            .messages
            .last()
            .map(|message| message.content.as_str())
            .unwrap_or("");
        Ok(ChatResponse {
            text: format!("Indwell mock response: {}", last),
        })
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            text: true,
            vision: false,
            audio_in: false,
            audio_out: false,
            embeddings: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MockVisionProvider;

#[async_trait]
impl VisionProvider for MockVisionProvider {
    async fn analyze_image(&self, req: VisionRequest) -> Result<VisionResponse, ProviderError> {
        Ok(VisionResponse {
            description: format!(
                "Mock vision saw {} bytes of {} image data.",
                req.image_bytes.len(),
                req.mime_type
            ),
        })
    }
}

#[derive(Debug, Clone)]
pub struct MockAsrProvider;

#[async_trait]
impl AsrProvider for MockAsrProvider {
    async fn transcribe(&self, audio: AudioBlob) -> Result<Transcript, ProviderError> {
        Ok(Transcript {
            text: format!("mock transcript for {} bytes", audio.bytes.len()),
            language: Some("en".to_string()),
        })
    }
}

#[derive(Debug, Clone)]
pub struct MockTtsProvider;

#[async_trait]
impl TtsProvider for MockTtsProvider {
    async fn synthesize(
        &self,
        text: &str,
        voice: VoiceProfile,
    ) -> Result<AudioBlob, ProviderError> {
        Ok(AudioBlob {
            bytes: format!("mock audio:{}:{text}", voice.voice).into_bytes(),
            mime_type: "audio/wav".to_string(),
            duration_ms: Some((text.len() as u32).saturating_mul(40)),
        })
    }
}

#[derive(Debug, Clone)]
pub struct MockEmbeddingProvider;

#[async_trait]
impl EmbeddingProvider for MockEmbeddingProvider {
    async fn embed(&self, input: &str) -> Result<Vec<f32>, ProviderError> {
        let mut vector = vec![0.0_f32; 8];
        for (index, byte) in input.bytes().enumerate() {
            vector[index % 8] += f32::from(byte) / 255.0;
        }
        Ok(vector)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn provider() -> OpenAiCompatibleProvider {
        OpenAiCompatibleProvider::new(OpenAiCompatibleConfig {
            base_url: "https://api.example.com/v1/".to_string(),
            api_key: Some("test-key".to_string()),
            api_key_ref: "key_llm_main".to_string(),
            model: "gpt-compatible-mini".to_string(),
            max_input_tokens: Some(4000),
            max_output_tokens: Some(600),
        })
    }

    #[test]
    fn builds_openai_compatible_chat_completions_url() {
        assert_eq!(
            provider().chat_completions_url(),
            "https://api.example.com/v1/chat/completions"
        );
        assert_eq!(
            provider().audio_transcriptions_url(),
            "https://api.example.com/v1/audio/transcriptions"
        );
        assert_eq!(
            provider().audio_speech_url(),
            "https://api.example.com/v1/audio/speech"
        );
        assert_eq!(
            provider().embeddings_url(),
            "https://api.example.com/v1/embeddings"
        );
    }

    #[test]
    fn serializes_messages_in_openai_compatible_shape() {
        let body = provider().build_chat_request(ChatRequest {
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: "You are Indwell OS.".to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: "Hello from the device.".to_string(),
                },
            ],
        });

        let serialized = serde_json::to_value(body).expect("request should serialize");

        assert_eq!(
            serialized,
            json!({
                "model": "gpt-compatible-mini",
                "messages": [
                    {
                        "role": "system",
                        "content": "You are Indwell OS."
                    },
                    {
                        "role": "user",
                        "content": "Hello from the device."
                    }
                ],
                "max_tokens": 600
            })
        );
    }

    #[test]
    fn builds_openai_compatible_multimodal_request_shapes() {
        let provider = provider();

        let vision = serde_json::to_value(provider.build_vision_chat_request(VisionRequest {
            image_bytes: vec![1, 2, 3],
            mime_type: "image/jpeg".to_string(),
            prompt: Some("What changed on the desk?".to_string()),
        }))
        .expect("vision request should serialize");
        assert_eq!(vision["messages"][0]["content"][0]["type"], "text");
        assert_eq!(vision["messages"][0]["content"][1]["type"], "image_url");
        assert_eq!(
            vision["messages"][0]["content"][1]["image_url"]["url"],
            "data:image/jpeg;base64,AQID"
        );

        let asr = provider.build_transcription_request_metadata(&AudioBlob {
            bytes: vec![0; 16],
            mime_type: "audio/wav".to_string(),
            duration_ms: Some(500),
        });
        assert_eq!(asr.byte_len, 16);
        assert_eq!(asr.duration_ms, Some(500));

        let tts = provider.build_speech_request(
            "hello",
            VoiceProfile {
                voice: "warm_indwell".to_string(),
                language: None,
            },
        );
        assert_eq!(tts.input, "hello");
        assert_eq!(tts.voice, "warm_indwell");

        let embedding = provider.build_embedding_request("memory query");
        assert_eq!(embedding.input, "memory query");
    }

    #[test]
    fn omits_max_tokens_when_not_configured() {
        let provider = OpenAiCompatibleProvider::new(OpenAiCompatibleConfig {
            base_url: "https://api.example.com/v1".to_string(),
            api_key: Some("test-key".to_string()),
            api_key_ref: "key_llm_main".to_string(),
            model: "gpt-compatible-mini".to_string(),
            max_input_tokens: None,
            max_output_tokens: None,
        });

        let serialized = serde_json::to_value(provider.build_chat_request(ChatRequest {
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Ping".to_string(),
            }],
        }))
        .expect("request should serialize");

        assert_eq!(
            serialized,
            json!({
                "model": "gpt-compatible-mini",
                "messages": [
                    {
                        "role": "user",
                        "content": "Ping"
                    }
                ]
            })
        );
    }

    #[test]
    fn parses_openai_compatible_chat_response() {
        let response = parse_openai_compatible_chat_response(
            r#"{
                "id": "chatcmpl-test",
                "object": "chat.completion",
                "choices": [
                    {
                        "index": 0,
                        "message": {
                            "role": "assistant",
                            "content": "Hello from the real provider path."
                        },
                        "finish_reason": "stop"
                    }
                ]
            }"#,
        )
        .expect("response should parse");

        assert_eq!(response.text, "Hello from the real provider path.");
    }

    #[test]
    fn parses_openai_compatible_speech_and_embedding_responses() {
        let transcript = parse_openai_compatible_transcript_response(
            r#"{ "text": "hello from audio", "language": "en" }"#,
        )
        .expect("transcript should parse");
        assert_eq!(transcript.text, "hello from audio");
        assert_eq!(transcript.language.as_deref(), Some("en"));

        let embedding = parse_openai_compatible_embedding_response(
            r#"{ "data": [ { "embedding": [0.1, 0.2, 0.3] } ] }"#,
        )
        .expect("embedding should parse");
        assert_eq!(embedding, vec![0.1, 0.2, 0.3]);
    }

    #[tokio::test]
    async fn mock_speech_vision_and_embedding_providers_work() {
        let asr = MockAsrProvider;
        let transcript = asr
            .transcribe(AudioBlob {
                bytes: vec![1, 2, 3],
                mime_type: "audio/wav".to_string(),
                duration_ms: Some(120),
            })
            .await
            .unwrap();
        assert!(transcript.text.contains("3 bytes"));

        let tts = MockTtsProvider;
        let audio = tts
            .synthesize(
                "hello",
                VoiceProfile {
                    voice: "warm_indwell".to_string(),
                    language: Some("en".to_string()),
                },
            )
            .await
            .unwrap();
        assert_eq!(audio.mime_type, "audio/wav");

        let vision = MockVisionProvider;
        let seen = vision
            .analyze_image(VisionRequest {
                image_bytes: vec![0; 4],
                mime_type: "image/jpeg".to_string(),
                prompt: None,
            })
            .await
            .unwrap();
        assert!(seen.description.contains("4 bytes"));

        let embedding = MockEmbeddingProvider;
        assert_eq!(embedding.embed("memory").await.unwrap().len(), 8);
    }

    #[tokio::test]
    async fn returns_missing_base_url_before_sending_request() {
        let provider = OpenAiCompatibleProvider::new(OpenAiCompatibleConfig {
            base_url: "  ".to_string(),
            api_key: Some("test-key".to_string()),
            api_key_ref: "key_llm_main".to_string(),
            model: "gpt-compatible-mini".to_string(),
            max_input_tokens: None,
            max_output_tokens: None,
        });

        let error = provider
            .chat(ChatRequest {
                messages: vec![ChatMessage {
                    role: "user".to_string(),
                    content: "Ping".to_string(),
                }],
            })
            .await
            .expect_err("missing base url should fail");

        assert!(matches!(error, ProviderError::MissingBaseUrl));
    }

    #[tokio::test]
    async fn returns_missing_api_key_before_sending_request() {
        let provider = OpenAiCompatibleProvider::new(OpenAiCompatibleConfig {
            base_url: "https://api.example.com/v1".to_string(),
            api_key: None,
            api_key_ref: "key_llm_main".to_string(),
            model: "gpt-compatible-mini".to_string(),
            max_input_tokens: None,
            max_output_tokens: None,
        });

        let error = provider
            .chat(ChatRequest {
                messages: vec![ChatMessage {
                    role: "user".to_string(),
                    content: "Ping".to_string(),
                }],
            })
            .await
            .expect_err("missing api key should fail");

        assert!(matches!(
            error,
            ProviderError::MissingApiKey { api_key_ref } if api_key_ref == "key_llm_main"
        ));
    }

    #[tokio::test]
    async fn sends_openai_compatible_request_and_parses_response() {
        let server = TestServer::start(
            r#"{"choices":[{"message":{"role":"assistant","content":"Pong from provider."}}]}"#,
        );
        let provider = OpenAiCompatibleProvider::new(OpenAiCompatibleConfig {
            base_url: server.base_url(),
            api_key: Some("test-key".to_string()),
            api_key_ref: "key_llm_main".to_string(),
            model: "gpt-compatible-mini".to_string(),
            max_input_tokens: None,
            max_output_tokens: Some(120),
        });

        let response = provider
            .chat(ChatRequest {
                messages: vec![ChatMessage {
                    role: "user".to_string(),
                    content: "Ping".to_string(),
                }],
            })
            .await
            .expect("chat request should succeed");

        assert_eq!(response.text, "Pong from provider.");

        let captured = server.captured_request();
        assert!(captured.starts_with("POST /v1/chat/completions HTTP/1.1"));
        assert!(captured.contains("authorization: Bearer test-key"));
        let request_body = captured
            .split("\r\n\r\n")
            .nth(1)
            .expect("request should include a body");
        let body: serde_json::Value =
            serde_json::from_str(request_body).expect("request body should be json");
        assert_eq!(
            body,
            json!({
                "model": "gpt-compatible-mini",
                "messages": [
                    {
                        "role": "user",
                        "content": "Ping"
                    }
                ],
                "max_tokens": 120
            })
        );
    }

    #[tokio::test]
    async fn sends_openai_compatible_vision_request() {
        let server = TestServer::start(
            r#"{"choices":[{"message":{"role":"assistant","content":"A tidy desk."}}]}"#,
        );
        let provider = OpenAiCompatibleProvider::new(OpenAiCompatibleConfig {
            base_url: server.base_url(),
            api_key: Some("test-key".to_string()),
            api_key_ref: "key_vision_main".to_string(),
            model: "vision-model".to_string(),
            max_input_tokens: None,
            max_output_tokens: Some(80),
        });

        let response = provider
            .analyze_image(VisionRequest {
                image_bytes: vec![1, 2, 3],
                mime_type: "image/jpeg".to_string(),
                prompt: Some("what is here?".to_string()),
            })
            .await
            .expect("vision request should succeed");

        assert_eq!(response.description, "A tidy desk.");
        let captured = server.captured_request();
        assert!(captured.starts_with("POST /v1/chat/completions HTTP/1.1"));
        assert!(captured.contains("data:image/jpeg;base64,AQID"));
    }

    #[tokio::test]
    async fn sends_openai_compatible_tts_request() {
        let server = TestServer::start_with_content_type("WAVDATA", "audio/wav");
        let provider = OpenAiCompatibleProvider::new(OpenAiCompatibleConfig {
            base_url: server.base_url(),
            api_key: Some("test-key".to_string()),
            api_key_ref: "key_tts_main".to_string(),
            model: "tts-model".to_string(),
            max_input_tokens: None,
            max_output_tokens: None,
        });

        let audio = provider
            .synthesize(
                "hello",
                VoiceProfile {
                    voice: "warm_indwell".to_string(),
                    language: None,
                },
            )
            .await
            .expect("tts should return audio");

        assert_eq!(audio.bytes, b"WAVDATA");
        assert_eq!(audio.mime_type, "audio/wav");
        let captured = server.captured_request();
        assert!(captured.starts_with("POST /v1/audio/speech HTTP/1.1"));
        assert!(captured.contains("\"voice\":\"warm_indwell\""));
    }

    #[tokio::test]
    async fn sends_openai_compatible_embedding_request() {
        let server = TestServer::start(r#"{ "data": [ { "embedding": [1.0, 2.0] } ] }"#);
        let provider = OpenAiCompatibleProvider::new(OpenAiCompatibleConfig {
            base_url: server.base_url(),
            api_key: Some("test-key".to_string()),
            api_key_ref: "key_embedding_main".to_string(),
            model: "embedding-model".to_string(),
            max_input_tokens: None,
            max_output_tokens: None,
        });

        let embedding = provider.embed("memory").await.expect("embedding");

        assert_eq!(embedding, vec![1.0, 2.0]);
        let captured = server.captured_request();
        assert!(captured.starts_with("POST /v1/embeddings HTTP/1.1"));
        assert!(captured.contains("\"input\":\"memory\""));
    }

    struct TestServer {
        address: std::net::SocketAddr,
        captured: std::sync::Arc<std::sync::Mutex<Option<String>>>,
        handle: std::thread::JoinHandle<()>,
    }

    impl TestServer {
        fn start(response_body: &'static str) -> Self {
            Self::start_with_content_type(response_body, "application/json")
        }

        fn start_with_content_type(
            response_body: &'static str,
            content_type: &'static str,
        ) -> Self {
            let listener =
                std::net::TcpListener::bind("127.0.0.1:0").expect("test server should bind");
            let address = listener.local_addr().expect("test server should have addr");
            let captured = std::sync::Arc::new(std::sync::Mutex::new(None));
            let captured_for_thread = std::sync::Arc::clone(&captured);
            let handle = std::thread::spawn(move || {
                use std::io::{Read, Write};
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: {}\r\ncontent-length: {}\r\n\r\n{}",
                    content_type,
                    response_body.len(),
                    response_body
                );

                let (mut stream, _) = listener.accept().expect("test server should accept");
                let mut bytes = Vec::new();
                let mut buffer = [0; 1024];
                let header_end;
                loop {
                    let read = stream.read(&mut buffer).expect("request should read");
                    assert!(read > 0, "client closed before sending headers");
                    bytes.extend_from_slice(&buffer[..read]);
                    if let Some(position) =
                        bytes.windows(4).position(|window| window == b"\r\n\r\n")
                    {
                        header_end = position + 4;
                        break;
                    }
                }

                let headers = String::from_utf8_lossy(&bytes[..header_end]).to_lowercase();
                let content_length = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length: "))
                    .and_then(|value| value.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                while bytes.len() < header_end + content_length {
                    let read = stream.read(&mut buffer).expect("request body should read");
                    assert!(read > 0, "client closed before sending body");
                    bytes.extend_from_slice(&buffer[..read]);
                }

                let request = String::from_utf8(bytes).expect("request should be utf8");
                *captured_for_thread.lock().expect("captured should lock") = Some(request);
                stream
                    .write_all(response.as_bytes())
                    .expect("response should write");
            });

            Self {
                address,
                captured,
                handle,
            }
        }

        fn base_url(&self) -> String {
            format!("http://{}/v1", self.address)
        }

        fn captured_request(self) -> String {
            self.handle
                .join()
                .expect("test server thread should finish");
            self.captured
                .lock()
                .expect("captured should lock")
                .clone()
                .expect("request should be captured")
        }
    }
}
