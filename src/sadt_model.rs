use crate::sadt_elements::{ArrowId, ArrowType, ConnectionPoint, NodeId, Side};
use egui::Rect;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessNode {
    pub id: NodeId,
    pub name: String,
    pub rect: Rect, // Position et taille dans l'UI egui
    pub algorithm: String, // Name of the selected algorithm
    // On pourrait stocker les IDs des flèches connectées ici, mais
    // il est souvent plus simple de les retrouver via le diagramme global.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arrow {
    pub id: ArrowId,
    pub label: Option<String>,
    pub arrow_type: ArrowType,
    pub source: ConnectionPoint,
    pub target: ConnectionPoint,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SadtDiagram {
    pub nodes: HashMap<NodeId, ProcessNode>,
    pub arrows: HashMap<ArrowId, Arrow>,
    // Potentiellement: métadonnées du diagramme (nom, version, etc.)
}

impl SadtDiagram {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_node(&mut self, name: String, pos: egui::Pos2) -> NodeId {
        let id = Uuid::new_v4();
        let node = ProcessNode {
            id,
            name,
            rect: Rect::from_min_size(pos, egui::vec2(120.0, 60.0)),
            algorithm: "add".to_string(), // Default algorithm
        };
        self.nodes.insert(id, node);
        id
    }

    pub fn add_arrow(
        &mut self,
        source: ConnectionPoint,
        target: ConnectionPoint,
        arrow_type: ArrowType,
        label: Option<String>,
    ) -> Option<ArrowId> {
        // Vérifier que les noeuds source et target existent
        if !self.nodes.contains_key(&source.node_id) || !self.nodes.contains_key(&target.node_id) {
           log::warn!("Tentative de création de flèche vers/depuis un noeud inexistant.");
           return None;
        }

        // Vérifier les règles SADT (ex: Input à gauche, Output à droite, etc.)
        // Note: Pour un prototype, on peut être plus flexible initialement.
        match arrow_type {
            ArrowType::Input => if source.side != Side::Left && target.side != Side::Left { /* Pas SADT strict */ },
            ArrowType::Output => if source.side != Side::Right && target.side != Side::Right { /* Pas SADT strict */ },
            ArrowType::Control => if source.side != Side::Top && target.side != Side::Top { /* Pas SADT strict */ },
            ArrowType::Mechanism => if source.side != Side::Bottom && target.side != Side::Bottom { /* Pas SADT strict */ },
        }


        let id = Uuid::new_v4();
        let arrow = Arrow {
            id,
            label,
            arrow_type,
            source,
            target,
        };
        self.arrows.insert(id, arrow);
        Some(id)
    }

    pub fn get_node(&self, id: NodeId) -> Option<&ProcessNode> {
        self.nodes.get(&id)
    }

     pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut ProcessNode> {
        self.nodes.get_mut(&id)
    }

    pub fn get_arrow(&self, id: ArrowId) -> Option<&Arrow> {
        self.arrows.get(&id)
    }

    pub fn remove_node(&mut self, id: NodeId) -> Option<ProcessNode> {
        // Supprimer aussi les flèches connectées
        self.arrows.retain(|_, arrow| arrow.source.node_id != id && arrow.target.node_id != id);
        self.nodes.remove(&id)
    }

     pub fn remove_arrow(&mut self, id: ArrowId) -> Option<Arrow> {
        self.arrows.remove(&id)
    }
}
