use indwell_channel::{ChannelKind, ChannelPolicy};
use indwell_core::{default_tools, AuthContext, DeviceState, ToolDescriptor};
use indwell_memory::{MemoryQuery, MemoryRecord};
use indwell_security::{PolicyDecision, PolicyEngine};

#[derive(Debug, Clone)]
pub struct ContextAssembly {
    pub persona_snapshot: Option<String>,
    pub recent_turns: Vec<String>,
    pub retrieved_memories: Vec<MemoryRecord>,
    pub allowed_tools: Vec<ToolDescriptor>,
    pub policy_blocks: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ContextAssembler {
    max_memories: usize,
}

impl Default for ContextAssembler {
    fn default() -> Self {
        Self { max_memories: 5 }
    }
}

impl ContextAssembler {
    pub fn memory_query(&self, text: &str) -> MemoryQuery {
        MemoryQuery {
            wing: None,
            room: None,
            text: meaningful_query_text(text),
            limit: Some(self.max_memories),
        }
    }

    pub fn assemble(
        &self,
        text: &str,
        channel: ChannelKind,
        memories: Vec<MemoryRecord>,
        policy: &PolicyEngine,
        auth: &AuthContext,
    ) -> ContextAssembly {
        let channel_policy = ChannelPolicy::default_for(channel);
        let mut allowed_tools = Vec::new();
        let mut policy_blocks = Vec::new();
        for tool in default_tools()
            .into_iter()
            .filter(|tool| relevant_to_text(tool, text))
        {
            match policy.evaluate_tool(&tool, auth, &channel_policy) {
                PolicyDecision::Allow => allowed_tools.push(tool),
                decision => policy_blocks.push(format!("{} blocked: {decision:?}", tool.name)),
            }
        }

        ContextAssembly {
            persona_snapshot: Some(
                "Host simulator persona: concise, local-first, safe by default.".to_string(),
            ),
            recent_turns: Vec::new(),
            retrieved_memories: memories,
            allowed_tools,
            policy_blocks,
        }
    }
}

pub fn apply_context_pack(
    run: &mut indwell_core::AgentRun,
    device_state: DeviceState,
    assembly: ContextAssembly,
) {
    run.context_pack.persona_snapshot = assembly.persona_snapshot;
    run.context_pack.current_device_state = device_state;
    run.context_pack.recent_turns = assembly.recent_turns;
    run.context_pack.retrieved_memories = assembly
        .retrieved_memories
        .iter()
        .map(|record| format!("{}:{}: {}", record.wing, record.room, record.content))
        .collect();
    run.allowed_tools = assembly.allowed_tools;

    for memory in assembly.retrieved_memories {
        run.record_retrieved_memory(memory.id);
    }

    for block in assembly.policy_blocks {
        run.record_policy_block(block);
    }
}

fn meaningful_query_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.len() < 3 {
        return None;
    }

    Some(
        trimmed
            .split_whitespace()
            .take(8)
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn relevant_to_text(tool: &ToolDescriptor, text: &str) -> bool {
    let lowered = text.to_lowercase();
    match tool.name.as_str() {
        "memory.search" => contains_any(
            &lowered,
            &[
                "remember",
                "memory",
                "recall",
                "last time",
                "记住",
                "记忆",
                "回忆",
            ],
        ),
        "memory.write_candidate" => contains_any(
            &lowered,
            &["remember", "save", "note", "记住", "保存", "记录"],
        ),
        "memory.delete" => contains_any(&lowered, &["delete memory", "forget", "删除记忆", "忘记"]),
        "device.camera.capture" => contains_any(
            &lowered,
            &["see", "look", "photo", "picture", "camera", "看看", "拍照"],
        ),
        "device.sensor.read" => contains_any(
            &lowered,
            &[
                "sensor",
                "temperature",
                "light level",
                "传感器",
                "温度",
                "光照",
            ],
        ),
        "device.speaker.speak" => contains_any(&lowered, &["say", "speak", "voice"]),
        "system.status" => contains_any(&lowered, &["status", "health", "system", "状态", "系统"]),
        "system.update.check" => contains_any(&lowered, &["update", "version", "更新", "版本"]),
        "system.update.apply" => contains_any(&lowered, &["apply update", "install update"]),
        "identity.whoami" => contains_any(&lowered, &["who am i", "identity", "owner"]),
        "device.led.set" => {
            contains_any(&lowered, &["led", "light", "color", "灯", "灯光", "颜色"])
        }
        "auth.request_confirmation" => contains_any(&lowered, &["confirm", "approve"]),
        _ => false,
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

#[cfg(test)]
mod tests {
    use indwell_channel::ChannelKind;
    use indwell_core::AuthContext;
    use indwell_memory::{MemoryKind, MemoryRecord, MemorySource};
    use indwell_security::PolicyEngine;

    use super::ContextAssembler;

    #[test]
    fn local_pwa_gets_relevant_memory_tools() {
        let assembler = ContextAssembler::default();
        let assembly = assembler.assemble(
            "remember that I like quiet lights",
            ChannelKind::LocalPwa,
            Vec::new(),
            &PolicyEngine,
            &AuthContext::anonymous(),
        );

        let names: Vec<_> = assembly
            .allowed_tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect();

        assert!(names.contains(&"memory.write_candidate"));
        assert!(names.contains(&"memory.search"));
    }

    #[test]
    fn telegram_does_not_get_camera_tool_even_if_relevant() {
        let assembler = ContextAssembler::default();
        let assembly = assembler.assemble(
            "look through the camera",
            ChannelKind::Telegram,
            Vec::new(),
            &PolicyEngine,
            &AuthContext::anonymous(),
        );

        assert!(!assembly
            .allowed_tools
            .iter()
            .any(|tool| tool.name == "device.camera.capture"));
        assert_eq!(assembly.policy_blocks.len(), 1);
        assert!(assembly.policy_blocks[0].contains("device.camera.capture"));
    }

    #[test]
    fn keeps_retrieved_memory_records_for_audit() {
        let assembler = ContextAssembler::default();
        let memory = MemoryRecord::new(
            MemoryKind::Preference,
            "user_owner",
            "preferences",
            "User likes quiet lights.",
            MemorySource::UserSaid,
            1,
        );

        let assembly = assembler.assemble(
            "remember lights",
            ChannelKind::LocalPwa,
            vec![memory],
            &PolicyEngine,
            &AuthContext::anonymous(),
        );

        assert_eq!(assembly.retrieved_memories.len(), 1);
    }
}
