use crate::app::AppState;
use crate::sadt_elements::{ArrowType, Side};
use crate::ui::drawing::{find_closest_connection_point, get_connection_pos, CONNECTION_POINT_RADIUS};
use crate::ui::transform::Transform; // Importer Transform
use egui::{vec2, Context, Key, PointerButton, Pos2, Ui, Response};

const ARROW_SELECT_DISTANCE: f32 = 5.0; // Tolérance écran pour sélectionner une flèche

// Helper: Calcule la distance² d'un point à un segment de ligne (en coordonnées monde)
fn distance_sq_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let l2 = a.distance_sq(b);
    if l2 == 0.0 { return p.distance_sq(a); }
    let t = ((p.x - a.x) * (b.x - a.x) + (p.y - a.y) * (b.y - a.y)) / l2;
    let t = t.clamp(0.0, 1.0);
    let projection = a + (b - a) * t;
    p.distance_sq(projection)
}

// Fonction principale appelée depuis app.rs
pub fn handle_canvas_interactions(
    ctx: &Context,
    _ui: &mut Ui,
    response: &Response,
    transform: &Transform, // Prend la transformation en argument
    app_state: &mut AppState,
) {
    // Stocker la position MONDE de la souris (déjà calculée dans app.rs)
    // let screen_pos_mouse = response.hover_pos().unwrap_or_else(|| response.rect.center());
    // app_state.ui_state.mouse_pos = transform.screen_to_world(screen_pos_mouse);
    // Est fait dans app.rs maintenant, ui_state.mouse_pos est déjà en monde

    handle_arrow_selection(ctx, response, transform, app_state);
    handle_node_drag_and_select(ctx, response, transform, app_state);
    handle_arrow_creation(ctx, response, transform, app_state);
    handle_rename(ctx, transform, app_state); // Passe transform pour positionnement fenêtre
    handle_deletion(ctx, app_state);

    // --- Menu Contextuel (utilise transform pour position ajout nœud) ---
    response.context_menu(|ui| {
        if let Some(node_id) = app_state.ui_state.selected_node {
             if ui.button("Renommer Nœud").clicked() {
                 app_state.ui_state.renaming_node = Some(node_id);
                 if let Some(node) = app_state.diagram.get_node(node_id) {
                    app_state.ui_state.renaming_label_text = node.name.clone();
                 } else { app_state.ui_state.renaming_label_text = String::new(); }
                 app_state.ui_state.renaming_arrow = None; ui.close_menu();
            }
            ui.separator();
            if ui.button("Supprimer Nœud").clicked() {
                if let Some(id) = app_state.ui_state.selected_node.take() {
                    app_state.diagram.remove_node(id); log::info!("Nœud {} supprimé via menu", id);
                } ui.close_menu();
            }
        }
        else if let Some(arrow_id) = app_state.ui_state.selected_arrow {
            if ui.button("Editer Label Flèche").clicked() {
                app_state.ui_state.renaming_arrow = Some(arrow_id);
                 if let Some(arrow) = app_state.diagram.get_arrow(arrow_id) {
                    app_state.ui_state.renaming_label_text = arrow.label.clone().unwrap_or_default();
                 } else { app_state.ui_state.renaming_label_text = String::new(); }
                app_state.ui_state.renaming_node = None; ui.close_menu();
            }
             ui.separator();
            if ui.button("Supprimer Flèche").clicked() {
                 if let Some(id) = app_state.ui_state.selected_arrow.take() {
                    app_state.diagram.remove_arrow(id); log::info!("Flèche {} supprimée via menu", id);
                 } ui.close_menu();
            }
        } else {
            if ui.button("Ajouter Nœud").clicked() {
                let screen_pos = ctx.input(|i| i.pointer.interact_pos()).unwrap_or_else(|| response.rect.center());
                let world_pos = transform.screen_to_world(screen_pos);
                 let node_name = format!("Activité {}", app_state.diagram.nodes.len() + 1);
                app_state.diagram.add_node(node_name, world_pos);
                 log::info!("Nœud ajouté via menu contextuel à monde {:?}", world_pos);
                 ui.close_menu();
            }
        }
    });
}

// Gère sélection et drag des nœuds
fn handle_node_drag_and_select(ctx: &Context, response: &Response, transform: &Transform, app_state: &mut AppState) {
    let pointer = &ctx.input(|i| i.pointer.clone());

    if response.hovered() && pointer.button_pressed(PointerButton::Primary) {
        let click_pos_screen = pointer.press_origin().unwrap_or_default();
        let click_pos_world = transform.screen_to_world(click_pos_screen);

        let clicked_on_connection_point = find_closest_connection_point(
            &app_state.diagram, click_pos_world, CONNECTION_POINT_RADIUS * 3.0).is_some();

        let mut clicked_on_node = None;
        if !clicked_on_connection_point {
            for node in app_state.diagram.nodes.values() {
                if node.rect.contains(click_pos_world) {
                    clicked_on_node = Some(node.id); break;
                }
            }
        }

        if let Some(node_id) = clicked_on_node {
             if app_state.ui_state.arrow_creation_start.is_none() {
                app_state.ui_state.selected_node = Some(node_id);
                app_state.ui_state.selected_arrow = None;
                 ctx.memory_mut(|mem| mem.set_dragged_id(response.id)); // Marquer pour drag du canvas
                 log::trace!("Nœud {} sélectionné pour drag potentiel", node_id);
             } else { log::trace!("Clic sur nœud ignoré (création flèche en cours)"); }
        } else { log::trace!("Clic ni sur nœud ni sur point connexion (pour sélection nœud)"); }
    }

    if pointer.button_down(PointerButton::Primary) && ctx.is_being_dragged(response.id) && app_state.ui_state.arrow_creation_start.is_none() {
        if let Some(node_id) = app_state.ui_state.selected_node {
            let delta_world = transform.screen_vec_to_world(pointer.delta());
            if delta_world.length_sq() > 0.0 { // Seulement si mouvement réel
                if let Some(node) = app_state.diagram.get_node_mut(node_id) {
                    node.rect = node.rect.translate(delta_world);
                    log::trace!("Dragging node {} par monde {:?}", node_id, delta_world);
                }
            }
        }
    }

    if pointer.any_released() && ctx.is_being_dragged(response.id) {
        // Ne pas appeler stop_dragging() ici car on pourrait vouloir continuer le pan
        // ctx.stop_dragging(); // Retiré
    }
}

// Gère sélection des flèches et désélection dans le vide
fn handle_arrow_selection(ctx: &Context, response: &Response, transform: &Transform, app_state: &mut AppState) {
    let pointer = &ctx.input(|i| i.pointer.clone());

    if response.hovered() && pointer.button_pressed(PointerButton::Primary)
        && !ctx.is_being_dragged(response.id) // Ignore si on drague un nœud
        && app_state.ui_state.arrow_creation_start.is_none() // Ignore si on crée une flèche
    {
        let click_pos_screen = pointer.press_origin().unwrap_or_default();
        let click_pos_world = transform.screen_to_world(click_pos_screen);
        let mut clicked_on_arrow = None;

        let selection_dist_world_sq = (ARROW_SELECT_DISTANCE / transform.zoom).powi(2);

        for arrow in app_state.diagram.arrows.values() {
            if let (Some(src_node), Some(tgt_node)) = (
                app_state.diagram.get_node(arrow.source.node_id),
                app_state.diagram.get_node(arrow.target.node_id),
            ) {
                let start_pos_world = get_connection_pos(src_node, arrow.source.side);
                let end_pos_world = get_connection_pos(tgt_node, arrow.target.side);
                if distance_sq_to_segment(click_pos_world, start_pos_world, end_pos_world) < selection_dist_world_sq {
                    clicked_on_arrow = Some(arrow.id); break;
                }
            }
        }

        if let Some(arrow_id) = clicked_on_arrow {
             let clicked_on_conn_point = find_closest_connection_point(&app_state.diagram, click_pos_world, CONNECTION_POINT_RADIUS * 3.0).is_some();
             if !clicked_on_conn_point {
                app_state.ui_state.selected_arrow = Some(arrow_id);
                app_state.ui_state.selected_node = None;
                log::debug!("Flèche sélectionnée: {}", arrow_id);
             } else { log::trace!("Clic sur flèche ignoré (proche point connexion)"); }
        } else {
             let clicked_on_conn_point = find_closest_connection_point(&app_state.diagram, click_pos_world, CONNECTION_POINT_RADIUS * 3.0).is_some();
             let mut clicked_on_node = false;
             for node in app_state.diagram.nodes.values() { if node.rect.contains(click_pos_world) { clicked_on_node = true; break; } }

             if !clicked_on_conn_point && !clicked_on_node {
                log::trace!("Clic détecté dans le vide, désélection.");
                app_state.ui_state.selected_node = None;
                app_state.ui_state.selected_arrow = None;
             }
        }
    }
}

// Gère création de flèches
fn handle_arrow_creation(ctx: &Context, response: &Response, transform: &Transform, app_state: &mut AppState) {
    let pointer = &ctx.input(|i| i.pointer.clone());
    let mouse_pos_world = app_state.ui_state.mouse_pos; // Position monde de la souris

    if pointer.button_pressed(PointerButton::Primary) {
        let click_pos_screen = pointer.press_origin().unwrap_or_default();
        let click_pos_world = transform.screen_to_world(click_pos_screen);
        if let Some(start_point) = find_closest_connection_point(&app_state.diagram, click_pos_world, CONNECTION_POINT_RADIUS * 3.0) {
            if !ctx.is_being_dragged(response.id) {
                 app_state.ui_state.selected_node = None;
                 app_state.ui_state.selected_arrow = None;
                 app_state.ui_state.arrow_creation_start = Some(start_point.clone());
                 log::debug!("Début création flèche depuis: {:?}", start_point);
            } else { log::trace!("Ignoré début flèche (drag en cours)"); }
        }
    }

    if let Some(start_point) = &app_state.ui_state.arrow_creation_start {
        if pointer.any_released() {
             log::debug!("Relâchement détecté, tentative fin flèche à monde {:?}", mouse_pos_world);
             if let Some(end_point) = find_closest_connection_point(&app_state.diagram, mouse_pos_world, CONNECTION_POINT_RADIUS * 4.0) {
                if start_point.node_id != end_point.node_id && app_state.diagram.nodes.contains_key(&end_point.node_id) {
                    let arrow_type = match (start_point.side, end_point.side) {
                        (Side::Right, Side::Left) => ArrowType::Output, (Side::Left, Side::Right) => ArrowType::Input,
                        (_, Side::Top) => ArrowType::Control, (_, Side::Bottom) => ArrowType::Mechanism,
                        (Side::Right, _) => ArrowType::Output, (_, Side::Left) => ArrowType::Input,
                        _ => ArrowType::Input,
                    };
                    log::debug!("Fin création flèche vers: {:?}, type: {:?}", end_point, arrow_type);
                    app_state.diagram.add_arrow(start_point.clone(), end_point, arrow_type, None);
                } else { log::debug!("Annulation flèche (même nœud ou cible invalide)"); }
            } else { log::debug!("Annulation flèche (relâchement dans vide)"); }
            app_state.ui_state.arrow_creation_start = None; // Toujours réinitialiser
        }
    }
}

// Gère renommage nœuds et flèches
fn handle_rename(ctx: &Context, transform: &Transform, app_state: &mut AppState) { // Prend Transform
    // Renommage Nœud
    if let Some(node_id) = app_state.ui_state.renaming_node {
        if let Some(node) = app_state.diagram.get_node(node_id) {
             // Positionner la fenêtre près du nœud (conversion écran)
             let node_screen_rect = transform.world_rect_to_screen(node.rect);
             egui::Window::new("Renommer Nœud")
                .collapsible(false).resizable(false)
                .default_pos(node_screen_rect.center_bottom() + vec2(0.0, 5.0)) // Sous le nœud à l'écran
                .show(ctx, |ui| {
                    let text_edit_response = ui.text_edit_singleline(&mut app_state.ui_state.renaming_label_text);
                    if !ctx.memory(|mem| mem.has_focus(text_edit_response.id)) { text_edit_response.request_focus(); }

                    let mut close = false; let mut success = false;
                    if text_edit_response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) { success = true; close = true; }
                    else if !text_edit_response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) { success = true; close = true; }
                    else if ui.input(|i| i.key_pressed(Key::Escape)) { success = false; close = true; }
                    else if text_edit_response.lost_focus() { success = false; close = true; log::debug!("Annulation renommage nœud (focus perdu)"); }

                    if close {
                        if success { if let Some(n) = app_state.diagram.get_node_mut(node_id) { n.name = app_state.ui_state.renaming_label_text.clone(); log::info!("Nœud renommé"); } }
                        else { log::info!("Renommage nœud annulé"); }
                        app_state.ui_state.renaming_node = None; app_state.ui_state.renaming_label_text.clear();
                    }
                });
        } else { app_state.ui_state.renaming_node = None; }
    }
    // Renommage Flèche
    else if let Some(arrow_id) = app_state.ui_state.renaming_arrow {
         if let Some(arrow) = app_state.diagram.get_arrow(arrow_id) {
             let mid_pos_world = if let (Some(src), Some(tgt)) = (app_state.diagram.get_node(arrow.source.node_id), app_state.diagram.get_node(arrow.target.node_id)) {
                 get_connection_pos(src, arrow.source.side).lerp(get_connection_pos(tgt, arrow.target.side), 0.5)
             } else { Pos2::ZERO };
             let mid_pos_screen = transform.world_to_screen(mid_pos_world); // Position écran

             egui::Window::new("Editer Label Flèche")
                .collapsible(false).resizable(false)
                .default_pos(mid_pos_screen + vec2(0.0, -20.0)) // Près du milieu écran
                .show(ctx, |ui| {
                    let text_edit_response = ui.text_edit_singleline(&mut app_state.ui_state.renaming_label_text);
                    if !ctx.memory(|mem| mem.has_focus(text_edit_response.id)) { text_edit_response.request_focus(); }

                    let mut close = false; let mut success = false;
                    if text_edit_response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) { success = true; close = true; }
                    else if !text_edit_response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) { success = true; close = true; }
                    else if ui.input(|i| i.key_pressed(Key::Escape)) { success = false; close = true; }
                    else if text_edit_response.lost_focus() { success = false; close = true; log::debug!("Annulation edit label (focus perdu)"); }

                    if close {
                        if success { if let Some(a) = app_state.diagram.arrows.get_mut(&arrow_id) { let lbl = app_state.ui_state.renaming_label_text.clone(); a.label = if lbl.is_empty() { None } else { Some(lbl) }; log::info!("Label flèche édité"); } }
                        else { log::info!("Edition label flèche annulée"); }
                        app_state.ui_state.renaming_arrow = None; app_state.ui_state.renaming_label_text.clear();
                    }
                });
        } else { app_state.ui_state.renaming_arrow = None; }
    }
}

// Gère suppression via clavier
fn handle_deletion(ctx: &Context, app_state: &mut AppState) {
    if ctx.input(|i| i.key_pressed(Key::Delete)) || ctx.input(|i| i.key_pressed(Key::Backspace)) {
        if let Some(node_id) = app_state.ui_state.selected_node.take() {
             log::info!("Suppression noeud via clavier: {}", node_id);
            app_state.diagram.remove_node(node_id);
            app_state.ui_state.renaming_node = None; app_state.ui_state.renaming_arrow = None;
        } else if let Some(arrow_id) = app_state.ui_state.selected_arrow.take() {
             log::info!("Suppression flèche via clavier: {}", arrow_id);
             app_state.diagram.remove_arrow(arrow_id);
             app_state.ui_state.renaming_arrow = None; app_state.ui_state.renaming_node = None;
        }
    }
}