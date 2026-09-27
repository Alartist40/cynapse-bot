use cynpase_bot::persona::{PersonaConfig, PersonaManager};
use tempfile::tempdir;

#[tokio::test]
async fn test_persona_and_memory() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("dendrite_test.db");

    let manager = PersonaManager::new(PersonaConfig {
        name: "TestBot".to_string(),
        identity: "Friendly assistant".to_string(),
        soul: "Loves exploring".to_string(),
        voice: "en_US".to_string(),
        persona_dir: None,
        ..Default::default()
    })
    .with_memory(&db_path)
    .unwrap();

    // 1. Record interaction
    manager.record_interaction("User favorite color", "User likes blue").unwrap();

    // 2. Build system prompt and verify memory injection
    let prompt = manager.build_system_prompt();
    assert!(prompt.contains("TestBot"));
    assert!(prompt.contains("Friendly assistant"));
    assert!(prompt.contains("User favorite color"));
}
