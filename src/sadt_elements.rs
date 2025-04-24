use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Alias pour clarté
pub type NodeId = Uuid;
pub type ArrowId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArrowType {
    Input,    // Donnée entrant dans l'activité
    Output,   // Donnée sortant de l'activité
    Control,  // Contrainte ou règle guidant l'activité
    Mechanism, // Ressource utilisée par l'activité (humain, machine)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

// Représente un point de connexion sur un côté d'un nœud
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectionPoint {
    pub node_id: NodeId,
    pub side: Side,
    // Position relative sur le côté (0.0 à 1.0) - Optionnel pour l'instant, on peut connecter au milieu
    // pub relative_pos: f32,
}