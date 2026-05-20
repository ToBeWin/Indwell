use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

use indwell_core::AgentRun;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum RunStoreError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

pub trait RunStore {
    fn append(&mut self, run: &AgentRun) -> Result<(), RunStoreError>;
    fn list(&self) -> Result<Vec<AgentRun>, RunStoreError>;
    fn get(&self, id: Uuid) -> Result<Option<AgentRun>, RunStoreError>;
}

#[derive(Debug, Clone)]
pub struct JsonlRunStore {
    path: PathBuf,
}

impl JsonlRunStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, RunStoreError> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        Ok(Self {
            path: root.join("runs.jsonl"),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl RunStore for JsonlRunStore {
    fn append(&mut self, run: &AgentRun) -> Result<(), RunStoreError> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        serde_json::to_writer(&mut file, run)?;
        writeln!(file)?;
        Ok(())
    }

    fn list(&self) -> Result<Vec<AgentRun>, RunStoreError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut runs = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            runs.push(serde_json::from_str(&line)?);
        }
        Ok(runs)
    }

    fn get(&self, id: Uuid) -> Result<Option<AgentRun>, RunStoreError> {
        Ok(self.list()?.into_iter().find(|run| run.id == id))
    }
}

#[cfg(test)]
mod tests {
    use indwell_core::{AgentRun, AuthContext, DeviceState, Event, ProviderSelection};

    use super::{JsonlRunStore, RunStore};

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
    fn appends_and_reads_agent_run_jsonl() {
        let root = std::env::temp_dir().join(format!("indwell-runs-{}", uuid::Uuid::new_v4()));
        let mut store = JsonlRunStore::new(root).unwrap();
        let mut run = AgentRun::new(
            Event::BootCompleted,
            Some("hello".to_string()),
            AuthContext::anonymous(),
            DeviceState::Thinking,
            provider(),
            1,
        );
        run.finish_with_summary("ok", 2);

        store.append(&run).unwrap();
        let fetched = store.get(run.id).unwrap().unwrap();

        assert_eq!(fetched.id, run.id);
        assert_eq!(fetched.audit.provider_output_summary.as_deref(), Some("ok"));
    }
}
