use indwell_core::{default_tools, ToolDescriptor};
use indwell_provider::ToolSpec;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize)]
pub struct HostToolCatalogItem {
    pub name: String,
    pub description: String,
    pub risk: indwell_core::RiskLevel,
    pub requires_owner: bool,
    pub requires_confirmation: bool,
    pub input_schema: Value,
    pub output_schema: Value,
}

pub fn lookup_tool(name: &str) -> ToolDescriptor {
    default_tools()
        .into_iter()
        .find(|candidate| candidate.name == name)
        .unwrap_or_else(|| {
            ToolDescriptor::new(
                name.to_string(),
                "Unknown tool.",
                indwell_core::RiskLevel::Forbidden,
            )
        })
}

pub fn tool_catalog() -> Vec<HostToolCatalogItem> {
    default_tools()
        .into_iter()
        .map(|tool| HostToolCatalogItem {
            input_schema: tool_input_schema(&tool.name),
            output_schema: tool_output_schema(&tool.name),
            name: tool.name,
            description: tool.description,
            risk: tool.risk,
            requires_owner: tool.requires_owner,
            requires_confirmation: tool.requires_confirmation,
        })
        .collect()
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

pub fn tool_output_schema(tool: &str) -> Value {
    match tool {
        "system.status" => json!({
            "type": "object",
            "properties": {
                "state": { "type": "string" },
                "network": { "type": "string" },
                "provider": { "type": "object" },
                "memory_backend": { "type": "string" }
            }
        }),
        "system.update.check" => json!({
            "type": "object",
            "properties": {
                "update_available": { "type": "boolean" },
                "current_version": { "type": "string" },
                "manifest": { "type": "object" },
                "verification": { "type": "object" }
            }
        }),
        "device.camera.capture" => json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "width": { "type": "integer" },
                "height": { "type": "integer" },
                "mime_type": { "type": "string" },
                "byte_len": { "type": "integer" },
                "retention": { "type": "string" },
                "analyzed": { "type": "boolean" },
                "vision": { "type": ["object", "null"] }
            }
        }),
        "device.sensor.read" => json!({
            "type": "object",
            "properties": {
                "sensor": { "type": "string" },
                "value": { "type": "object" }
            }
        }),
        _ => json!({ "type": "object" }),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use indwell_core::default_tools;

    use super::{lookup_tool, tool_catalog};

    #[test]
    fn catalog_covers_default_tools_with_unique_names_and_schemas() {
        let default_names = default_tools()
            .into_iter()
            .map(|tool| tool.name)
            .collect::<BTreeSet<_>>();
        let catalog = tool_catalog();
        let catalog_names = catalog
            .iter()
            .map(|tool| tool.name.clone())
            .collect::<BTreeSet<_>>();

        assert_eq!(catalog_names, default_names);
        assert_eq!(catalog.len(), catalog_names.len());
        assert!(catalog.iter().all(|tool| tool.input_schema.is_object()));
        assert!(catalog.iter().all(|tool| tool.output_schema.is_object()));
    }

    #[test]
    fn unknown_tool_lookup_is_forbidden() {
        let descriptor = lookup_tool("unknown.experimental.tool");

        assert_eq!(descriptor.risk, indwell_core::RiskLevel::Forbidden);
        assert!(descriptor.requires_confirmation);
    }
}
