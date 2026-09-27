use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Clone, Default)]
pub struct VisionManager {
    latest_frame: Arc<RwLock<Option<Vec<u8>>>>,
    frame_count: Arc<RwLock<u64>>,
}

impl VisionManager {
    pub fn new() -> Self {
        Self {
            latest_frame: Arc::new(RwLock::new(None)),
            frame_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Check if a binary buffer is a JPEG image payload
    pub fn is_jpeg_payload(data: &[u8]) -> bool {
        if data.len() < 4 {
            return false;
        }
        // Direct JPEG magic bytes (0xFF, 0xD8) or OmniBot/StackChan binary packet header 0x02
        (data[0] == 0xFF && data[1] == 0xD8) || (data[0] == 0x02 && data.len() > 3 && data[1] == 0xFF && data[2] == 0xD8)
    }

    /// Store incoming camera frame
    pub async fn ingest_frame(&self, mut data: Vec<u8>) {
        if data.starts_with(&[0x02]) {
            data.remove(0); // strip protocol type byte
        }

        let mut lock = self.latest_frame.write().await;
        let mut count = self.frame_count.write().await;
        *count += 1;
        *lock = Some(data);

        if *count % 30 == 0 {
            info!(frames = *count, "Ingested 30 camera frames");
        }
    }

    /// Retrieve the most recent JPEG frame
    pub async fn get_latest_frame(&self) -> Option<Vec<u8>> {
        let lock = self.latest_frame.read().await;
        lock.clone()
    }
}
