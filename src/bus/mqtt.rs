use crate::vision::tracker::GazeCommand;
use crate::vision::Detection;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceCommand {
    pub expression: String, // "happy", "thinking", "neutral", "sleep", "talking"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TouchEvent {
    pub zone: u8, // 1 to 3
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetTelemetry {
    pub device_id: String,
    pub batt: f32,
    pub rssi: i32,
    pub trigger: String,
    pub uptime_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubStatus {
    pub uptime_secs: u64,
    pub services: serde_json::Value,
}

pub struct MqttBus {
    client: AsyncClient,
    pub host: String,
    pub port: u16,
}

impl MqttBus {
    pub fn new(host: &str, port: u16, client_id: &str) -> (Self, rumqttc::EventLoop) {
        let mut mqttoptions = MqttOptions::new(client_id, host, port);
        mqttoptions.set_keep_alive(Duration::from_secs(10));

        let (client, eventloop) = AsyncClient::new(mqttoptions, 50);
        (
            Self {
                client,
                host: host.to_string(),
                port,
            },
            eventloop,
        )
    }

    pub async fn publish_gaze(&self, gaze: &GazeCommand) -> Result<(), String> {
        let payload = serde_json::to_string(gaze).map_err(|e| e.to_string())?;
        self.client
            .publish("stackchan/cmd/gaze", QoS::AtLeastOnce, false, payload)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn publish_face(&self, expression: &str) -> Result<(), String> {
        let cmd = FaceCommand {
            expression: expression.to_string(),
        };
        let payload = serde_json::to_string(&cmd).map_err(|e| e.to_string())?;
        self.client
            .publish("stackchan/cmd/face", QoS::AtLeastOnce, false, payload)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn publish_detections(&self, src: &str, detections: &[Detection]) -> Result<(), String> {
        let topic = format!("vision/detections/{}", src);
        let payload = serde_json::to_string(detections).map_err(|e| e.to_string())?;
        self.client
            .publish(topic, QoS::AtMostOnce, false, payload)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn publish_status(&self, status: &HubStatus) -> Result<(), String> {
        let payload = serde_json::to_string(status).map_err(|e| e.to_string())?;
        self.client
            .publish("hub/status", QoS::AtLeastOnce, true, payload)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn subscribe_topics(&self) -> Result<(), String> {
        self.client
            .subscribe("stackchan/event/#", QoS::AtLeastOnce)
            .await
            .map_err(|e| e.to_string())?;
        self.client
            .subscribe("fleet/+/telemetry", QoS::AtLeastOnce)
            .await
            .map_err(|e| e.to_string())
    }
}
