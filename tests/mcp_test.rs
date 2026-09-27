use cynpase_bot::mcp::{HardwareAction, McpDispatcher};

#[test]
fn test_mcp_action_dispatch() {
    // 1. Test parsing action commands
    let stop_action = McpDispatcher::parse_command_text("emergency stop right now").unwrap();
    assert_eq!(stop_action, HardwareAction::EmergencyStop);

    let wave_action = McpDispatcher::parse_command_text("wave hello").unwrap();
    assert_eq!(wave_action, HardwareAction::PlayAnimation { animation: "wave".to_string() });

    let look_action = McpDispatcher::parse_command_text("look up please").unwrap();
    assert_eq!(look_action, HardwareAction::MoveServo { pan: 0, tilt: 25 });

    // 2. Test MCP JSON serialization format for StackChan hal_mcp.cpp / mcp_server.cc
    let mcp_json = McpDispatcher::format_mcp_call(look_action);
    assert_eq!(mcp_json["type"], "mcp");
    assert_eq!(mcp_json["payload"]["jsonrpc"], "2.0");
    assert_eq!(mcp_json["payload"]["method"], "tools/call");
    assert_eq!(mcp_json["payload"]["params"]["name"], "self.robot.set_head_angles");
    assert_eq!(mcp_json["payload"]["params"]["arguments"]["pitch"], 25);
}
