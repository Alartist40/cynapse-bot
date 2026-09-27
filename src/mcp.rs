use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum HardwareAction {
    MoveServo { pan: i32, tilt: i32 },
    SetFace { expression: String },
    PlayAnimation { animation: String },
    SetLedColor { red: u8, green: u8, blue: u8 },
    CreateReminder { duration_seconds: u32, message: String, repeat: bool },
    StopReminder { id: i32 },
    GetHeadAngles,
    GetReminders,
    EmergencyStop,
    Custom { name: String, args: Value },
}

#[derive(Clone)]
pub struct McpRegistry {
    tools: Arc<RwLock<HashMap<String, ToolDefinition>>>,
}

impl Default for McpRegistry {
    fn default() -> Self {
        let mut registry = Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        };
        registry.register_defaults();
        registry
    }
}

impl McpRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn register_defaults(&mut self) {
        let tools = vec![
            ToolDefinition {
                name: "self.robot.get_head_angles".to_string(),
                description: "Returns current yaw/pitch in degrees. Neutral position is {yaw:0, pitch:0}.".to_string(),
                parameters: json!({"type": "object", "properties": {}}),
            },
            ToolDefinition {
                name: "self.robot.set_head_angles".to_string(),
                description: "Adjust head position. Yaw(-128 to 128), Pitch(0 to 90), Speed(100 to 1000).".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "yaw": {"type": "integer", "minimum": -128, "maximum": 128},
                        "pitch": {"type": "integer", "minimum": 0, "maximum": 90},
                        "speed": {"type": "integer", "default": 150}
                    }
                }),
            },
            ToolDefinition {
                name: "self.robot.set_led_color".to_string(),
                description: "Set the color of internal onboard LEDs. Values: 0-168 (safe range). Red/Green/Blue.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "red": {"type": "integer", "minimum": 0, "maximum": 168},
                        "green": {"type": "integer", "minimum": 0, "maximum": 168},
                        "blue": {"type": "integer", "minimum": 0, "maximum": 168}
                    },
                    "required": ["red", "green", "blue"]
                }),
            },
            ToolDefinition {
                name: "self.robot.create_reminder".to_string(),
                description: "Create a reminder timer on the robot.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "duration_seconds": {"type": "integer", "minimum": 1, "maximum": 86400},
                        "message": {"type": "string"},
                        "repeat": {"type": "boolean", "default": false}
                    },
                    "required": ["duration_seconds", "message"]
                }),
            },
            ToolDefinition {
                name: "self.robot.get_reminders".to_string(),
                description: "Get list of active reminders.".to_string(),
                parameters: json!({"type": "object", "properties": {}}),
            },
            ToolDefinition {
                name: "self.robot.stop_reminder".to_string(),
                description: "Stop a reminder by ID.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "integer"}
                    },
                    "required": ["id"]
                }),
            },
        ];

        let mut lock = self.tools.blocking_write();
        for tool in tools {
            lock.insert(tool.name.clone(), tool);
        }
    }

    /// Register a custom user tool dynamically
    pub async fn register_custom_tool(&self, tool: ToolDefinition) {
        let mut lock = self.tools.write().await;
        lock.insert(tool.name.clone(), tool);
    }

    /// Get all registered tool definitions for LLM system context
    pub async fn list_tools(&self) -> Vec<ToolDefinition> {
        let lock = self.tools.read().await;
        lock.values().cloned().collect()
    }
}

pub struct McpDispatcher;

impl McpDispatcher {
    pub fn format_mcp_call(action: HardwareAction) -> Value {
        match action {
            HardwareAction::MoveServo { pan, tilt } => {
                json!({
                    "type": "mcp",
                    "tool": "self.robot.set_head_angles",
                    "arguments": {
                        "yaw": pan,
                        "pitch": tilt,
                        "speed": 150
                    }
                })
            }
            HardwareAction::SetFace { expression } => {
                let (r, g, b) = match expression.to_lowercase().as_str() {
                    "happy" | "talk_happy" => (0, 150, 100),
                    "curious" => (120, 100, 0),
                    "surprised" => (150, 0, 150),
                    "sleep" => (0, 0, 20),
                    "angry" | "error" => (168, 0, 0),
                    _ => (40, 40, 60),
                };
                json!({
                    "type": "mcp",
                    "tool": "self.robot.set_led_color",
                    "arguments": {
                        "red": r,
                        "green": g,
                        "blue": b
                    }
                })
            }
            HardwareAction::PlayAnimation { animation } => {
                let (yaw, pitch) = match animation.to_lowercase().as_str() {
                    "dance" => (25, 15),
                    "nod" => (0, -20),
                    "shake" => (-35, 0),
                    "wave" => (15, 20),
                    "look_around" => (-50, 10),
                    "sleep" => (0, -25),
                    "wake" => (0, 15),
                    _ => (0, 0),
                };
                json!({
                    "type": "mcp",
                    "tool": "self.robot.set_head_angles",
                    "arguments": {
                        "yaw": yaw,
                        "pitch": pitch,
                        "speed": 300
                    }
                })
            }
            HardwareAction::SetLedColor { red, green, blue } => {
                json!({
                    "type": "mcp",
                    "tool": "self.robot.set_led_color",
                    "arguments": {
                        "red": red,
                        "green": green,
                        "blue": blue
                    }
                })
            }
            HardwareAction::CreateReminder { duration_seconds, message, repeat } => {
                json!({
                    "type": "mcp",
                    "tool": "self.robot.create_reminder",
                    "arguments": {
                        "duration_seconds": duration_seconds,
                        "message": message,
                        "repeat": repeat
                    }
                })
            }
            HardwareAction::GetHeadAngles => {
                json!({
                    "type": "mcp",
                    "tool": "self.robot.get_head_angles",
                    "arguments": {}
                })
            }
            HardwareAction::GetReminders => {
                json!({
                    "type": "mcp",
                    "tool": "self.robot.get_reminders",
                    "arguments": {}
                })
            }
            HardwareAction::StopReminder { id } => {
                json!({
                    "type": "mcp",
                    "tool": "self.robot.stop_reminder",
                    "arguments": { "id": id }
                })
            }
            HardwareAction::EmergencyStop => {
                json!({
                    "type": "mcp",
                    "tool": "self.robot.set_head_angles",
                    "arguments": {
                        "yaw": 0,
                        "pitch": 0,
                        "speed": 500
                    }
                })
            }
            HardwareAction::Custom { name, args } => {
                json!({
                    "type": "mcp",
                    "tool": name,
                    "arguments": args
                })
            }
        }
    }

    pub fn parse_command_text(input: &str) -> Option<HardwareAction> {
        let clean = input.trim().to_lowercase();
        if clean.contains("stop") || clean.contains("freeze") || clean.contains("halt") {
            Some(HardwareAction::EmergencyStop)
        } else if clean.contains("nod") {
            Some(HardwareAction::PlayAnimation { animation: "nod".to_string() })
        } else if clean.contains("dance") {
            Some(HardwareAction::PlayAnimation { animation: "dance".to_string() })
        } else if clean.contains("wave") {
            Some(HardwareAction::PlayAnimation { animation: "wave".to_string() })
        } else if clean.contains("look up") {
            Some(HardwareAction::MoveServo { pan: 0, tilt: 25 })
        } else if clean.contains("look down") {
            Some(HardwareAction::MoveServo { pan: 0, tilt: -20 })
        } else if clean.contains("red light") {
            Some(HardwareAction::SetLedColor { red: 168, green: 0, blue: 0 })
        } else if clean.contains("green light") {
            Some(HardwareAction::SetLedColor { red: 0, green: 168, blue: 0 })
        } else if clean.contains("blue light") {
            Some(HardwareAction::SetLedColor { red: 0, green: 0, blue: 168 })
        } else {
            None
        }
    }
}
