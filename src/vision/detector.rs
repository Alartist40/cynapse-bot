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
}

impl VisionDetector {
    pub fn new(model_name: &str, confidence_threshold: f32) -> Self {
        Self {
            model_name: model_name.to_string(),
            confidence_threshold,
        }
    }

    /// Process a frame buffer (JPEG / raw bytes) and extract bounding box detections.
    pub fn detect_objects(&self, _frame_bytes: &[u8]) -> Vec<Detection> {
        // Fallback / simulated detection for testbench / development
        // (Production RKNN NPU uses librknnrt FFI on RK3588)
        vec![Detection {
            class: "person".to_string(),
            conf: 0.88,
            cx: 0.5,
            cy: 0.45,
            w: 0.35,
            h: 0.60,
        }]
    }
}
