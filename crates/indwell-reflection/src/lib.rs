use indwell_memory::{MemoryKind, MemoryRecord, MemorySource, Sensitivity, TtlPolicy};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReflectionInput {
    pub source_records: Vec<MemoryRecord>,
    pub now_ms: u64,
    pub budget: ReflectionBudget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReflectionBudget {
    pub max_new_memories: usize,
    pub allow_sensitive: bool,
    pub allow_skill_generation: bool,
}

impl Default for ReflectionBudget {
    fn default() -> Self {
        Self {
            max_new_memories: 8,
            allow_sensitive: false,
            allow_skill_generation: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReflectionReport {
    pub new_memories: Vec<MemoryRecord>,
    pub skills: Vec<SkillTemplate>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillTemplate {
    pub name: String,
    pub trigger: String,
    pub steps: Vec<String>,
    pub source_record_ids: Vec<String>,
}

#[derive(Debug, Error)]
pub enum ReflectionError {
    #[error("reflection budget allows no new memories")]
    EmptyBudget,
}

#[derive(Debug, Default, Clone)]
pub struct ReflectionEngine;

impl ReflectionEngine {
    pub fn reflect(&self, input: ReflectionInput) -> Result<ReflectionReport, ReflectionError> {
        if input.budget.max_new_memories == 0 {
            return Err(ReflectionError::EmptyBudget);
        }

        let mut report = ReflectionReport {
            new_memories: Vec::new(),
            skills: Vec::new(),
            warnings: Vec::new(),
        };

        for record in &input.source_records {
            if report.new_memories.len() >= input.budget.max_new_memories {
                break;
            }
            if is_sensitive(record) && !input.budget.allow_sensitive {
                report
                    .warnings
                    .push(format!("skipped sensitive source {}", record.id));
                continue;
            }

            if let Some(memory) = preference_from_text(record, input.now_ms) {
                report.new_memories.push(memory);
            } else if let Some(memory) = relationship_from_text(record, input.now_ms) {
                report.new_memories.push(memory);
            } else if let Some(memory) = emotional_from_text(record, input.now_ms) {
                report.new_memories.push(memory);
            }

            if input.budget.allow_skill_generation {
                if let Some(skill) = skill_from_text(record) {
                    report.skills.push(skill);
                }
            }
        }

        Ok(report)
    }
}

fn is_sensitive(record: &MemoryRecord) -> bool {
    matches!(
        record.sensitivity,
        Sensitivity::Sensitive | Sensitivity::Critical
    )
}

fn preference_from_text(record: &MemoryRecord, now_ms: u64) -> Option<MemoryRecord> {
    let lowered = record.content.to_lowercase();
    if !contains_any(&lowered, &["i like", "i prefer", "我喜欢", "偏好"]) {
        return None;
    }
    Some(derived_memory(
        MemoryKind::Preference,
        "preferences",
        format!("Derived preference from {}: {}", record.id, record.content),
        record,
        now_ms,
    ))
}

fn relationship_from_text(record: &MemoryRecord, now_ms: u64) -> Option<MemoryRecord> {
    let lowered = record.content.to_lowercase();
    if !contains_any(
        &lowered,
        &[
            "my friend",
            "my mother",
            "my father",
            "朋友",
            "妈妈",
            "爸爸",
        ],
    ) {
        return None;
    }
    Some(derived_memory(
        MemoryKind::Relationship,
        "relationships",
        format!(
            "Derived relationship note from {}: {}",
            record.id, record.content
        ),
        record,
        now_ms,
    ))
}

fn emotional_from_text(record: &MemoryRecord, now_ms: u64) -> Option<MemoryRecord> {
    let lowered = record.content.to_lowercase();
    if !contains_any(
        &lowered,
        &["anxious", "sad", "happy", "焦虑", "难过", "开心"],
    ) {
        return None;
    }
    Some(derived_memory(
        MemoryKind::Emotional,
        "emotions",
        format!(
            "Derived emotional pattern from {}: {}",
            record.id, record.content
        ),
        record,
        now_ms,
    ))
}

fn derived_memory(
    kind: MemoryKind,
    room: &str,
    content: String,
    source: &MemoryRecord,
    now_ms: u64,
) -> MemoryRecord {
    let mut memory = MemoryRecord::new(
        kind,
        source.wing.clone(),
        room,
        content,
        MemorySource::Reflection,
        now_ms,
    );
    memory.confidence = 0.65;
    memory.importance = (source.importance + 0.15).min(1.0);
    memory.sensitivity = source.sensitivity.clone();
    memory.ttl_policy = TtlPolicy::Review;
    memory.tags.push(format!("source:{}", source.id));
    memory
}

fn skill_from_text(record: &MemoryRecord) -> Option<SkillTemplate> {
    let lowered = record.content.to_lowercase();
    if !contains_any(&lowered, &["when i ask", "every time", "每次", "当我"]) {
        return None;
    }
    Some(SkillTemplate {
        name: format!(
            "skill_from_{}",
            record.id.chars().take(8).collect::<String>()
        ),
        trigger: record.content.clone(),
        steps: vec![
            "Match the user's repeated request pattern.".to_string(),
            "Assemble relevant memories and safe tools.".to_string(),
            "Ask for confirmation before medium/high risk actions.".to_string(),
        ],
        source_record_ids: vec![record.id.clone()],
    })
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

#[cfg(test)]
mod tests {
    use indwell_memory::{MemoryKind, MemoryRecord, MemorySource, Sensitivity};

    use super::{ReflectionBudget, ReflectionEngine, ReflectionError, ReflectionInput};

    fn episodic(content: &str) -> MemoryRecord {
        MemoryRecord::new(
            MemoryKind::Episodic,
            "user_owner",
            "episodes",
            content,
            MemorySource::UserSaid,
            1,
        )
    }

    #[test]
    fn derives_preference_relationship_emotion_and_skill() {
        let engine = ReflectionEngine;
        let input = ReflectionInput {
            source_records: vec![
                episodic("I like quiet warm lights."),
                episodic("My mother visits on Sundays."),
                episodic("I feel anxious before exams."),
                episodic("When I ask for study mode, set a calm voice."),
            ],
            now_ms: 100,
            budget: ReflectionBudget::default(),
        };

        let report = engine.reflect(input).unwrap();

        assert!(report
            .new_memories
            .iter()
            .any(|memory| memory.kind == MemoryKind::Preference));
        assert!(report
            .new_memories
            .iter()
            .any(|memory| memory.kind == MemoryKind::Relationship));
        assert!(report
            .new_memories
            .iter()
            .any(|memory| memory.kind == MemoryKind::Emotional));
        assert_eq!(report.skills.len(), 1);
    }

    #[test]
    fn skips_sensitive_records_without_budget() {
        let engine = ReflectionEngine;
        let mut sensitive = episodic("I like something private.");
        sensitive.sensitivity = Sensitivity::Sensitive;
        let report = engine
            .reflect(ReflectionInput {
                source_records: vec![sensitive],
                now_ms: 100,
                budget: ReflectionBudget::default(),
            })
            .unwrap();

        assert!(report.new_memories.is_empty());
        assert_eq!(report.warnings.len(), 1);
    }

    #[test]
    fn rejects_empty_budget() {
        let engine = ReflectionEngine;
        let error = engine
            .reflect(ReflectionInput {
                source_records: vec![episodic("I like tea.")],
                now_ms: 100,
                budget: ReflectionBudget {
                    max_new_memories: 0,
                    allow_sensitive: false,
                    allow_skill_generation: false,
                },
            })
            .unwrap_err();

        assert!(matches!(error, ReflectionError::EmptyBudget));
    }
}
