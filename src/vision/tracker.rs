use super::detector::Detection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const TILT_MIN_DEG: f32 = 5.0;
pub const TILT_MAX_DEG: f32 = 85.0;
pub const PAN_MIN_DEG: f32 = 0.0;
pub const PAN_MAX_DEG: f32 = 180.0;

pub const STABLE_FRAME_THRESHOLD: usize = 5;
pub const LOST_FRAME_THRESHOLD: usize = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackState {
    Candidate,
    Stable,
    Lost,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: u32,
    pub class: String,
    pub conf: f32,
    pub cx: f32,
    pub cy: f32,
    pub w: f32,
    pub h: f32,
    pub consecutive_frames: usize,
    pub missing_frames: usize,
    pub state: TrackState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GazeCommand {
    pub pan: f32,        // -1.0 to 1.0 (normalized offset from center)
    pub tilt: f32,       // 0.0 to 1.0 (normalized height)
    pub pan_angle: f32,  // 0.0 to 180.0 degrees
    pub tilt_angle: f32, // 5.0 to 85.0 degrees (hard hardware clamp!)
    pub speed: f32,
}

pub struct VisionTracker {
    next_id: u32,
    tracks: HashMap<u32, Track>,
    current_pan: f32,
    current_tilt: f32,
    pub mirror_pan: bool,
}

impl Default for VisionTracker {
    fn default() -> Self {
        Self::new(true)
    }
}

impl VisionTracker {
    pub fn new(mirror_pan: bool) -> Self {
        Self {
            next_id: 1,
            tracks: HashMap::new(),
            current_pan: 90.0,
            current_tilt: 45.0, // center between 5 and 85
            mirror_pan,
        }
    }

    /// Non-mutating read-only view of active tracks (does NOT increment missing frames).
    pub fn peek_tracks(&self) -> Vec<Track> {
        self.tracks
            .values()
            .filter(|t| t.state != TrackState::Lost)
            .cloned()
            .collect()
    }

    /// Update tracker with detections from the current frame.
    pub fn update(&mut self, detections: &[Detection]) -> (Vec<Track>, Option<GazeCommand>) {
        let mut matched_track_ids = Vec::new();

        for det in detections {
            let mut best_id = None;
            let mut min_dist = 0.35; // association threshold

            for (id, track) in self.tracks.iter() {
                if track.class == det.class && track.state != TrackState::Lost {
                    let dist = ((track.cx - det.cx).powi(2) + (track.cy - det.cy).powi(2)).sqrt();
                    if dist < min_dist {
                        min_dist = dist;
                        best_id = Some(*id);
                    }
                }
            }

            if let Some(id) = best_id {
                if let Some(track) = self.tracks.get_mut(&id) {
                    track.cx = track.cx * 0.7 + det.cx * 0.3; // exponential smoothing
                    track.cy = track.cy * 0.7 + det.cy * 0.3;
                    track.w = det.w;
                    track.h = det.h;
                    track.conf = det.conf;
                    track.consecutive_frames += 1;
                    track.missing_frames = 0;

                    if track.consecutive_frames >= STABLE_FRAME_THRESHOLD {
                        track.state = TrackState::Stable;
                    }
                    matched_track_ids.push(id);
                }
            } else {
                // New candidate track
                let id = self.next_id;
                self.next_id += 1;
                self.tracks.insert(
                    id,
                    Track {
                        id,
                        class: det.class.clone(),
                        conf: det.conf,
                        cx: det.cx,
                        cy: det.cy,
                        w: det.w,
                        h: det.h,
                        consecutive_frames: 1,
                        missing_frames: 0,
                        state: TrackState::Candidate,
                    },
                );
                matched_track_ids.push(id);
            }
        }

        // Increment missing frames for unmatched tracks
        let all_ids: Vec<u32> = self.tracks.keys().copied().collect();
        for id in all_ids {
            if !matched_track_ids.contains(&id) {
                if let Some(track) = self.tracks.get_mut(&id) {
                    track.missing_frames += 1;
                    if track.missing_frames >= LOST_FRAME_THRESHOLD {
                        track.state = TrackState::Lost;
                    }
                }
            }
        }

        // Cleanup lost tracks
        self.tracks.retain(|_, t| t.state != TrackState::Lost);

        // Compute gaze target for deterministic primary target
        let gaze = self.compute_gaze_for_primary_target();

        (self.peek_tracks(), gaze)
    }

    /// Select primary target deterministically by highest confidence and area
    fn compute_gaze_for_primary_target(&mut self) -> Option<GazeCommand> {
        let mut stable_people: Vec<&Track> = self
            .tracks
            .values()
            .filter(|t| t.class == "person" && t.state == TrackState::Stable)
            .collect();

        if stable_people.is_empty() {
            return None;
        }

        // Sort deterministically: highest confidence first, then largest bounding box area
        stable_people.sort_by(|a, b| {
            b.conf
                .partial_cmp(&a.conf)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    let area_a = a.w * a.h;
                    let area_b = b.w * b.h;
                    area_b.partial_cmp(&area_a).unwrap_or(std::cmp::Ordering::Equal)
                })
        });

        let target = stable_people[0];

        // Map pan coordinate with configurable mirroring
        let target_pan = if self.mirror_pan {
            (1.0 - target.cx) * 180.0
        } else {
            target.cx * 180.0
        };

        // Map tilt coordinate to [5.0, 85.0]
        let target_tilt = target.cy * (TILT_MAX_DEG - TILT_MIN_DEG) + TILT_MIN_DEG;

        // Apply strict hardware clamps
        let clamped_pan = target_pan.clamp(PAN_MIN_DEG, PAN_MAX_DEG);
        let clamped_tilt = target_tilt.clamp(TILT_MIN_DEG, TILT_MAX_DEG);

        // Exponential smoothing
        self.current_pan = self.current_pan * 0.8 + clamped_pan * 0.2;
        self.current_tilt = self.current_tilt * 0.8 + clamped_tilt * 0.2;

        let norm_pan = (self.current_pan - 90.0) / 90.0; // -1.0 to 1.0
        let norm_tilt = (self.current_tilt - TILT_MIN_DEG) / (TILT_MAX_DEG - TILT_MIN_DEG); // 0.0 to 1.0

        Some(GazeCommand {
            pan: norm_pan,
            tilt: norm_tilt,
            pan_angle: self.current_pan,
            tilt_angle: self.current_tilt.clamp(TILT_MIN_DEG, TILT_MAX_DEG),
            speed: 0.5,
        })
    }
}
