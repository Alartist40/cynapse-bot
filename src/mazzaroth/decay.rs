use crate::mazzaroth::node::{MemoryNode, MemoryTier};

pub struct CognitiveEngine;

impl CognitiveEngine {
    /// Calculate current retention activation score R(t) in range [0.0, 1.0]
    pub fn compute_retention(node: &MemoryNode, current_time_secs: i64) -> f32 {
        if node.tier == MemoryTier::L4Core || node.decay_rate <= 0.0 {
            return 1.0;
        }

        let elapsed_hours = (current_time_secs - node.last_accessed_secs).max(0) as f32 / 3600.0;
        let stability = (node.strength * (1.0 + (node.access_count as f32 * 0.1))).max(0.1);
        
        // Ebbinghaus exponential decay: R = exp(- (decay_rate * dt) / stability)
        (- (node.decay_rate * elapsed_hours) / stability).exp().clamp(0.01, 1.0)
    }

    /// Reinforce node memory strength upon recall or access
    pub fn reinforce_access(node: &mut MemoryNode, current_time_secs: i64) {
        node.access_count += 1;
        node.last_accessed_secs = current_time_secs;
        // Increase strength asymptotically towards 1.0
        node.strength = (node.strength + 0.15 * (1.0 - node.strength)).clamp(0.1, 1.0);
    }

    /// Reinforce Hebbian association between two co-active memory nodes
    pub fn reinforce_hebbian_link(node_a: &mut MemoryNode, node_b_id: &str) {
        if !node_a.links.contains(&node_b_id.to_string()) {
            node_a.links.push(node_b_id.to_string());
        }
        // Strengthen connection
        node_a.strength = (node_a.strength + 0.05).min(1.0);
    }
}
