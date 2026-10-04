use crate::vision::tracker::GazeCommand;
use crate::vision::Detection;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceCommand {
    pub expression: String, // "happy", "thinking", "neutral", "sleep", "talking"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SayCommand {
    pub text: String,
    pub expression: String,
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

#[derive(Clone)]
pub struct MqttBus {
    client: AsyncClient,
    pub host: String,
    pub port: u16,
}

impl MqttBus {
    pub fn new(host: &str, port: u16, client_id: &str) -> (Self, rumqttc::EventLoop) {
        let mut mqttoptions = MqttOptions::new(client_id, host, port);
        mqttoptions.set_keep_alive(Duration::from_secs(10));
        mqttoptions.set_clean_session(true);

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

    pub fn clone_client(&self) -> AsyncClient {
        self.client.clone()
    }


    pub async fn run_event_loop(client: AsyncClient, mut eventloop: rumqttc::EventLoop) {
        loop {
            match eventloop.poll().await {
                Ok(notification) => match notification {
                    Event::Incoming(Incoming::Publish(publish)) => {
                        let topic = publish.topic;
                        let payload = String::from_utf8_lossy(&publish.payload);
                        tracing::info!("MQTT received [{}] -> {}", topic, payload);
                    }
                    Event::Incoming(Incoming::ConnAck(_)) => {
                        tracing::info!("MQTT broker connected/reconnected. Registering subscriptions...");
                        if let Err(e) = client.subscribe("stackchan/event/#", QoS::AtLeastOnce).await {
                            tracing::warn!("Failed to subscribe to stackchan/event/#: {}", e);
                        }
                        if let Err(e) = client.subscribe("fleet/+/telemetry", QoS::AtLeastOnce).await {
                            tracing::warn!("Failed to subscribe to fleet/+/telemetry: {}", e);
                        }
                    }
                    _ => {}
                },
                Err(e) => {
                    tracing::warn!("MQTT connection error: {}. Reconnecting in 3s...", e);
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            }
        }
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

    pub async fn publish_say(&self, text: &str, expression: &str) -> Result<(), String> {
        let cmd = SayCommand {
            text: text.to_string(),
            expression: expression.to_string(),
        };
        let payload = serde_json::to_string(&cmd).map_err(|e| e.to_string())?;
        self.client
            .publish("stackchan/cmd/say", QoS::AtLeastOnce, false, payload)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn publish_audio(&self, audio_bytes: &[u8]) -> Result<(), String> {
        self.client
            .publish("stackchan/cmd/audio", QoS::AtLeastOnce, false, audio_bytes)
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
}


