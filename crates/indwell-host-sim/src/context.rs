use indwell_channel::{ChannelKind, ChannelPolicy};
use indwell_core::{default_tools, AgentRun, AuthContext, DeviceState, ToolDescriptor};
use indwell_memory::{MemoryQuery, MemoryRecord};
use indwell_provider::{ChatMessage, ChatRequest, ToolSpec};
use indwell_security::{PolicyDecision, PolicyEngine};
use serde_json::{json, Value};

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

pub fn contextual_chat_request(run: &AgentRun, user_text: &str) -> ChatRequest {
    let mut messages = vec![ChatMessage {
        role: "system".to_string(),
        content: system_contract(),
    }];

    if let Some(persona) = run.context_pack.persona_snapshot.as_deref() {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: format!("Persona Snapshot:\n{}", compact_text(persona, 600)),
        });
    }

    messages.push(ChatMessage {
        role: "system".to_string(),
        content: format!(
            "Current Device State: {:?}",
            run.context_pack.current_device_state
        ),
    });

    if !run.context_pack.retrieved_memories.is_empty() {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: format!(
                "Retrieved Memories:\n{}",
                compact_lines(&run.context_pack.retrieved_memories, 6, 900)
            ),
        });
    }

    if !run.context_pack.recent_turns.is_empty() {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: format!(
                "Recent Turns:\n{}",
                compact_lines(&run.context_pack.recent_turns, 6, 900)
            ),
        });
    }

    if !run.audit.policy_blocks.is_empty() {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: format!(
                "Policy Blocks:\n{}",
                compact_lines(&run.audit.policy_blocks, 4, 500)
            ),
        });
    }

    messages.push(ChatMessage {
        role: "user".to_string(),
        content: user_text.to_string(),
    });

    ChatRequest {
        messages,
        tools: tool_specs_from_descriptors(&run.allowed_tools),
    }
}

pub fn tool_specs_from_descriptors(tools: &[ToolDescriptor]) -> Vec<ToolSpec> {
    tools
        .iter()
        .map(|tool| ToolSpec {
            name: tool.name.clone(),
            description: tool.description.clone(),
            input_schema: tool_input_schema(&tool.name),
        })
        .collect()
}

pub fn tool_input_schema(tool: &str) -> Value {
    match tool {
        "device.led.set" => json!({
            "type": "object",
            "properties": { "color": { "type": "string" } },
            "required": ["color"],
        }),
        "device.speaker.speak" => json!({
            "type": "object",
            "properties": { "text": { "type": "string" } },
            "required": ["text"],
        }),
        "memory.search" => json!({
            "type": "object",
            "properties": {
                "wing": { "type": ["string", "null"] },
                "room": { "type": ["string", "null"] },
                "text": { "type": ["string", "null"] },
                "limit": { "type": "integer", "minimum": 1, "maximum": 100 }
            },
        }),
        "memory.write_candidate" => json!({
            "type": "object",
            "properties": {
                "wing": { "type": "string" },
                "room": { "type": "string" },
                "content": { "type": "string" }
            },
            "required": ["content"],
        }),
        "memory.delete" => json!({
            "type": "object",
            "properties": { "id": { "type": "string" } },
            "required": ["id"],
        }),
        "device.camera.capture" => json!({
            "type": "object",
            "properties": {
                "analyze": { "type": "boolean" },
                "prompt": { "type": "string" }
            },
        }),
        "device.sensor.read" => json!({
            "type": "object",
            "properties": { "sensor": { "type": "string" } },
        }),
        _ => json!({ "type": "object" }),
    }
}

fn system_contract() -> String {
    [
        "You are the Indwell OS companion agent running from a local-first device runtime.",
        "Use only the relevant provided memories and tools; do not assume full history is available.",
        "For sensitive or high-risk actions, prefer confirmation and explain the safety boundary briefly.",
        "Keep replies concise and useful for an embodied device.",
    ]
    .join(" ")
}

fn compact_lines(lines: &[String], max_lines: usize, max_chars: usize) -> String {
    let mut rendered = String::new();
    for (index, line) in lines.iter().take(max_lines).enumerate() {
        if !rendered.is_empty() {
            rendered.push('\n');
        }
        rendered.push_str("- ");
        rendered.push_str(&compact_text(
            line,
            max_chars.saturating_sub(rendered.len()),
        ));
        if rendered.len() >= max_chars {
            break;
        }
        if index + 1 == max_lines && lines.len() > max_lines {
            rendered.push_str("\n- ...");
        }
    }
    rendered
}

fn compact_text(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.len() <= max_chars {
        return trimmed.to_string();
    }

    let end = trimmed
        .char_indices()
        .take_while(|(index, _)| *index < max_chars.saturating_sub(1))
        .last()
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    format!("{}...", &trimmed[..end])
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
    use indwell_core::{AgentRun, AuthContext, DeviceState, Event, ProviderSelection};
    use indwell_memory::{MemoryKind, MemoryRecord, MemorySource};
    use indwell_security::PolicyEngine;

    use super::{apply_context_pack, contextual_chat_request, ContextAssembler};

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

    #[test]
    fn contextual_chat_request_includes_memory_and_keeps_user_text_last() {
        let assembler = ContextAssembler::default();
        let memory = MemoryRecord::new(
            MemoryKind::Preference,
            "user_owner",
            "preferences",
            "User prefers quiet blue lights after dinner.",
            MemorySource::UserSaid,
            1,
        );
        let assembly = assembler.assemble(
            "remember my lighting preference",
            ChannelKind::LocalPwa,
            vec![memory],
            &PolicyEngine,
            &AuthContext::anonymous(),
        );
        let mut run = AgentRun::new(
            Event::ChannelMessage {
                channel: "local_pwa".to_string(),
                session_id: "session-1".to_string(),
            },
            Some("remember my lighting preference".to_string()),
            AuthContext::anonymous(),
            DeviceState::Thinking,
            ProviderSelection {
                llm: "mock:phase0".to_string(),
                vision: None,
                asr: None,
                tts: None,
                embedding: None,
            },
            1,
        );
        apply_context_pack(&mut run, DeviceState::Thinking, assembly);

        let request = contextual_chat_request(&run, "remember my lighting preference");
        let joined = request
            .messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(joined.contains("Host simulator persona"));
        assert!(joined.contains("Current Device State: Thinking"));
        assert!(joined.contains("quiet blue lights"));
        assert_eq!(
            request.messages.last().map(|message| message.role.as_str()),
            Some("user")
        );
        assert_eq!(
            request
                .messages
                .last()
                .map(|message| message.content.as_str()),
            Some("remember my lighting preference")
        );
        assert!(request
            .tools
            .iter()
            .any(|tool| tool.name == "memory.write_candidate"));
    }
}
