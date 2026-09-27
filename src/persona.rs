use cynapse_memory::store::DendriteStore;
use cynapse_memory::graph::{Node, NodeType};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaConfig {
    pub name: String,
    pub identity: String,
    pub soul: String,
    pub tools: String,
    pub voice: String,
    pub persona_dir: Option<PathBuf>,
}

impl Default for PersonaConfig {
    fn default() -> Self {
        Self {
            name: "CynapseBot".to_string(),
            identity: "You are CynapseBot, an embodied desktop companion robot running locally.".to_string(),
            soul: "Friendly, helpful, witty, and concise. You love moving your servos and reacting to the user.".to_string(),
            tools: "Available tools: servo_control, face_display, play_animation, emergency_stop.".to_string(),
            voice: "alba".to_string(),
            persona_dir: Some(PathBuf::from("data/persona")),
        }
    }
}

pub struct PersonaManager {
    pub config: PersonaConfig,
    pub memory_store: Option<Arc<DendriteStore>>,
}

impl PersonaManager {
    pub fn new(config: PersonaConfig) -> Self {
        let mut mgr = Self {
            config,
            memory_store: None,
        };
        mgr.load_persona_files();
        mgr
    }

    pub fn load_persona_files(&mut self) {
        if let Some(ref dir) = self.config.persona_dir {
            if dir.exists() {
                if let Ok(soul) = fs::read_to_string(dir.join("SOUL.md")) {
                    self.config.soul = soul.trim().to_string();
                }
                if let Ok(identity) = fs::read_to_string(dir.join("IDENTITY.md")) {
                    self.config.identity = identity.trim().to_string();
                }
                if let Ok(tools) = fs::read_to_string(dir.join("TOOLS.md")) {
                    self.config.tools = tools.trim().to_string();
                }
                info!("Loaded persona markdown files from {:?}", dir);
            }
        }
    }

    pub fn with_memory(mut self, db_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let store = DendriteStore::open(db_path)?;
        self.memory_store = Some(Arc::new(store));
        Ok(self)
    }

    pub fn build_system_prompt(&self) -> String {
        let mut prompt = format!(
            "Name: {}\nIdentity:\n{}\n\nSoul:\n{}\n\nTools:\n{}\n",
            self.config.name, self.config.identity, self.config.soul, self.config.tools
        );

        if let Some(ref store) = self.memory_store {
            let graph = cynapse_memory::graph::Dendrite::default();
            if store.load_all(&graph).is_ok() {
                let nodes = graph.all();
                if !nodes.is_empty() {
                    prompt.push_str("\nRecent Memory Facts:\n");
                    for node in nodes.iter().take(5) {
                        prompt.push_str(&format!("- {}: {}\n", node.title, node.content));
                    }
                }
            }
        }

        prompt
    }

    pub fn record_interaction(&self, title: &str, content: &str) -> anyhow::Result<()> {
        if let Some(ref store) = self.memory_store {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            let mut node = Node::placeholder(format!("turn_{}", now), now);
            node.title = title.to_string();
            node.content = content.to_string();
            node.node_type = NodeType::TurnLog;
            node.tags = vec!["robot_turn".to_string()];
            store.save(&node)?;
            info!(title = %title, "Recorded turn in Dendrite memory graph");
        }
        Ok(())
    }
}
