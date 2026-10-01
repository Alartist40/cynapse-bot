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

#[test]
fn test_multi_object_tracking_and_streak_decay() {
    let mut tracker = VisionTracker::new(true);

    let two_people = vec![
        Detection {
            class: "person".to_string(),
            conf: 0.90,
            cx: 0.2,
            cy: 0.5,
            w: 0.2,
            h: 0.5,
        },
        Detection {
            class: "person".to_string(),
            conf: 0.85,
            cx: 0.8,
            cy: 0.5,
            w: 0.2,
            h: 0.5,
        },
    ];

    // Frame 1: Both should create distinct tracks
    let (tracks, _) = tracker.update(&two_people);
    assert_eq!(tracks.len(), 2);
    assert_ne!(tracks[0].id, tracks[1].id);

    // After 5 frames, both become Stable
    for _ in 2..=5 {
        tracker.update(&two_people);
    }
    let (tracks, _) = tracker.update(&two_people);
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0].state, TrackState::Stable);
    assert_eq!(tracks[1].state, TrackState::Stable);

    // Frame gap: if no detections are sent, streak decays
    tracker.update(&[]);
    let peek = tracker.peek_tracks();
    assert_eq!(peek.len(), 2);
    assert_eq!(peek[0].consecutive_frames, 0);
    assert_eq!(peek[1].consecutive_frames, 0);
}

#[test]
fn test_multi_object_association_within_threshold_guard() {
    let mut tracker = VisionTracker::new(true);

    // Frame 1: Single detection creates Track #1 at cx = 0.50
    let det_init = vec![Detection {
        class: "person".to_string(),
        conf: 0.90,
        cx: 0.50,
        cy: 0.50,
        w: 0.2,
        h: 0.5,
    }];
    let (tracks, _) = tracker.update(&det_init);
    assert_eq!(tracks.len(), 1);
    let track_id_1 = tracks[0].id;

    // Frame 2: Two detections of same class both within association distance (0.35) of Track #1
    // Detection A is at cx = 0.52 (dist = 0.02)
    // Detection B is at cx = 0.58 (dist = 0.08)
    let det_two_close = vec![
        Detection {
            class: "person".to_string(),
            conf: 0.92,
            cx: 0.52,
            cy: 0.50,
            w: 0.2,
            h: 0.5,
        },
        Detection {
            class: "person".to_string(),
            conf: 0.88,
            cx: 0.58,
            cy: 0.50,
            w: 0.2,
            h: 0.5,
        },
    ];

    let (tracks, _) = tracker.update(&det_two_close);
    // The !matched_track_ids.contains guard MUST prevent Detection B from hijacking Track #1,
    // creating a distinct second track instead.
    assert_eq!(tracks.len(), 2, "Must contain exactly 2 distinct tracks");
    let track_ids: Vec<u32> = tracks.iter().map(|t| t.id).collect();
    assert!(track_ids.contains(&track_id_1), "Track #1 must be updated");
    assert_eq!(tracks.iter().filter(|t| t.id == track_id_1).count(), 1);
}


