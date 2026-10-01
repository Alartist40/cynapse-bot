pub mod detector;
pub mod tracker;

pub use detector::{Detection, VisionDetector};
pub use tracker::{GazeCommand, Track, TrackState, VisionTracker, TILT_MIN_DEG, TILT_MAX_DEG};
