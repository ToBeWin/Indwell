use indwell_core::ToolDescriptor;
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct PlannedToolCall {
    pub tool: String,
    pub input: Value,
}

pub fn plan_tool_calls(text: &str, allowed_tools: &[ToolDescriptor]) -> Vec<PlannedToolCall> {
    let lowered = text.to_lowercase();
    let mut plans = Vec::new();
    let memory_intent = contains_any(
        &lowered,
        &["remember", "save", "note", "记住", "保存", "记录"],
    );

    if is_allowed(allowed_tools, "memory.write_candidate") && memory_intent {
        plans.push(PlannedToolCall {
            tool: "memory.write_candidate".to_string(),
            input: json!({
                "wing": "user_unknown",
                "room": "episodes",
                "content": memory_candidate_content(text),
            }),
        });
    }

    if is_allowed(allowed_tools, "system.status")
        && contains_any(&lowered, &["status", "health", "system", "状态", "系统"])
    {
        plans.push(PlannedToolCall {
            tool: "system.status".to_string(),
            input: json!({}),
        });
    }

    if is_allowed(allowed_tools, "system.update.check")
        && contains_any(
            &lowered,
            &["check update", "update", "version", "更新", "版本"],
        )
    {
        plans.push(PlannedToolCall {
            tool: "system.update.check".to_string(),
            input: json!({}),
        });
    }

    if is_allowed(allowed_tools, "device.camera.capture")
        && contains_any(
            &lowered,
            &[
                "capture camera",
                "camera",
                "photo",
                "picture",
                "拍照",
                "看看",
            ],
        )
    {
        plans.push(PlannedToolCall {
            tool: "device.camera.capture".to_string(),
            input: json!({
                "analyze": wants_vision_analysis(&lowered),
                "prompt": "Describe the captured scene for the Indwell user."
            }),
        });
    }

    if is_allowed(allowed_tools, "device.sensor.read")
        && contains_any(
            &lowered,
            &[
                "sensor",
                "temperature",
                "light level",
                "传感器",
                "温度",
                "光照",
            ],
        )
    {
        plans.push(PlannedToolCall {
            tool: "device.sensor.read".to_string(),
            input: json!({
                "sensor": infer_sensor(&lowered),
            }),
        });
    }

    if is_allowed(allowed_tools, "device.led.set")
        && (!memory_intent || has_explicit_led_control_intent(&lowered))
        && contains_any(&lowered, &["led", "light", "color", "灯", "灯光", "颜色"])
    {
        plans.push(PlannedToolCall {
            tool: "device.led.set".to_string(),
            input: json!({
                "color": infer_color(&lowered),
            }),
        });
    }

    plans
}

fn is_allowed(allowed_tools: &[ToolDescriptor], tool_name: &str) -> bool {
    allowed_tools.iter().any(|tool| tool.name == tool_name)
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn memory_candidate_content(text: &str) -> String {
    let trimmed = text.trim();
    for prefix in [
        "remember that",
        "remember",
        "save that",
        "save",
        "note that",
        "note",
        "记住",
        "保存",
        "记录",
    ] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            let rest = rest.trim_matches(|ch: char| ch == ':' || ch == '：' || ch.is_whitespace());
            if !rest.is_empty() {
                return rest.to_string();
            }
        }
    }

    trimmed.to_string()
}

fn infer_color(text: &str) -> &'static str {
    if contains_any(text, &["red", "红"]) {
        "red"
    } else if contains_any(text, &["blue", "蓝"]) {
        "blue"
    } else if contains_any(text, &["yellow", "黄"]) {
        "yellow"
    } else if contains_any(text, &["white", "白"]) {
        "white"
    } else {
        "green"
    }
}

fn infer_sensor(text: &str) -> &'static str {
    if contains_any(text, &["temperature", "温度"]) {
        "temperature"
    } else if contains_any(text, &["imu", "motion", "姿态", "移动"]) {
        "imu"
    } else {
        "ambient_light"
    }
}

fn has_explicit_led_control_intent(text: &str) -> bool {
    contains_any(
        text,
        &[
            "set", "turn", "change", "switch", "make", "调", "调成", "设置", "改成", "切换",
        ],
    )
}

fn wants_vision_analysis(text: &str) -> bool {
    contains_any(
        text,
        &[
            "see",
            "look",
            "what is",
            "what's",
            "describe",
            "看到",
            "看看",
            "看见",
            "描述",
            "这是什么",
        ],
    )
}

#[cfg(test)]
mod tests {
    use indwell_core::{RiskLevel, ToolDescriptor};

    use super::plan_tool_calls;

    fn tools(names: &[&str]) -> Vec<ToolDescriptor> {
        names
            .iter()
            .map(|name| ToolDescriptor::new(*name, "test", RiskLevel::Safe))
            .collect()
    }

    #[test]
    fn plans_memory_write_for_remember_intent() {
        let plans = plan_tool_calls(
            "remember that I like quiet lights",
            &tools(&["memory.write_candidate"]),
        );

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].tool, "memory.write_candidate");
        assert_eq!(plans[0].input["content"], "I like quiet lights");
    }

    #[test]
    fn does_not_plan_disallowed_tool() {
        let plans = plan_tool_calls("remember that I like quiet lights", &[]);
        assert!(plans.is_empty());
    }

    #[test]
    fn plans_chinese_light_color() {
        let plans = plan_tool_calls("把灯光调成蓝色", &tools(&["device.led.set"]));

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].tool, "device.led.set");
        assert_eq!(plans[0].input["color"], "blue");
    }

    #[test]
    fn does_not_change_led_for_memory_about_lights() {
        let plans = plan_tool_calls(
            "remember that I like warm quiet light",
            &tools(&["memory.write_candidate", "device.led.set"]),
        );

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].tool, "memory.write_candidate");
    }

    #[test]
    fn plans_camera_for_capture_command() {
        let plans = plan_tool_calls("capture camera image", &tools(&["device.camera.capture"]));

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].tool, "device.camera.capture");
        assert_eq!(plans[0].input["analyze"], false);
    }

    #[test]
    fn plans_camera_with_vision_analysis_for_look_command() {
        let plans = plan_tool_calls("看看桌面上有什么", &tools(&["device.camera.capture"]));

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].tool, "device.camera.capture");
        assert_eq!(plans[0].input["analyze"], true);
    }

    #[test]
    fn plans_sensor_read_for_temperature() {
        let plans = plan_tool_calls("read temperature sensor", &tools(&["device.sensor.read"]));

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].input["sensor"], "temperature");
    }
}
