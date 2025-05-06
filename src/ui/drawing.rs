// src/ui/drawing.rs
use crate::sadt_model::{Arrow, ProcessNode, SadtDiagram};
use crate::sadt_elements::{ArrowType, NodeId, ArrowId, Side, ConnectionPoint};
use egui::{vec2, Color32, Painter, Pos2, Rect, Stroke, Vec2, FontId, Align2};
use crate::ui::transform::Transform; // Importer Transform

// Constantes pour le dessin
const NODE_CORNER_RADIUS: f32 = 5.0;
const ARROW_HEAD_SIZE: f32 = 10.0; // Taille de base de la tête de flèche (sera scalée)
pub const CONNECTION_POINT_RADIUS: f32 = 4.0; // Rayon monde du point de connexion

// Structure d'état UI (pas de changements ici)
#[derive(Debug, Default, Clone)]
pub struct UiState {
    pub selected_node: Option<NodeId>,
    pub selected_arrow: Option<ArrowId>,
    pub dragging_node: Option<NodeId>, // Peut être utile pour le curseur plus tard
    pub arrow_creation_start: Option<ConnectionPoint>,
    pub mouse_pos: Pos2, // Coordonnées MONDE de la souris
    pub renaming_node: Option<NodeId>,
    pub renaming_arrow: Option<ArrowId>,
    pub renaming_label_text: String,
}

// Fonction principale de dessin
pub fn draw_diagram(diagram: &SadtDiagram, transform: &Transform, painter: &Painter, ui_state: &UiState) {
    // Dessiner flèches en premier (dessous)
    for arrow in diagram.arrows.values() {
        draw_arrow(arrow, diagram, transform, painter, ui_state);
    }
    // Dessiner nœuds ensuite (dessus)
    for node in diagram.nodes.values() {
        draw_node(node, transform, painter, ui_state.selected_node == Some(node.id));
    }

    // Dessiner la flèche en cours de création (prévisualisation)
    if let Some(start_point) = &ui_state.arrow_creation_start {
       if let Some(start_node) = diagram.get_node(start_point.node_id) {
           let start_pos_world = get_connection_pos(start_node, start_point.side);
           let start_pos_screen = transform.world_to_screen(start_pos_world);
           // La position de la souris dans ui_state est déjà en monde
           let mouse_pos_screen = transform.world_to_screen(ui_state.mouse_pos);
           painter.line_segment(
               [start_pos_screen, mouse_pos_screen],
                Stroke::new(1.5, Color32::LIGHT_BLUE)
           );
       }
    }
}

// Dessine un nœud
fn draw_node(node: &ProcessNode, transform: &Transform, painter: &Painter, is_selected: bool) {
    let stroke_color = if is_selected { Color32::YELLOW } else { Color32::GRAY };
    let stroke = Stroke::new(if is_selected { 2.0 } else { 1.0 }, stroke_color);

    let screen_rect = transform.world_rect_to_screen(node.rect);

    // Optimisation: Ne pas dessiner si trop petit ou hors champ
    if !painter.clip_rect().intersects(screen_rect) || screen_rect.width() < 2.0 || screen_rect.height() < 2.0 {
        return;
    }

    let corner_radius_screen: f32 = NODE_CORNER_RADIUS * transform.zoom; // Ajuster rayon et minimum
    let corner_radius_screen = corner_radius_screen.max(1.5_f32);

    painter.rect(screen_rect, corner_radius_screen, Color32::from_gray(50), stroke);

    // Ajuster taille police et ne dessiner que si assez grand
    let font_size = 14.0 * transform.zoom.sqrt();
    if font_size > 5.0 { // Seuil minimum pour dessiner le texte
        painter.text(
            screen_rect.center(),
            Align2::CENTER_CENTER,
            &node.name,
            FontId::proportional(font_size),
            Color32::WHITE,
        );
    }


    // Dessiner points de connexion (si assez zoomé)
    let conn_point_radius_screen = CONNECTION_POINT_RADIUS * transform.zoom;
    if conn_point_radius_screen > 1.0 { // Seuil minimum pour dessiner les points
        for side in [Side::Left, Side::Right, Side::Top, Side::Bottom] {
            let world_pos = get_connection_pos(node, side);
            let screen_pos = transform.world_to_screen(world_pos);
            // Optimisation simple: ne dessine que si dans le clip_rect
            if painter.clip_rect().contains(screen_pos) {
                painter.circle_filled(screen_pos, conn_point_radius_screen.max(1.0), Color32::DARK_GRAY); // Min 1px
                painter.circle_stroke(screen_pos, conn_point_radius_screen.max(1.0), Stroke::new(1.0, Color32::GRAY));
            }
        }
    }
}

// Dessine une flèche
fn draw_arrow(arrow: &Arrow, diagram: &SadtDiagram, transform: &Transform, painter: &Painter, ui_state: &UiState) {
    if let (Some(source_node), Some(target_node)) = (
        diagram.nodes.get(&arrow.source.node_id),
        diagram.nodes.get(&arrow.target.node_id),
    ) {
        let start_pos_world = get_connection_pos(source_node, arrow.source.side);
        let end_pos_world = get_connection_pos(target_node, arrow.target.side);
        let start_pos_screen = transform.world_to_screen(start_pos_world);
        let end_pos_screen = transform.world_to_screen(end_pos_world);

        // Optimisation: vérifier si la ligne croise le rectangle visible
        let line_rect = Rect::from_two_pos(start_pos_screen, end_pos_screen);
         if !painter.clip_rect().intersects(line_rect) {
             // Si les deux points sont hors écran du même côté, on peut skipper.
             // Un test plus fin serait nécessaire pour les longues lignes traversantes.
             // Pour l'instant, on dessine si l'un ou l'autre est visible ou si le rect englobant intersecte.
             // return; // Peut cacher des flèches longues, attention.
         }

        let base_color = match arrow.arrow_type {
            ArrowType::Input => Color32::LIGHT_GREEN, ArrowType::Output => Color32::LIGHT_BLUE,
            ArrowType::Control => Color32::LIGHT_RED, ArrowType::Mechanism => Color32::LIGHT_YELLOW,
        };
        let is_selected = ui_state.selected_arrow == Some(arrow.id);
        let stroke_width = if is_selected { 3.0 } else { 1.5 }; // Epaisseur écran fixe
        let color = if is_selected { Color32::YELLOW } else { base_color };
        let stroke = Stroke::new(stroke_width, color);

        painter.line_segment([start_pos_screen, end_pos_screen], stroke);

        // Dessiner tête de flèche (taille ajustée au zoom)
        let head_size: f32 = ARROW_HEAD_SIZE * transform.zoom.sqrt();
        let head_size = head_size.max(3.0_f32); // Min 3px
        draw_arrow_head(painter, end_pos_screen, start_pos_screen, head_size, color);

        // Dessiner label (si assez zoomé et pas en cours d'édition)
        let font_size: f32 = 10.0 * transform.zoom.sqrt();
        let font_size = font_size.max(6.0_f32); // Min 6px
        if font_size > 6.0 && ui_state.renaming_arrow != Some(arrow.id) {
            if let Some(label) = &arrow.label {
                let mid_screen_pos = start_pos_screen.lerp(end_pos_screen, 0.5);
                 // Petit décalage pour ne pas être pile sur la ligne
                 let text_pos = mid_screen_pos + vec2(0.0, -stroke_width - 2.0);
                 painter.text(text_pos, Align2::CENTER_CENTER, label, FontId::proportional(font_size), color);
            }
        }
    } else {
        log::warn!("Impossible de dessiner flèche {}: nœud source/cible manquant.", arrow.id);
    }
}

// Calcule la position monde d'un point de connexion
pub fn get_connection_pos(node: &ProcessNode, side: Side) -> Pos2 {
    match side {
        Side::Left => Pos2::new(node.rect.left(), node.rect.center().y),
        Side::Right => Pos2::new(node.rect.right(), node.rect.center().y),
        Side::Top => Pos2::new(node.rect.center().x, node.rect.top()),
        Side::Bottom => Pos2::new(node.rect.center().x, node.rect.bottom()),
    }
}

// Trouve le point de connexion le plus proche (en coordonnées monde)
pub fn find_closest_connection_point(
    diagram: &SadtDiagram,
    world_pos: Pos2,
    max_dist_world: f32, // Comparaison en distance monde
) -> Option<ConnectionPoint> {
    let max_dist_sq = max_dist_world * max_dist_world;
    let mut closest_point: Option<ConnectionPoint> = None;
    let mut min_dist_sq = max_dist_sq;

    for node in diagram.nodes.values() {
         for side in [Side::Left, Side::Right, Side::Top, Side::Bottom] {
            let conn_pos_world = get_connection_pos(node, side);
            let dist_sq = conn_pos_world.distance_sq(world_pos);
            if dist_sq < min_dist_sq {
                min_dist_sq = dist_sq;
                closest_point = Some(ConnectionPoint { node_id: node.id, side });
            }
         }
    }
    closest_point
}

// Dessine la tête de flèche (en coordonnées écran)
fn draw_arrow_head(painter: &Painter, tip: Pos2, origin: Pos2, size: f32, color: Color32) {
    let dir = (tip - origin).normalized();
    if dir.length_sq() < 0.1 { return; }
    let normal = Vec2::new(-dir.y, dir.x);
    let p1 = tip - dir * size; // Base de la tête
    let p2 = p1 + normal * size / 2.0; // Pointe côté 1
    let p3 = p1 - normal * size / 2.0; // Pointe côté 2
    painter.add(egui::Shape::convex_polygon(vec![tip, p2, p3], color, Stroke::NONE));
}