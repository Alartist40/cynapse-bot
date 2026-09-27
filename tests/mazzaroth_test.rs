use cynpase_bot::mazzaroth::{
    CognitiveEngine, MazzarothEngine, MemoryNode, MemoryTier, NodeTaxonomy,
};
use tempfile::tempdir;

#[tokio::test]
async fn test_mazzaroth_core_persistence() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("mazzaroth_test.db");

    // 1. Create engine and store core node
    {
        let engine = MazzarothEngine::open(&db_path).unwrap();
        let node = MemoryNode::new(
            "core_identity".to_string(),
            "Robot Core Identity".to_string(),
            "I am CynapseBot running Mazzaroth memory.".to_string(),
            MemoryTier::L4Core,
            NodeTaxonomy::Identity,
            1000,
        );
        engine.upsert_node(node).await.unwrap();
    }

    // 2. Re-open engine and verify persistence + FTS retrieval
    {
        let engine = MazzarothEngine::open(&db_path).unwrap();
        let recalled = engine.recall("CynapseBot", 5).await;
        assert_eq!(recalled.len(), 1);
        assert_eq!(recalled[0].id, "core_identity");
        assert_eq!(recalled[0].tier, MemoryTier::L4Core);
    }
}

#[tokio::test]
async fn test_cognitive_decay_and_hebbian() {
    let mut node = MemoryNode::new(
        "temp_topic".to_string(),
        "Temporary Topic".to_string(),
        "Discussion about today's weather.".to_string(),
        MemoryTier::L1Working,
        NodeTaxonomy::Event,
        1000,
    );

    // Initial retention
    let r0 = CognitiveEngine::compute_retention(&node, 1000);
    assert!((r0 - 1.0).abs() < 0.01);

    // Retention after 10 hours
    let r10 = CognitiveEngine::compute_retention(&node, 1000 + 36000);
    assert!(r10 < r0, "Memory retention must decay over elapsed time");

    // Reinforce access
    let old_strength = node.strength;
    CognitiveEngine::reinforce_access(&mut node, 1000 + 36000);
    assert!(node.strength > old_strength, "Strength must increase upon recall");

    // Hebbian Link
    CognitiveEngine::reinforce_hebbian_link(&mut node, "concept_weather");
    assert!(node.links.contains(&"concept_weather".to_string()));
}

#[tokio::test]
async fn test_hierarchy_consolidation() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("mazzaroth_hierarchy.db");
    let engine = MazzarothEngine::open(&db_path).unwrap();

    // 1. Record episodic turn
    let turn_id = engine
        .record_turn("My favorite programming language is Rust.", "Rust is an excellent language for systems programming.")
        .await
        .unwrap();
    assert!(turn_id.starts_with("turn_"));

    // 2. Consolidate into permanent semantic fact
    let fact_id = engine
        .consolidate_episodic_to_semantic("User Preference: Rust", "User's favorite programming language is Rust.")
        .await
        .unwrap();
    assert!(fact_id.starts_with("fact_"));

    // 3. Recall fact by keyword
    let recalled = engine.recall("Rust", 5).await;
    assert!(!recalled.is_empty());
}

#[tokio::test]
async fn test_celestial_spatial_layout() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("mazzaroth_celestial.db");
    let engine = MazzarothEngine::open(&db_path).unwrap();

    // Add multiple nodes
    let node_a = MemoryNode::new("star_a".to_string(), "Star A".to_string(), "Content A".to_string(), MemoryTier::L3Semantic, NodeTaxonomy::Person, 100);
    let mut node_b = MemoryNode::new("star_b".to_string(), "Star B".to_string(), "Content B".to_string(), MemoryTier::L3Semantic, NodeTaxonomy::Concept, 100);
    node_b.links.push("star_a".to_string());

    engine.upsert_node(node_a).await.unwrap();
    engine.upsert_node(node_b).await.unwrap();

    // Compute constellation map
    let (nodes, edges) = engine.get_constellation_map().await;
    assert_eq!(nodes.len(), 2);
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0], ("star_b".to_string(), "star_a".to_string()));
}
