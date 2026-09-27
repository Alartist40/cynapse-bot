use cynpase_bot::gui::{GuiState, ViewportProjection};

#[test]
fn test_gui_state_manager() {
    let mut state = GuiState::default();

    // 1. Pan/Tilt clamped range
    state.set_angles(120, -50);
    assert_eq!(state.pan, 90, "Pan must be clamped to +90");
    assert_eq!(state.tilt, -30, "Tilt must be clamped to -30");

    // 2. Transcript addition
    state.add_transcript("TEST", "Hello robot UI");
    assert_eq!(state.transcripts.len(), 1);
    assert!(state.transcripts[0].1.contains("Hello robot UI"));
}

#[test]
fn test_constellation_viewport_projection() {
    let screen_center = (400.0, 300.0);
    
    // Test center projection
    let (sx, sy, _) = ViewportProjection::project_3d_to_2d(0.0, 0.0, 0.0, 0.0, 0.0, 1.0, screen_center);
    assert_eq!(sx, 400.0);
    assert_eq!(sy, 300.0);

    // Test offset node projection
    let (sx2, sy2, _) = ViewportProjection::project_3d_to_2d(100.0, 50.0, 0.0, 0.0, 0.0, 1.0, screen_center);
    assert!(sx2 > 400.0);
    assert!(sy2 < 300.0); // Y inverted for screen coordinate system
}
