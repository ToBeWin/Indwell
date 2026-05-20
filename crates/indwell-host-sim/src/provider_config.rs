use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use indwell_protocol::{ProviderConfig, ProviderConfigSet};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderConfigStoreError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("api_key_ref must be a local key reference, not a raw API key")]
    RawApiKeyRef,
}

#[derive(Debug, Clone)]
pub struct JsonProviderConfigStore {
    path: PathBuf,
}

impl JsonProviderConfigStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, ProviderConfigStoreError> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        let store = Self {
            path: root.join("providers.json"),
        };

        if !store.path.exists() {
            store.save(&ProviderConfigSet::default_host_sim())?;
        }

        Ok(store)
    }

    pub fn load(&self) -> Result<ProviderConfigSet, ProviderConfigStoreError> {
        let file = File::open(&self.path)?;
        Ok(serde_json::from_reader(file)?)
    }

    pub fn save(&self, config: &ProviderConfigSet) -> Result<(), ProviderConfigStoreError> {
        validate_provider_refs(config)?;
        let tmp_path = self.path.with_extension("json.tmp");
        let mut file = File::create(&tmp_path)?;
        serde_json::to_writer_pretty(&mut file, config)?;
        writeln!(file)?;
        file.flush()?;
        fs::rename(tmp_path, &self.path)?;
        Ok(())
    }
}

trait DefaultHostSimProviderConfig {
    fn default_host_sim() -> Self;
}

impl DefaultHostSimProviderConfig for ProviderConfigSet {
    fn default_host_sim() -> Self {
        Self {
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
}

fn validate_provider_refs(config: &ProviderConfigSet) -> Result<(), ProviderConfigStoreError> {
    for provider in [
        Some(&config.llm),
        config.vision.as_ref(),
        config.asr.as_ref(),
        config.tts.as_ref(),
        config.embedding.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if provider
            .api_key_ref
            .as_ref()
            .is_some_and(|value| looks_like_raw_api_key(value))
        {
            return Err(ProviderConfigStoreError::RawApiKeyRef);
        }
    }
    Ok(())
}

fn looks_like_raw_api_key(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.starts_with("sk-") || trimmed.starts_with("sk_") || trimmed.starts_with("sk-proj-")
}

#[cfg(test)]
mod tests {
    use indwell_protocol::{ProviderConfig, ProviderConfigSet};

    use super::{JsonProviderConfigStore, ProviderConfigStoreError};

    #[test]
    fn creates_default_provider_config() {
        let root =
            std::env::temp_dir().join(format!("indwell-provider-config-{}", uuid::Uuid::new_v4()));
        let store = JsonProviderConfigStore::new(root).unwrap();
        let config = store.load().unwrap();

        assert_eq!(config.llm.kind, "mock");
        assert_eq!(config.llm.model, "mock:phase0");
    }

    #[test]
    fn rejects_raw_api_key_in_api_key_ref() {
        let root =
            std::env::temp_dir().join(format!("indwell-provider-config-{}", uuid::Uuid::new_v4()));
        let store = JsonProviderConfigStore::new(root).unwrap();
        let mut config = ProviderConfigSet {
            llm: ProviderConfig {
                kind: "openai_compatible".to_string(),
                base_url: Some("https://api.example.com/v1".to_string()),
                api_key_ref: Some("sk-proj-not-a-reference".to_string()),
                model: "model".to_string(),
                max_input_tokens: None,
                max_output_tokens: None,
            },
            vision: None,
            asr: None,
            tts: None,
            embedding: None,
        };

        let error = store.save(&config).unwrap_err();
        assert!(matches!(error, ProviderConfigStoreError::RawApiKeyRef));

        config.llm.api_key_ref = Some("key_llm_main".to_string());
        store.save(&config).unwrap();
    }
}
