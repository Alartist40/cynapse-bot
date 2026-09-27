use crate::mazzaroth::node::MemoryNode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiState {
    pub hub_url: String,
    pub ws_url: String,
    pub connected: bool,
    pub pan: i32,
    pub tilt: i32,
    pub expression: String,
    pub neon_red: u8,
    pub neon_green: u8,
    pub neon_blue: u8,
    pub transcripts: Vec<(String, String)>,
    pub camera_frame_count: u64,
    pub constellation_nodes: Vec<MemoryNode>,
    pub constellation_edges: Vec<(String, String)>,
    pub rot_x: f32,
    pub rot_y: f32,
    pub zoom: f32,
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            hub_url: "http://127.0.0.1:8000".to_string(),
            ws_url: "ws://127.0.0.1:8000/ws/monitor".to_string(),
            connected: false,
            pan: 0,
            tilt: 0,
            expression: "neutral".to_string(),
            neon_red: 0,
            neon_green: 168,
            neon_blue: 0,
            transcripts: Vec::new(),
            camera_frame_count: 0,
            constellation_nodes: Vec::new(),
            constellation_edges: Vec::new(),
            rot_x: 0.2,
            rot_y: 0.3,
            zoom: 1.0,
        }
    }
}

impl GuiState {
    pub fn add_transcript(&mut self, sender: &str, message: &str) {
        let ts = format!(
            "{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() % 86400
        );
        self.transcripts.push((ts, format!("{}: {}", sender, message)));
        if self.transcripts.len() > 100 {
            self.transcripts.remove(0);
        }
    }

    pub fn set_angles(&mut self, pan: i32, tilt: i32) {
        self.pan = pan.clamp(-90, 90);
        self.tilt = tilt.clamp(-30, 30);
    }
}
