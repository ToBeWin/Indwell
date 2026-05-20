use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

use chrono::{Datelike, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Sensitivity {
    Public,
    Personal,
    Private,
    Sensitive,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemorySource {
    UserSaid,
    DeviceEvent,
    AgentRun { run_id: String },
    Reflection,
    Imported,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TtlPolicy {
    Keep,
    Review,
    ExpireAtMs(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

impl MemoryRecord {
    pub fn new(
        kind: MemoryKind,
        wing: impl Into<String>,
        room: impl Into<String>,
        content: impl Into<String>,
        source: MemorySource,
        created_at_ms: u64,
    ) -> Self {
        let content = content.into();
        let hash = content_hash(&content);
        Self {
            id: Uuid::new_v4().to_string(),
            kind,
            wing: wing.into(),
            room: room.into(),
            content,
            source,
            verbatim_ref: None,
            confidence: 0.8,
            importance: 0.5,
            sensitivity: Sensitivity::Personal,
            created_at_ms,
            updated_at_ms: created_at_ms,
            last_used_at_ms: None,
            ttl_policy: TtlPolicy::Review,
            tags: Vec::new(),
            hash,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryQuery {
    pub wing: Option<String>,
    pub room: Option<String>,
    pub text: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryExport {
    pub records: Vec<MemoryRecord>,
    pub snapshots: MemorySnapshots,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct MemorySnapshots {
    pub persona: PersonaSnapshot,
    pub relationship: RelationshipSnapshot,
    pub index: MemoryIndexSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PersonaSnapshot {
    pub identities: Vec<SnapshotEntry>,
    pub preferences: Vec<SnapshotEntry>,
    pub emotional_patterns: Vec<SnapshotEntry>,
    pub safety_notes: Vec<SnapshotEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RelationshipSnapshot {
    pub relationships: Vec<SnapshotEntry>,
    pub recent_episodes: Vec<SnapshotEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct MemoryIndexSnapshot {
    pub total_records: usize,
    pub by_kind: BTreeMap<String, usize>,
    pub by_wing_room: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotEntry {
    pub record_id: String,
    pub wing: String,
    pub room: String,
    pub content: String,
    pub confidence: f32,
    pub importance: f32,
    pub sensitivity: Sensitivity,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryMetabolismReport {
    pub decayed: Vec<String>,
    pub expired: Vec<String>,
    pub consolidated: Vec<MemoryRecord>,
}

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

pub trait MemoryStore {
    fn append(&mut self, record: MemoryRecord) -> Result<(), MemoryError>;
    fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryRecord>, MemoryError>;
    fn get(&self, id: &str) -> Result<Option<MemoryRecord>, MemoryError>;
    fn delete(&mut self, id: &str) -> Result<(), MemoryError>;
    fn compact(&mut self) -> Result<(), MemoryError>;
    fn metabolize(&mut self, now_ms: u64) -> Result<MemoryMetabolismReport, MemoryError>;
    fn export(&self) -> Result<MemoryExport, MemoryError>;
}

#[derive(Debug, Clone)]
pub struct JsonlMemoryStore {
    root: PathBuf,
}

impl JsonlMemoryStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, MemoryError> {
        let root = root.into();
        fs::create_dir_all(root.join("drawers"))?;
        fs::create_dir_all(root.join("snapshots"))?;
        Ok(Self { root })
    }

    fn drawer_path(&self) -> PathBuf {
        let now = Utc::now();
        self.root.join("drawers").join(format!(
            "{:04}-{:02}-{:02}.jsonl",
            now.year(),
            now.month(),
            now.day()
        ))
    }

    fn all_records(&self) -> Result<Vec<MemoryRecord>, MemoryError> {
        let drawers = self.root.join("drawers");
        if !drawers.exists() {
            return Ok(Vec::new());
        }

        let mut paths = Vec::new();
        for entry in fs::read_dir(drawers)? {
            let path = entry?.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
                continue;
            }
            paths.push(path);
        }
        paths.sort();

        let mut records = BTreeMap::new();
        for path in paths {
            read_jsonl(&path, &mut records)?;
        }
        Ok(records.into_values().collect())
    }
}

impl MemoryStore for JsonlMemoryStore {
    fn append(&mut self, record: MemoryRecord) -> Result<(), MemoryError> {
        let path = self.drawer_path();
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        serde_json::to_writer(&mut file, &record)?;
        writeln!(file)?;
        Ok(())
    }

    fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryRecord>, MemoryError> {
        let needle = query.text.as_ref().map(|text| text.to_lowercase());
        let limit = query.limit.unwrap_or(20);
        let records = self.all_records()?;
        Ok(records
            .into_iter()
            .filter(|record| {
                query
                    .wing
                    .as_ref()
                    .map_or(true, |wing| &record.wing == wing)
            })
            .filter(|record| {
                query
                    .room
                    .as_ref()
                    .map_or(true, |room| &record.room == room)
            })
            .filter(|record| {
                needle
                    .as_ref()
                    .map_or(true, |needle| text_matches(&record.content, needle))
            })
            .take(limit)
            .collect())
    }

    fn get(&self, id: &str) -> Result<Option<MemoryRecord>, MemoryError> {
        Ok(self
            .all_records()?
            .into_iter()
            .find(|record| record.id == id))
    }

    fn delete(&mut self, id: &str) -> Result<(), MemoryError> {
        let path = self.drawer_path();
        let tombstone = MemoryTombstone::new(id);
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        serde_json::to_writer(&mut file, &tombstone)?;
        writeln!(file)?;
        Ok(())
    }

    fn compact(&mut self) -> Result<(), MemoryError> {
        let export = self.export()?;
        let snapshot_path = self.root.join("snapshots").join("memory_index.json");
        let mut file = File::create(snapshot_path)?;
        serde_json::to_writer_pretty(&mut file, &export)?;
        writeln!(file)?;

        let drawers = self.root.join("drawers");
        fs::create_dir_all(&drawers)?;
        let compacted_tmp = drawers.join("compacted.tmp");
        let compacted_path = drawers.join("compacted.jsonl");
        let mut compacted = File::create(&compacted_tmp)?;
        for record in &export.records {
            serde_json::to_writer(&mut compacted, record)?;
            writeln!(compacted)?;
        }
        compacted.flush()?;

        for entry in fs::read_dir(&drawers)? {
            let path = entry?.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
                fs::remove_file(path)?;
            }
        }
        fs::rename(compacted_tmp, compacted_path)?;
        Ok(())
    }

    fn metabolize(&mut self, now_ms: u64) -> Result<MemoryMetabolismReport, MemoryError> {
        let mut records = self.all_records()?;
        let report = metabolize_records(&mut records, now_ms);
        let expired = &report.expired;

        let drawers = self.root.join("drawers");
        fs::create_dir_all(&drawers)?;
        let metabolized_tmp = drawers.join("metabolized.tmp");
        let metabolized_path = drawers.join("compacted.jsonl");
        let mut file = File::create(&metabolized_tmp)?;
        for record in records
            .iter()
            .filter(|record| !expired.iter().any(|id| id == &record.id))
            .chain(report.consolidated.iter())
        {
            serde_json::to_writer(&mut file, record)?;
            writeln!(file)?;
        }
        file.flush()?;

        for entry in fs::read_dir(&drawers)? {
            let path = entry?.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
                fs::remove_file(path)?;
            }
        }
        fs::rename(metabolized_tmp, metabolized_path)?;
        Ok(report)
    }

    fn export(&self) -> Result<MemoryExport, MemoryError> {
        let records = self.all_records()?;
        Ok(MemoryExport {
            snapshots: build_snapshots(&records),
            records,
        })
    }
}

pub fn metabolize_records(records: &mut [MemoryRecord], now_ms: u64) -> MemoryMetabolismReport {
    let mut decayed = Vec::new();
    let mut expired = Vec::new();
    let mut preference_groups: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    const THIRTY_DAYS_MS: u64 = 30 * 24 * 60 * 60 * 1000;

    for record in records.iter_mut() {
        if matches!(record.ttl_policy, TtlPolicy::ExpireAtMs(expires_at) if now_ms >= expires_at) {
            expired.push(record.id.clone());
            continue;
        }

        if !matches!(record.ttl_policy, TtlPolicy::Keep)
            && record.last_used_at_ms.unwrap_or(record.created_at_ms) + THIRTY_DAYS_MS < now_ms
            && record.importance > 0.05
        {
            record.importance = (record.importance * 0.9).max(0.01);
            record.updated_at_ms = now_ms;
            decayed.push(record.id.clone());
        }

        if record.kind == MemoryKind::Preference && !expired.iter().any(|id| id == &record.id) {
            preference_groups
                .entry((record.wing.clone(), record.room.clone()))
                .or_default()
                .push(record.content.clone());
        }
    }

    let consolidated = preference_groups
        .into_iter()
        .filter(|(_, contents)| contents.len() >= 2)
        .map(|((wing, room), contents)| {
            let mut record = MemoryRecord::new(
                MemoryKind::Reflection,
                wing,
                "reflections",
                format!(
                    "Consolidated {} preference memories from room {room}: {}",
                    contents.len(),
                    contents.join(" | ")
                ),
                MemorySource::Reflection,
                now_ms,
            );
            record.importance = 0.7;
            record.confidence = 0.75;
            record.tags.push("consolidated".to_string());
            record
        })
        .collect();

    MemoryMetabolismReport {
        decayed,
        expired,
        consolidated,
    }
}

pub fn build_snapshots(records: &[MemoryRecord]) -> MemorySnapshots {
    let mut snapshots = MemorySnapshots::default();
    snapshots.index.total_records = records.len();

    for record in records {
        *snapshots
            .index
            .by_kind
            .entry(format!("{:?}", record.kind).to_lowercase())
            .or_insert(0) += 1;
        *snapshots
            .index
            .by_wing_room
            .entry(format!("{}/{}", record.wing, record.room))
            .or_insert(0) += 1;

        let entry = SnapshotEntry::from(record);
        match record.kind {
            MemoryKind::Identity => snapshots.persona.identities.push(entry),
            MemoryKind::Preference => snapshots.persona.preferences.push(entry),
            MemoryKind::Emotional => snapshots.persona.emotional_patterns.push(entry),
            MemoryKind::Safety => snapshots.persona.safety_notes.push(entry),
            MemoryKind::Relationship => snapshots.relationship.relationships.push(entry),
            MemoryKind::Episodic => snapshots.relationship.recent_episodes.push(entry),
            MemoryKind::Reflection | MemoryKind::Skill | MemoryKind::Environment => {}
        }
    }

    sort_snapshot_entries(&mut snapshots.persona.identities);
    sort_snapshot_entries(&mut snapshots.persona.preferences);
    sort_snapshot_entries(&mut snapshots.persona.emotional_patterns);
    sort_snapshot_entries(&mut snapshots.persona.safety_notes);
    sort_snapshot_entries(&mut snapshots.relationship.relationships);
    sort_snapshot_entries(&mut snapshots.relationship.recent_episodes);
    snapshots.relationship.recent_episodes.truncate(20);

    snapshots
}

impl From<&MemoryRecord> for SnapshotEntry {
    fn from(record: &MemoryRecord) -> Self {
        Self {
            record_id: record.id.clone(),
            wing: record.wing.clone(),
            room: record.room.clone(),
            content: record.content.clone(),
            confidence: record.confidence,
            importance: record.importance,
            sensitivity: record.sensitivity.clone(),
            updated_at_ms: record.updated_at_ms,
        }
    }
}

fn sort_snapshot_entries(entries: &mut [SnapshotEntry]) {
    entries.sort_by(|left, right| {
        right
            .importance
            .partial_cmp(&left.importance)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.updated_at_ms.cmp(&left.updated_at_ms))
    });
}

#[derive(Debug, Serialize)]
struct MemoryTombstone {
    op: &'static str,
    id: String,
    created_at_ms: u64,
}

impl MemoryTombstone {
    fn new(id: &str) -> Self {
        Self {
            op: "delete",
            id: id.to_string(),
            created_at_ms: Utc::now().timestamp_millis() as u64,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum MemoryLogEntry {
    Tombstone { op: String, id: String },
    Record(MemoryRecord),
}

fn read_jsonl(
    path: &Path,
    records: &mut BTreeMap<String, MemoryRecord>,
) -> Result<(), MemoryError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<MemoryLogEntry>(&line)? {
            MemoryLogEntry::Tombstone { op, id } if op == "delete" => {
                records.remove(&id);
            }
            MemoryLogEntry::Tombstone { .. } => {}
            MemoryLogEntry::Record(record) => {
                records.insert(record.id.clone(), record);
            }
        }
    }
    Ok(())
}

fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn text_matches(content: &str, needle: &str) -> bool {
    let content = content.to_lowercase();
    if content.contains(needle) {
        return true;
    }

    needle
        .split_whitespace()
        .filter(|token| token.len() >= 3)
        .any(|token| content.contains(token))
}

#[cfg(test)]
mod tests {
    use super::{
        build_snapshots, metabolize_records, JsonlMemoryStore, MemoryKind, MemoryQuery,
        MemoryRecord, MemorySource, MemoryStore, TtlPolicy,
    };

    #[test]
    fn append_and_search_jsonl_memory() {
        let root =
            std::env::temp_dir().join(format!("indwell-memory-test-{}", uuid::Uuid::new_v4()));
        let mut store = JsonlMemoryStore::new(&root).unwrap();
        let record = MemoryRecord::new(
            MemoryKind::Preference,
            "user_owner",
            "preferences",
            "User likes quiet morning study sessions.",
            MemorySource::UserSaid,
            1,
        );
        store.append(record).unwrap();

        let results = store
            .search(MemoryQuery {
                room: Some("preferences".to_string()),
                text: Some("morning".to_string()),
                ..MemoryQuery::default()
            })
            .unwrap();

        assert_eq!(results.len(), 1);
    }

    #[test]
    fn delete_hides_record_from_reads() {
        let root =
            std::env::temp_dir().join(format!("indwell-memory-test-{}", uuid::Uuid::new_v4()));
        let mut store = JsonlMemoryStore::new(&root).unwrap();
        let record = MemoryRecord::new(
            MemoryKind::Episodic,
            "user_owner",
            "episodes",
            "User asked for a replayable audit trail.",
            MemorySource::AgentRun {
                run_id: "run_123".to_string(),
            },
            1,
        );
        let id = record.id.clone();
        store.append(record).unwrap();

        assert!(store.get(&id).unwrap().is_some());
        store.delete(&id).unwrap();

        assert!(store.get(&id).unwrap().is_none());
        assert!(store.export().unwrap().records.is_empty());
    }

    #[test]
    fn compact_removes_deleted_records_from_drawers() {
        let root =
            std::env::temp_dir().join(format!("indwell-memory-test-{}", uuid::Uuid::new_v4()));
        let mut store = JsonlMemoryStore::new(&root).unwrap();
        let deleted = MemoryRecord::new(
            MemoryKind::Preference,
            "user_owner",
            "preferences",
            "User likes records that should be deleted.",
            MemorySource::UserSaid,
            1,
        );
        let kept = MemoryRecord::new(
            MemoryKind::Preference,
            "user_owner",
            "preferences",
            "User likes compact replay logs.",
            MemorySource::UserSaid,
            2,
        );
        let deleted_id = deleted.id.clone();
        let kept_id = kept.id.clone();
        store.append(deleted).unwrap();
        store.append(kept).unwrap();
        store.delete(&deleted_id).unwrap();
        store.compact().unwrap();

        let reopened = JsonlMemoryStore::new(&root).unwrap();
        assert!(reopened.get(&deleted_id).unwrap().is_none());
        assert_eq!(
            reopened.get(&kept_id).unwrap().unwrap().content,
            "User likes compact replay logs."
        );
        assert_eq!(reopened.export().unwrap().records.len(), 1);
    }

    #[test]
    fn search_matches_meaningful_terms_when_phrase_differs() {
        let root =
            std::env::temp_dir().join(format!("indwell-memory-test-{}", uuid::Uuid::new_v4()));
        let mut store = JsonlMemoryStore::new(&root).unwrap();
        let record = MemoryRecord::new(
            MemoryKind::Preference,
            "user_owner",
            "preferences",
            "User likes quiet lights when studying.",
            MemorySource::UserSaid,
            1,
        );
        store.append(record).unwrap();

        let results = store
            .search(MemoryQuery {
                text: Some("remember lights".to_string()),
                ..MemoryQuery::default()
            })
            .unwrap();

        assert_eq!(results.len(), 1);
    }

    #[test]
    fn metabolize_expires_decays_and_consolidates_records() {
        let now = 100 * 24 * 60 * 60 * 1000;
        let mut expired = MemoryRecord::new(
            MemoryKind::Episodic,
            "user_owner",
            "episodes",
            "Temporary event.",
            MemorySource::UserSaid,
            1,
        );
        expired.ttl_policy = TtlPolicy::ExpireAtMs(10);
        let mut stale = MemoryRecord::new(
            MemoryKind::Preference,
            "user_owner",
            "preferences",
            "User likes calm voice.",
            MemorySource::UserSaid,
            1,
        );
        stale.importance = 0.5;
        let another = MemoryRecord::new(
            MemoryKind::Preference,
            "user_owner",
            "preferences",
            "User likes short replies.",
            MemorySource::UserSaid,
            2,
        );
        let mut records = vec![expired, stale, another];

        let report = metabolize_records(&mut records, now);

        assert_eq!(report.expired.len(), 1);
        assert_eq!(report.decayed.len(), 2);
        assert_eq!(report.consolidated.len(), 1);
        assert_eq!(report.consolidated[0].kind, MemoryKind::Reflection);
    }

    #[test]
    fn export_builds_persona_and_relationship_snapshots() {
        let root =
            std::env::temp_dir().join(format!("indwell-memory-test-{}", uuid::Uuid::new_v4()));
        let mut store = JsonlMemoryStore::new(&root).unwrap();
        let identity = MemoryRecord::new(
            MemoryKind::Identity,
            "user_owner",
            "identity",
            "User is Bingo.",
            MemorySource::UserSaid,
            1,
        );
        let preference = MemoryRecord::new(
            MemoryKind::Preference,
            "user_owner",
            "preferences",
            "User prefers warm lights at night.",
            MemorySource::UserSaid,
            2,
        );
        let relationship = MemoryRecord::new(
            MemoryKind::Relationship,
            "user_owner",
            "relationships",
            "User calls the device Indwell.",
            MemorySource::Reflection,
            3,
        );
        store.append(identity).unwrap();
        store.append(preference).unwrap();
        store.append(relationship).unwrap();

        let export = store.export().unwrap();

        assert_eq!(export.snapshots.index.total_records, 3);
        assert_eq!(export.snapshots.persona.identities.len(), 1);
        assert_eq!(export.snapshots.persona.preferences.len(), 1);
        assert_eq!(export.snapshots.relationship.relationships.len(), 1);
    }

    #[test]
    fn deleted_records_do_not_enter_snapshots() {
        let root =
            std::env::temp_dir().join(format!("indwell-memory-test-{}", uuid::Uuid::new_v4()));
        let mut store = JsonlMemoryStore::new(&root).unwrap();
        let deleted = MemoryRecord::new(
            MemoryKind::Identity,
            "user_owner",
            "identity",
            "This identity should be forgotten.",
            MemorySource::UserSaid,
            1,
        );
        let id = deleted.id.clone();
        store.append(deleted).unwrap();
        store.delete(&id).unwrap();

        let export = store.export().unwrap();

        assert_eq!(export.snapshots.index.total_records, 0);
        assert!(export.snapshots.persona.identities.is_empty());
    }

    #[test]
    fn snapshot_builder_sorts_by_importance() {
        let mut low = MemoryRecord::new(
            MemoryKind::Preference,
            "user_owner",
            "preferences",
            "Low priority preference.",
            MemorySource::UserSaid,
            1,
        );
        low.importance = 0.1;
        let mut high = MemoryRecord::new(
            MemoryKind::Preference,
            "user_owner",
            "preferences",
            "High priority preference.",
            MemorySource::UserSaid,
            2,
        );
        high.importance = 0.9;

        let snapshots = build_snapshots(&[low, high]);

        assert_eq!(
            snapshots.persona.preferences[0].content,
            "High priority preference."
        );
    }
}
