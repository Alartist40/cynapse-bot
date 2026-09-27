use cynpase_bot::{AnimationLibrary, VisionManager};

#[tokio::test]
async fn test_vision_and_animations() {
    // 1. Test Vision JPEG payload detector
    let jpeg_sample = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
    assert!(VisionManager::is_jpeg_payload(&jpeg_sample));

    let packet_jpeg = vec![0x02, 0xFF, 0xD8, 0xFF, 0xE0];
    assert!(VisionManager::is_jpeg_payload(&packet_jpeg));

    let audio_sample = vec![0xF8, 0xFF, 0xFE, 0x01];
    assert!(!VisionManager::is_jpeg_payload(&audio_sample));

    // 2. Test Ingest & Retrieve
    let vision = VisionManager::new();
    vision.ingest_frame(packet_jpeg).await;
    let latest = vision.get_latest_frame().await.unwrap();
    assert_eq!(latest, vec![0xFF, 0xD8, 0xFF, 0xE0]);

    // 3. Test Animation Library
    let dance = AnimationLibrary::get("dance").unwrap();
    assert_eq!(dance.name, "dance");
    assert!(!dance.keyframes.is_empty());

    let nod = AnimationLibrary::get("nod").unwrap();
    assert_eq!(nod.name, "nod");
    assert_eq!(nod.keyframes[0].expression, "neutral");
}
