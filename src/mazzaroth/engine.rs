use crate::mazzaroth::celestial::CelestialLayout;
use crate::mazzaroth::decay::CognitiveEngine;
use crate::mazzaroth::node::{MemoryNode, MemoryTier, NodeTaxonomy};
use crate::mazzaroth::store::MazzarothStore;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::info;

#[derive(Clone)]
pub struct MazzarothEngine {
    store: Arc<MazzarothStore>,
    nodes: Arc<RwLock<HashMap<String, MemoryNode>>>,
}

impl MazzarothEngine {
    pub fn open<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let store = MazzarothStore::open(db_path)?;
        let loaded = store.load_all()?;

        let mut map = HashMap::new();
        for node in loaded {
            map.insert(node.id.clone(), node);
        }

        Ok(Self {
            store: Arc::new(store),
            nodes: Arc::new(RwLock::new(map)),
        })
    }

    pub fn store(&self) -> &MazzarothStore {
        &self.store
    }

    pub fn get_all_nodes_sync(&self) -> Result<Vec<MemoryNode>> {
        self.store.load_all()
    }

    fn now_secs() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }

    /// Insert or update a memory node across any tier
    pub async fn upsert_node(&self, mut node: MemoryNode) -> Result<()> {
        let now = Self::now_secs();
        CognitiveEngine::reinforce_access(&mut node, now);

        self.store.save_node(&node)?;

        let mut lock = self.nodes.write().await;
        lock.insert(node.id.clone(), node);
        Ok(())
    }

    /// Record a conversation turn into episodic and working tiers
    pub async fn record_turn(&self, user_text: &str, assistant_reply: &str) -> Result<String> {
        let now = Self::now_secs();
        let id = format!("turn_{}", now);
        let title = format!("Turn at {}", now);
        let content = format!("User: {}\nAssistant: {}", user_text, assistant_reply);

        let mut node = MemoryNode::new(
            id.clone(),
            title,
            content,
            MemoryTier::L2Episodic,
            NodeTaxonomy::Event,
            now,
        );
        node.tags = vec!["conversation".to_string(), "turn".to_string()];

        // Hebbian association check against existing concepts
        let mut related_ids = Vec::new();
        {
            let lock = self.nodes.read().await;
            for existing in lock.values() {
                if existing.tier >= MemoryTier::L3Semantic
                    && (user_text.to_lowercase().contains(&existing.title.to_lowercase())
                        || assistant_reply.to_lowercase().contains(&existing.title.to_lowercase()))
                {
                    related_ids.push(existing.id.clone());
                }
            }
        }

        for rel_id in related_ids {
            CognitiveEngine::reinforce_hebbian_link(&mut node, &rel_id);
        }

        self.upsert_node(node).await?;
        info!(turn_id = %id, "Recorded turn in Mazzaroth memory engine");
        Ok(id)
    }

    /// Hybrid Recall: Full-text match + Cognitive decay retention + Graph degree
    pub async fn recall(&self, query: &str, top_k: usize) -> Vec<MemoryNode> {
        let now = Self::now_secs();
        let matched_ids = self.store.search_fts_ids(query, top_k * 2).unwrap_or_default();

        let mut scored_nodes = Vec::new();
        let lock = self.nodes.read().await;

        for id in matched_ids {
            if let Some(node) = lock.get(&id) {
                let retention = CognitiveEngine::compute_retention(node, now);
                let degree_bonus = 1.0 + (node.links.len() as f32 * 0.1);
                let score = retention * degree_bonus;
                scored_nodes.push((node.clone(), score));
            }
        }

        scored_nodes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored_nodes.into_iter().take(top_k).map(|(node, _)| node).collect()
    }

    /// Consolidate episodic interactions into permanent semantic atomic facts
    pub async fn consolidate_episodic_to_semantic(&self, fact_title: &str, fact_content: &str) -> Result<String> {
        let now = Self::now_secs();
        let id = format!("fact_{}", now);

        let mut node = MemoryNode::new(
            id.clone(),
            fact_title.to_string(),
            fact_content.to_string(),
            MemoryTier::L3Semantic,
            NodeTaxonomy::AtomicFact,
            now,
        );
        node.tags = vec!["semantic_fact".to_string(), "consolidated".to_string()];

        self.upsert_node(node).await?;
        info!(fact_id = %id, title = %fact_title, "Consolidated episodic memory into Mazzaroth semantic tier");
        Ok(id)
    }

    /// Run celestial 3D layout step and generate visual constellation graph
    pub async fn get_constellation_map(&self) -> (Vec<MemoryNode>, Vec<(String, String)>) {
        let mut nodes: Vec<MemoryNode> = {
            let lock = self.nodes.read().await;
            lock.values().cloned().collect()
        };

        CelestialLayout::update_3d_coordinates(&mut nodes);
        let edges = CelestialLayout::build_constellation_edges(&nodes);

        (nodes, edges)
    }
}
