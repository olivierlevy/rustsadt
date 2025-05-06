use crate::sadt_model::{Arrow, ProcessNode, SadtDiagram}; // ConnectionPoint était déjà dans sadt_model aussi, consolidation
use crate::sadt_elements::{ArrowType, NodeId, ArrowId, Side, ConnectionPoint}; // Ajout des imports manquants
use egui::{Color32, Painter, Pos2, Stroke, Vec2}; // Rect n'était pas utilisé, mais on le garde pour l'instant

const NODE_CORNER_RADIUS: f32 = 5.0;
const ARROW_HEAD_SIZE: f32 = 10.0;
pub const CONNECTION_POINT_RADIUS: f32 = 4.0; // Rayon du petit cercle indiquant où connecter

pub fn draw_diagram(diagram: &SadtDiagram, painter: &Painter, ui_state: &UiState) {
    for node in diagram.nodes.values() {
        draw_node(node, painter, ui_state.selected_node == Some(node.id));
    }
    for arrow in diagram.arrows.values() {
        draw_arrow(arrow, diagram, painter, ui_state);
    }

    // Dessiner la flèche en cours de création
    if let Some(start_point) = &ui_state.arrow_creation_start {
       if let Some(start_node) = diagram.get_node(start_point.node_id) {
           let start_pos = get_connection_pos(start_node, start_point.side);
           painter.line_segment([start_pos, ui_state.mouse_pos], Stroke::new(1.5, Color32::LIGHT_BLUE));
       }
    }
}

fn draw_node(node: &ProcessNode, painter: &Painter, is_selected: bool) {
    let stroke_color = if is_selected { Color32::YELLOW } else { Color32::GRAY };
    let stroke = Stroke::new(if is_selected { 2.0 } else { 1.0 }, stroke_color);

    painter.rect(
        node.rect,
        NODE_CORNER_RADIUS,
        Color32::from_gray(50), // Couleur de fond
        stroke,
    );

    // Dessiner le nom du noeud centré
    painter.text(
        node.rect.center(),
        egui::Align2::CENTER_CENTER,
        &node.name,
        egui::FontId::proportional(14.0),
        Color32::WHITE,
    );

    // Dessiner les points de connexion (visuels pour l'utilisateur)
    for side in [Side::Left, Side::Right, Side::Top, Side::Bottom] {
        let pos = get_connection_pos(node, side);
        painter.circle_filled(pos, CONNECTION_POINT_RADIUS, Color32::DARK_GRAY);
        painter.circle_stroke(pos, CONNECTION_POINT_RADIUS, Stroke::new(1.0, Color32::GRAY));
    }
}

fn draw_arrow(arrow: &Arrow, diagram: &SadtDiagram, painter: &Painter, ui_state: &UiState) {
    if let (Some(source_node), Some(target_node)) = (
        diagram.get_node(arrow.source.node_id),
        diagram.get_node(arrow.target.node_id),
    ) {
        let start_pos = get_connection_pos(source_node, arrow.source.side);
        let end_pos = get_connection_pos(target_node, arrow.target.side);

        let base_color = match arrow.arrow_type {
            ArrowType::Input => Color32::LIGHT_GREEN,
            ArrowType::Output => Color32::LIGHT_BLUE,
            ArrowType::Control => Color32::LIGHT_RED,
            ArrowType::Mechanism => Color32::LIGHT_YELLOW,
        };
        let is_selected = ui_state.selected_arrow == Some(arrow.id);
        let stroke_width = if is_selected { 3.0 } else { 1.5 };
        let color = if is_selected { Color32::YELLOW } else { base_color }; // Jaune si sélectionnée
        let stroke = Stroke::new(stroke_width, color);

        painter.line_segment([start_pos, end_pos], stroke);

        // Dessiner la tête de flèche
        draw_arrow_head(painter, end_pos, start_pos, color);

        // Dessiner le label (simplifié, au milieu)
        // Ne pas dessiner le label si on est en train de l'éditer (pour éviter superposition)
        if ui_state.renaming_arrow != Some(arrow.id) {
            if let Some(label) = &arrow.label {
                painter.text(
                    start_pos.lerp(end_pos, 0.5), // Position au milieu
                    egui::Align2::CENTER_CENTER,
                    label,
                    egui::FontId::proportional(10.0),
                    color,
                );
            }
        }
    } else {
        log::warn!("Impossible de dessiner la flèche {}: noeud source ou cible manquant.", arrow.id);
    }
}

// Calcule la position absolue d'un point de connexion sur un côté
pub fn get_connection_pos(node: &ProcessNode, side: Side) -> Pos2 {
    match side {
        Side::Left => Pos2::new(node.rect.left(), node.rect.center().y),
        Side::Right => Pos2::new(node.rect.right(), node.rect.center().y),
        Side::Top => Pos2::new(node.rect.center().x, node.rect.top()),
        Side::Bottom => Pos2::new(node.rect.center().x, node.rect.bottom()),
    }
}

// Trouve le point de connexion le plus proche d'une position donnée
pub fn find_closest_connection_point(
    diagram: &SadtDiagram,
    pos: Pos2,
    max_dist: f32,
) -> Option<ConnectionPoint> {
    let max_dist_sq = max_dist * max_dist;
    let mut closest_point: Option<ConnectionPoint> = None;
    let mut min_dist_sq = max_dist_sq;

    for node in diagram.nodes.values() {
         for side in [Side::Left, Side::Right, Side::Top, Side::Bottom] {
            let conn_pos = get_connection_pos(node, side);
            let dist_sq = conn_pos.distance_sq(pos);
            if dist_sq < min_dist_sq {
                min_dist_sq = dist_sq;
                closest_point = Some(ConnectionPoint { node_id: node.id, side });
            }
         }
    }
    closest_point
}


fn draw_arrow_head(painter: &Painter, tip: Pos2, origin: Pos2, color: Color32) {
    let dir = (tip - origin).normalized();
    if dir.length_sq() < 0.1 { return; } // Avoid division by zero or NaN

    let normal = Vec2::new(-dir.y, dir.x); // Perpendiculaire

    let p1 = tip - dir * ARROW_HEAD_SIZE;
    let p2 = p1 + normal * ARROW_HEAD_SIZE / 2.0;
    let p3 = p1 - normal * ARROW_HEAD_SIZE / 2.0;

    painter.add(egui::Shape::convex_polygon(
        vec![tip, p2, p3],
        color,
        Stroke::NONE,
    ));
}

// Structure temporaire pour l'état de l'UI (sélection, etc.)
// Sera typiquement dans `app.rs`
#[derive(Debug, Default)]
pub struct UiState {
    pub selected_node: Option<NodeId>,
    pub selected_arrow: Option<ArrowId>,
    pub dragging_node: Option<NodeId>,
    pub arrow_creation_start: Option<ConnectionPoint>, // Point de départ de la flèche en cours
    pub mouse_pos: Pos2, // Dernière position connue de la souris sur le canvas
    pub renaming_node: Option<NodeId>, // ID du noeud en cours de renommage
    pub renaming_arrow: Option<ArrowId>, // ID de la flèche en cours de renommage
    pub renaming_label_text: String,     // Texte temporaire pour l'édition (nœud ou flèche)
}