use crate::mazzaroth::node::{MemoryNode, NodeTaxonomy};
use std::collections::HashMap;

pub struct CelestialLayout;

impl CelestialLayout {
    /// Return the anchor galactic center for a given knowledge taxonomy
    pub fn taxonomy_anchor(taxonomy: &NodeTaxonomy) -> (f32, f32, f32) {
        match taxonomy {
            NodeTaxonomy::Identity => (0.0, 0.0, 0.0),            // Galactic Center
            NodeTaxonomy::Person => (100.0, 50.0, 0.0),           // Constellation Alpha
            NodeTaxonomy::Concept => (-100.0, 80.0, 50.0),        // Constellation Beta
            NodeTaxonomy::Project => (80.0, -100.0, -30.0),       // Constellation Gamma
            NodeTaxonomy::Procedure => (-80.0, -90.0, 40.0),      // Constellation Delta
            NodeTaxonomy::Lesson => (0.0, 120.0, -60.0),          // Constellation Epsilon
            NodeTaxonomy::Event => (120.0, -40.0, 80.0),          // Constellation Zeta
            NodeTaxonomy::AtomicFact => (-50.0, -50.0, -100.0),   // Constellation Eta
        }
    }

    /// Compute single iteration of 3D N-body celestial gravitational forces
    pub fn update_3d_coordinates(nodes: &mut [MemoryNode]) {
        let n = nodes.len();
        if n == 0 {
            return;
        }

        // 1. Attract nodes toward their taxonomy galactic anchor
        for node in nodes.iter_mut() {
            let (target_x, target_y, target_z) = Self::taxonomy_anchor(&node.taxonomy);
            let dx = target_x - node.x;
            let dy = target_y - node.y;
            let dz = target_z - node.z;
            
            // Move 15% towards anchor center
            node.x += dx * 0.15;
            node.y += dy * 0.15;
            node.z += dz * 0.15;
        }

        // 2. Repulsion between close nodes
        for i in 0..n {
            for j in (i + 1)..n {
                let dx = nodes[i].x - nodes[j].x;
                let dy = nodes[i].y - nodes[j].y;
                let dz = nodes[i].z - nodes[j].z;
                let dist_sq = dx * dx + dy * dy + dz * dz + 1.0;
                let dist = dist_sq.sqrt();

                if dist < 30.0 {
                    let force = (30.0 - dist) / dist * 0.1;
                    nodes[i].x += dx * force;
                    nodes[i].y += dy * force;
                    nodes[i].z += dz * force;
                    nodes[j].x -= dx * force;
                    nodes[j].y -= dy * force;
                    nodes[j].z -= dz * force;
                }
            }
        }
    }

    /// Build constellation graph edges
    pub fn build_constellation_edges(nodes: &[MemoryNode]) -> Vec<(String, String)> {
        let mut edges = Vec::new();
        let id_map: HashMap<&str, &MemoryNode> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();

        for node in nodes {
            for target_id in &node.links {
                if id_map.contains_key(target_id.as_str()) {
                    edges.push((node.id.clone(), target_id.clone()));
                }
            }
        }
        edges
    }
}
