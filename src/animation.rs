use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Keyframe {
    pub pan: i32,           // -90 to +90
    pub tilt: i32,          // -30 to +30
    pub expression: String, // "happy", "neutral", "curious", "surprised", "sleep"
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Animation {
    pub name: String,
    pub keyframes: Vec<Keyframe>,
}

pub struct AnimationLibrary;

impl AnimationLibrary {
    pub fn get(name: &str) -> Option<Animation> {
        match name.to_lowercase().as_str() {
            "dance" => Some(Animation {
                name: "dance".to_string(),
                keyframes: vec![
                    Keyframe { pan: 25, tilt: 15, expression: "happy".to_string(), duration_ms: 250 },
                    Keyframe { pan: -25, tilt: -10, expression: "talk_happy".to_string(), duration_ms: 250 },
                    Keyframe { pan: 20, tilt: 10, expression: "happy".to_string(), duration_ms: 250 },
                    Keyframe { pan: -20, tilt: 15, expression: "talk_happy".to_string(), duration_ms: 250 },
                    Keyframe { pan: 0, tilt: 0, expression: "happy".to_string(), duration_ms: 300 },
                ],
            }),
            "nod" => Some(Animation {
                name: "nod".to_string(),
                keyframes: vec![
                    Keyframe { pan: 0, tilt: -20, expression: "neutral".to_string(), duration_ms: 200 },
                    Keyframe { pan: 0, tilt: 15, expression: "happy".to_string(), duration_ms: 200 },
                    Keyframe { pan: 0, tilt: -10, expression: "happy".to_string(), duration_ms: 150 },
                    Keyframe { pan: 0, tilt: 0, expression: "neutral".to_string(), duration_ms: 200 },
                ],
            }),
            "shake" => Some(Animation {
                name: "shake".to_string(),
                keyframes: vec![
                    Keyframe { pan: -35, tilt: 0, expression: "neutral".to_string(), duration_ms: 180 },
                    Keyframe { pan: 35, tilt: 0, expression: "neutral".to_string(), duration_ms: 180 },
                    Keyframe { pan: -20, tilt: 0, expression: "neutral".to_string(), duration_ms: 150 },
                    Keyframe { pan: 20, tilt: 0, expression: "neutral".to_string(), duration_ms: 150 },
                    Keyframe { pan: 0, tilt: 0, expression: "neutral".to_string(), duration_ms: 200 },
                ],
            }),
            "wave" => Some(Animation {
                name: "wave".to_string(),
                keyframes: vec![
                    Keyframe { pan: 15, tilt: 20, expression: "happy".to_string(), duration_ms: 200 },
                    Keyframe { pan: -15, tilt: 15, expression: "happy".to_string(), duration_ms: 200 },
                    Keyframe { pan: 15, tilt: 20, expression: "happy".to_string(), duration_ms: 200 },
                    Keyframe { pan: 0, tilt: 0, expression: "happy".to_string(), duration_ms: 300 },
                ],
            }),
            "look_around" => Some(Animation {
                name: "look_around".to_string(),
                keyframes: vec![
                    Keyframe { pan: -50, tilt: 10, expression: "curious".to_string(), duration_ms: 500 },
                    Keyframe { pan: 50, tilt: 10, expression: "curious".to_string(), duration_ms: 800 },
                    Keyframe { pan: 0, tilt: 0, expression: "neutral".to_string(), duration_ms: 400 },
                ],
            }),
            "sleep" => Some(Animation {
                name: "sleep".to_string(),
                keyframes: vec![
                    Keyframe { pan: 0, tilt: -25, expression: "sleep".to_string(), duration_ms: 600 },
                ],
            }),
            "wake" => Some(Animation {
                name: "wake".to_string(),
                keyframes: vec![
                    Keyframe { pan: 0, tilt: 15, expression: "surprised".to_string(), duration_ms: 300 },
                    Keyframe { pan: 0, tilt: 0, expression: "happy".to_string(), duration_ms: 400 },
                ],
            }),
            _ => None,
        }
    }
}
