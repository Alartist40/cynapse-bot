use cynapse_bot::bus::{FaceCommand, FleetTelemetry, HubStatus, MqttBus};
use cynapse_bot::vision::tracker::GazeCommand;

#[test]
fn test_mqtt_payload_structures() {
    let gaze = GazeCommand {
        pan: 0.1,
        tilt: 0.5,
        pan_angle: 99.0,
        tilt_angle: 45.0,
        speed: 0.5,
    };
    let gaze_json = serde_json::to_string(&gaze).unwrap();
    assert!(gaze_json.contains("\"tilt_angle\":45.0"));

    let face = FaceCommand {
        expression: "happy".to_string(),
    };
    let face_json = serde_json::to_string(&face).unwrap();
    assert!(face_json.contains("\"expression\":\"happy\""));

    let telemetry = FleetTelemetry {
        device_id: "esp32-sensor-01".to_string(),
        batt: 4.15,
        rssi: -62,
        trigger: "fomo_person".to_string(),
        uptime_sec: 1420,
    };
    let telem_json = serde_json::to_string(&telemetry).unwrap();
    assert!(telem_json.contains("esp32-sensor-01"));

    let status = HubStatus {
        uptime_secs: 3600,
        services: serde_json::json!({"npu": "online", "llm": "ready"}),
    };
    let status_json = serde_json::to_string(&status).unwrap();
    assert!(status_json.contains("\"uptime_secs\":3600"));

    println!("BUS_TEST_PASS");
}

#[tokio::test]
async fn test_mqtt_bus_initialization() {
    let (bus, _) = MqttBus::new("127.0.0.1", 1883, "test-client");
    assert_eq!(bus.port, 1883);
}
