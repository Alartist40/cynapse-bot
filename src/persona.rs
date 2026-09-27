use crate::mazzaroth::node::{MemoryNode, MemoryTier, NodeTaxonomy};
use crate::mazzaroth::MazzarothEngine;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
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
            tools: "Available tools: self.robot.set_head_angles, self.robot.set_led_color, play_animation, self.robot.create_reminder, emergency_stop.".to_string(),
            voice: "alba".to_string(),
            persona_dir: Some(PathBuf::from("data/persona")),
        }
    }
}

pub struct PersonaManager {
    pub config: PersonaConfig,
    pub memory_engine: Option<MazzarothEngine>,
}

impl PersonaManager {
    pub fn new(config: PersonaConfig) -> Self {
        let mut mgr = Self {
            config,
            memory_engine: None,
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
        let engine = MazzarothEngine::open(db_path)?;
        self.memory_engine = Some(engine);
        Ok(self)
    }

    pub fn build_system_prompt(&self) -> String {
        let mut prompt = format!(
            "Name: {}\nIdentity:\n{}\n\nSoul:\n{}\n\nTools:\n{}\n",
            self.config.name, self.config.identity, self.config.soul, self.config.tools
        );

        if let Some(ref engine) = self.memory_engine {
            if let Ok(nodes) = engine.get_all_nodes_sync() {
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
        if let Some(ref engine) = self.memory_engine {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            let node = MemoryNode::new(
                format!("turn_{}", now),
                title.to_string(),
                content.to_string(),
                MemoryTier::L2Episodic,
                NodeTaxonomy::Event,
                now,
            );
            engine.store().save_node(&node)?;
            info!(title = %title, "Recorded turn in Mazzaroth memory engine");
        }
        Ok(())
    }
}
