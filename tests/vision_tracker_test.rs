use cynapse_bot::vision::{
    Detection, TrackState, VisionTracker, TILT_MAX_DEG, TILT_MIN_DEG,
};

#[test]
fn test_tracker_state_progression_and_ghost_suppression() {
    let mut tracker = VisionTracker::new(true);

    let det = vec![Detection {
        class: "person".to_string(),
        conf: 0.90,
        cx: 0.5,
        cy: 0.5,
        w: 0.3,
        h: 0.6,
    }];

    // Frame 1: Candidate state
    let (tracks, gaze) = tracker.update(&det);
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].state, TrackState::Candidate);
    assert!(gaze.is_none(), "Single-frame ghost should not produce gaze command");

    // Frames 2..4: Still Candidate
    for _ in 2..=4 {
        let (tracks, gaze) = tracker.update(&det);
        assert_eq!(tracks[0].state, TrackState::Candidate);
        assert!(gaze.is_none());
    }

    // Frame 5: Reaches STABLE threshold!
    let (tracks, gaze) = tracker.update(&det);
    assert_eq!(tracks[0].state, TrackState::Stable);
    assert!(gaze.is_some(), "Stable target must produce a gaze command");

    let gaze_cmd = gaze.unwrap();
    assert!(gaze_cmd.tilt_angle >= TILT_MIN_DEG && gaze_cmd.tilt_angle <= TILT_MAX_DEG);
}

#[test]
fn test_hard_servo_limit_clamping() {
    let mut tracker = VisionTracker::new(true);

    // Extreme top detection (cy = 0.0) -> target tilt = 5.0 deg
    let extreme_top = vec![Detection {
        class: "person".to_string(),
        conf: 0.95,
        cx: 0.5,
        cy: 0.0,
        w: 0.2,
        h: 0.2,
    }];

    for _ in 0..10 {
        tracker.update(&extreme_top);
    }

    let (_, gaze) = tracker.update(&extreme_top);
    assert!(gaze.is_some());
    let cmd = gaze.unwrap();
    assert!(
        cmd.tilt_angle >= TILT_MIN_DEG,
        "Tilt angle {} is below minimum {}",
        cmd.tilt_angle,
        TILT_MIN_DEG
    );

    // Extreme bottom detection (cy = 1.0) -> target tilt = 85.0 deg
    let extreme_bottom = vec![Detection {
        class: "person".to_string(),
        conf: 0.95,
        cx: 0.5,
        cy: 1.0,
        w: 0.2,
        h: 0.2,
    }];

    for _ in 0..15 {
        tracker.update(&extreme_bottom);
    }

    let (_, gaze) = tracker.update(&extreme_bottom);
    let cmd = gaze.unwrap();
    assert!(
        cmd.tilt_angle <= TILT_MAX_DEG,
        "Tilt angle {} is above maximum {}",
        cmd.tilt_angle,
        TILT_MAX_DEG
    );

    println!("VISION_TRACKER_PASS");
}
