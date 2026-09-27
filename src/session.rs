use std::time::Instant;
use uuid::Uuid;

#[derive(Debug)]
pub struct Session {
    pub id: String,
    pub device_id: String,
    pub client_id: String,
    pub is_listening: bool,
    pub audio_buffer: Vec<Vec<u8>>,
    pub created_at: Instant,
    pub last_activity: Instant,
}

impl Session {
    pub fn new(device_id: String, client_id: String) -> Self {
        let now = Instant::now();
        Self {
            id: Uuid::new_v4().to_string(),
            device_id,
            client_id,
            is_listening: false,
            audio_buffer: Vec::new(),
            created_at: now,
            last_activity: now,
        }
    }

    pub fn start_listening(&mut self) {
        self.is_listening = true;
        self.audio_buffer.clear();
        self.last_activity = Instant::now();
    }

    pub fn stop_listening(&mut self) -> Vec<Vec<u8>> {
        self.is_listening = false;
        self.last_activity = Instant::now();
        std::mem::take(&mut self.audio_buffer)
    }

    pub fn push_audio_frame(&mut self, frame: Vec<u8>) {
        if self.is_listening {
            self.audio_buffer.push(frame);
            self.last_activity = Instant::now();
        }
    }
}
