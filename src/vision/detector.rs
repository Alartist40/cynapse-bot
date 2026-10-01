use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Detection {
    pub class: String,
    pub conf: f32,
    pub cx: f32, // normalized 0.0 - 1.0 (center x)
    pub cy: f32, // normalized 0.0 - 1.0 (center y)
    pub w: f32,  // normalized 0.0 - 1.0 (width)
    pub h: f32,  // normalized 0.0 - 1.0 (height)
}

pub struct VisionDetector {
    pub model_name: String,
    pub confidence_threshold: f32,
    pub simulated: bool,
}

impl VisionDetector {
    pub fn new(model_name: &str, confidence_threshold: f32, simulated: bool) -> Self {
        Self {
            model_name: model_name.to_string(),
            confidence_threshold,
            simulated,
        }
    }

    pub fn status_str(&self) -> &'static str {
        if self.simulated {
            "simulated"
        } else {
            "idle_waiting_npu"
        }
    }


    /// Process a frame buffer (JPEG / raw bytes) and extract bounding box detections.
    pub fn detect_objects(&self, frame_bytes: &[u8]) -> Vec<Detection> {
        if !self.simulated {
            // In real mode without hardware RKNN NPU model loaded, returns empty
            if frame_bytes.is_empty() {
                return Vec::new();
            }
            return Vec::new();
        }

        // Controlled simulated detection when simulated is explicitly true
        if frame_bytes.is_empty() {
            Vec::new()
        } else {
            let candidate = Detection {
                class: "person".to_string(),
                conf: 0.88,
                cx: 0.5,
                cy: 0.45,
                w: 0.35,
                h: 0.60,
            };
            if candidate.conf >= self.confidence_threshold {
                vec![candidate]
            } else {
                Vec::new()
            }
        }
    }
}

