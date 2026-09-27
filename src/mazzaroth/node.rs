use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MemoryTier {
    L1Working,   // Temporary turn buffer (< 1 hour)
    L2Episodic,  // Interaction log (days / weeks)
    L3Semantic,  // Concepts, skills, user preferences (months / years)
    L4Core,      // Permanent identity, core rules (immutable / infinite retention)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeTaxonomy {
    Identity,
    Person,
    Concept,
    Project,
    Procedure,
    Lesson,
    Event,
    AtomicFact,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryNode {
    pub id: String,
    pub title: String,
    pub content: String,
    pub tier: MemoryTier,
    pub taxonomy: NodeTaxonomy,
    pub tags: Vec<String>,
    pub links: Vec<String>,
    pub access_count: u32,
    pub last_accessed_secs: i64,
    pub created_at_secs: i64,
    pub strength: f32,    // 0.1 to 1.0 (reinforcement stability)
    pub decay_rate: f32,  // 0.0 (no decay for L4) to 0.5 (fast decay for L1)
    pub x: f32,           // 3D Celestial X
    pub y: f32,           // 3D Celestial Y
    pub z: f32,           // 3D Celestial Z
    pub mass: f32,        // Gravitational mass
}

impl MemoryNode {
    pub fn new(id: String, title: String, content: String, tier: MemoryTier, taxonomy: NodeTaxonomy, now_secs: i64) -> Self {
        let decay_rate = match tier {
            MemoryTier::L1Working => 0.5,
            MemoryTier::L2Episodic => 0.1,
            MemoryTier::L3Semantic => 0.02,
            MemoryTier::L4Core => 0.0,
        };

        let mass = match tier {
            MemoryTier::L4Core => 5.0,
            MemoryTier::L3Semantic => 2.5,
            MemoryTier::L2Episodic => 1.2,
            MemoryTier::L1Working => 0.6,
        };

        Self {
            id,
            title,
            content,
            tier,
            taxonomy,
            tags: Vec::new(),
            links: Vec::new(),
            access_count: 1,
            last_accessed_secs: now_secs,
            created_at_secs: now_secs,
            strength: 0.5,
            decay_rate,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            mass,
        }
    }
}
