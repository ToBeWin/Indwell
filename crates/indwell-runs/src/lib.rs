use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

use indwell_core::{AgentRun, RunStatus};
use serde::{Deserialize, Serialize};
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
    fn append_checkpoint(
        &mut self,
        run: &AgentRun,
        stage: impl Into<String>,
        recorded_at_ms: u64,
    ) -> Result<(), RunStoreError>;
    fn list(&self) -> Result<Vec<AgentRun>, RunStoreError>;
    fn get(&self, id: Uuid) -> Result<Option<AgentRun>, RunStoreError>;
    fn entries_for_run(&self, id: Uuid) -> Result<Vec<RunLedgerEntry>, RunStoreError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunLedgerEntry {
    pub entry_id: Uuid,
    pub run_id: Uuid,
    pub stage: String,
    pub status: RunStatus,
    pub recorded_at_ms: u64,
    pub run: AgentRun,
}

#[derive(Debug, Clone)]
pub struct JsonlRunStore {
    path: PathBuf,
    entries_path: PathBuf,
}

impl JsonlRunStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, RunStoreError> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        Ok(Self {
            path: root.join("runs.jsonl"),
            entries_path: root.join("run_entries.jsonl"),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn entries_path(&self) -> &Path {
        &self.entries_path
    }
}

impl RunStore for JsonlRunStore {
    fn append(&mut self, run: &AgentRun) -> Result<(), RunStoreError> {
        self.append_checkpoint(
            run,
            stage_for_status(&run.status),
            run.audit.completed_at_ms.unwrap_or(run.created_at_ms),
        )
    }

    fn append_checkpoint(
        &mut self,
        run: &AgentRun,
        stage: impl Into<String>,
        recorded_at_ms: u64,
    ) -> Result<(), RunStoreError> {
        append_json_line(&self.path, run)?;
        append_json_line(
            &self.entries_path,
            &RunLedgerEntry {
                entry_id: Uuid::new_v4(),
                run_id: run.id,
                stage: stage.into(),
                status: run.status.clone(),
                recorded_at_ms,
                run: run.clone(),
            },
        )
    }

    fn list(&self) -> Result<Vec<AgentRun>, RunStoreError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut latest_by_id = BTreeMap::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let run: AgentRun = serde_json::from_str(&line)?;
            latest_by_id.insert(run.id, run);
        }
        let mut runs = latest_by_id.into_values().collect::<Vec<_>>();
        runs.sort_by_key(|run| (run.created_at_ms, run.id));
        Ok(runs)
    }

    fn get(&self, id: Uuid) -> Result<Option<AgentRun>, RunStoreError> {
        Ok(self.list()?.into_iter().find(|run| run.id == id))
    }

    fn entries_for_run(&self, id: Uuid) -> Result<Vec<RunLedgerEntry>, RunStoreError> {
        if !self.entries_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.entries_path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: RunLedgerEntry = serde_json::from_str(&line)?;
            if entry.run_id == id {
                entries.push(entry);
            }
        }
        entries.sort_by_key(|entry| (entry.recorded_at_ms, entry.entry_id));
        Ok(entries)
    }
}

fn append_json_line<T: Serialize>(path: &Path, value: &T) -> Result<(), RunStoreError> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut file, value)?;
    writeln!(file)?;
    Ok(())
}

fn stage_for_status(status: &RunStatus) -> &'static str {
    match status {
        RunStatus::Created => "created",
        RunStatus::AssemblingContext => "context",
        RunStatus::WaitingForProvider => "provider",
        RunStatus::WaitingForTool => "tool",
        RunStatus::Completed => "completed",
        RunStatus::BlockedByPolicy => "blocked",
        RunStatus::Failed => "failed",
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

    #[test]
    fn checkpoints_return_latest_snapshot_and_replay_entries() {
        let root = std::env::temp_dir().join(format!("indwell-runs-{}", uuid::Uuid::new_v4()));
        let mut store = JsonlRunStore::new(root).unwrap();
        let mut run = AgentRun::new(
            Event::BootCompleted,
            Some("hello".to_string()),
            AuthContext::anonymous(),
            DeviceState::Thinking,
            provider(),
            10,
        );

        store.append_checkpoint(&run, "created", 10).unwrap();
        run.record_retrieved_memory("mem_1");
        store.append_checkpoint(&run, "context", 20).unwrap();
        run.finish_with_summary("done", 30);
        store.append(&run).unwrap();

        let listed = store.list().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].status, indwell_core::RunStatus::Completed);
        assert_eq!(listed[0].audit.retrieved_memory_ids, ["mem_1"]);

        let fetched = store.get(run.id).unwrap().unwrap();
        assert_eq!(
            fetched.audit.provider_output_summary.as_deref(),
            Some("done")
        );

        let entries = store.entries_for_run(run.id).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.stage.as_str())
                .collect::<Vec<_>>(),
            ["created", "context", "completed"]
        );
    }
}
