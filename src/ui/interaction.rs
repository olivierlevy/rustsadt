use crate::app::AppState;
use crate::sadt_elements::{ArrowType, ConnectionPoint, NodeId, Side, ArrowId};
use crate::ui::drawing::{find_closest_connection_point, get_connection_pos, CONNECTION_POINT_RADIUS};
use egui::{Context, Key, PointerButton, Pos2, Ui, Response, Vec2, Align2}; // Ajout Align2 si utilisé dans la fenêtre de renommage
use egui::epaint::Hsva; // Pour manipuler les couleurs si besoin

const ARROW_SELECT_DISTANCE: f32 = 5.0; // Tolérance pour sélectionner une flèche

// Helper: Calcule la distance² d'un point à un segment de ligne
fn distance_sq_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let l2 = a.distance_sq(b);
    if l2 == 0.0 { return p.distance_sq(a); }
    let t = ((p.x - a.x) * (b.x - a.x) + (p.y - a.y) * (b.y - a.y)) / l2;
    let t = t.clamp(0.0, 1.0);
    let projection = a + (b - a) * t;
    p.distance_sq(projection)
}

pub fn handle_canvas_interactions(
    ctx: &Context,
    ui: &mut Ui, // Garder ui pour le menu contextuel
    response: &egui::Response,
    app_state: &mut AppState,
) {
    app_state.ui_state.mouse_pos = response.hover_pos().unwrap_or_default();

    // Ordre important: Vérifier sélection flèche et clic vide *avant* drag/select nœud
    // pour permettre la désélection et éviter les conflits
    handle_arrow_selection(ctx, response, app_state);
    handle_node_drag_and_select(ctx, response, app_state); // S'exécute après pour que la sélection de flèche ait pu désélectionner le nœud
    handle_arrow_creation(ctx, response, app_state); // Démarrer après la gestion des clics de sélection
    handle_rename(ctx, app_state);
    handle_deletion(ctx, app_state);

    // --- Menu Contextuel ---
    response.context_menu(|ui| {
        // Menu pour Nœud sélectionné
        if let Some(node_id) = app_state.ui_state.selected_node {
            if ui.button("Renommer Nœud").clicked() {
                 app_state.ui_state.renaming_node = Some(node_id);
                 if let Some(node) = app_state.diagram.get_node(node_id) {
                    app_state.ui_state.renaming_label_text = node.name.clone();
                 } else {
                    app_state.ui_state.renaming_label_text = String::new();
                 }
                 app_state.ui_state.renaming_arrow = None;
                 ui.close_menu();
            }
            ui.separator();
            if ui.button("Supprimer Nœud").clicked() {
                if let Some(id) = app_state.ui_state.selected_node.take() {
                    app_state.diagram.remove_node(id);
                    log::info!("Nœud {} supprimé via menu contextuel", id);
                }
                ui.close_menu();
            }
        }
        // Menu pour Flèche sélectionnée
        else if let Some(arrow_id) = app_state.ui_state.selected_arrow {
            if ui.button("Editer Label Flèche").clicked() {
                app_state.ui_state.renaming_arrow = Some(arrow_id);
                 if let Some(arrow) = app_state.diagram.get_arrow(arrow_id) {
                    app_state.ui_state.renaming_label_text = arrow.label.clone().unwrap_or_default();
                 } else {
                    app_state.ui_state.renaming_label_text = String::new();
                 }
                app_state.ui_state.renaming_node = None;
                ui.close_menu();
            }
             ui.separator();
            if ui.button("Supprimer Flèche").clicked() {
                 if let Some(id) = app_state.ui_state.selected_arrow.take() {
                    app_state.diagram.remove_arrow(id);
                    log::info!("Flèche {} supprimée via menu contextuel", id);
                 }
                ui.close_menu();
            }
        } else {
            // Menu pour Canvas (rien de sélectionné)
            if ui.button("Ajouter Nœud").clicked() {
                let pos = ctx.input(|i| i.pointer.interact_pos()).unwrap_or_else(|| response.rect.center());
                 let node_name = format!("Activité {}", app_state.diagram.nodes.len() + 1);
                app_state.diagram.add_node(node_name, pos);
                 log::info!("Nœud ajouté via menu contextuel à {:?}", pos);
                 ui.close_menu();
            }
        }
    });
}

fn handle_node_drag_and_select(ctx: &Context, response: &egui::Response, app_state: &mut AppState) {
    let pointer = &ctx.input(|i| i.pointer.clone());

    // Détection début de drag ou sélection de nœud
    if response.hovered() && pointer.button_pressed(PointerButton::Primary) {
        let click_pos = pointer.press_origin().unwrap_or_default();

        // Vérifier si on clique sur un point de connexion AVANT de sélectionner le nœud
        let clicked_on_connection_point = find_closest_connection_point(
            &app_state.diagram,
            click_pos,
            CONNECTION_POINT_RADIUS * 3.0,
        ).is_some();

        let mut clicked_on_node = None;
        if !clicked_on_connection_point { // NE chercher le nœud QUE si on n'a PAS cliqué sur un point de connexion
            for node in app_state.diagram.nodes.values() {
                if node.rect.contains(click_pos) {
                    clicked_on_node = Some(node.id);
                    break;
                }
            }
        }

        if let Some(node_id) = clicked_on_node {
             log::trace!("Clic détecté sur le nœud {}", node_id);
             // Ne sélectionner que si on ne démarre pas une flèche (vérification redondante mais sûre)
             if app_state.ui_state.arrow_creation_start.is_none() {
                app_state.ui_state.selected_node = Some(node_id);
                app_state.ui_state.selected_arrow = None; // Désélectionner flèche
                // Marquer pour potentiel drag
                 ctx.memory_mut(|mem| mem.set_dragged_id(response.id));
             } else {
                 log::trace!("Clic sur noeud ignoré pour sélection car arrow_creation_start est Some");
             }
        } else {
             log::trace!("Clic non détecté sur un nœud (ou sur un point de connexion)");
             // La désélection générale est gérée par handle_arrow_selection en cas de clic dans le vide
        }
    }

    // Gestion du drag en cours
    if pointer.button_down(PointerButton::Primary) && ctx.is_being_dragged(response.id) && app_state.ui_state.arrow_creation_start.is_none() { // Ne pas dragger si on crée une flèche
        if let Some(node_id) = app_state.ui_state.selected_node {
            log::trace!("Dragging node {}", node_id);
            if let Some(node) = app_state.diagram.get_node_mut(node_id) {
                node.rect = node.rect.translate(pointer.delta());
            }
        }
    }

    // Fin du drag
    if pointer.any_released() && ctx.is_being_dragged(response.id) { // S'assurer qu'on arrète le drag QUE si on dragait ce canvas
        ctx.stop_dragging();
    }
}

// Fonction pour gérer la sélection des flèches et la désélection globale
fn handle_arrow_selection(ctx: &Context, response: &egui::Response, app_state: &mut AppState) {
    let pointer = &ctx.input(|i| i.pointer.clone());

    // Détection clic pour sélection de flèche OU clic dans le vide
    if response.hovered()
        && pointer.button_pressed(PointerButton::Primary)
        && !ctx.is_being_dragged(response.id) // Pas pendant un drag de noeud
        && app_state.ui_state.arrow_creation_start.is_none() // Pas pendant création de flèche
    {
        let click_pos = pointer.press_origin().unwrap_or_default();
        let mut clicked_on_arrow = None;

        // Vérifier si on a cliqué près d'une flèche
        for arrow in app_state.diagram.arrows.values() {
            if let (Some(src_node), Some(tgt_node)) = (
                app_state.diagram.get_node(arrow.source.node_id),
                app_state.diagram.get_node(arrow.target.node_id),
            ) {
                let start_pos = get_connection_pos(src_node, arrow.source.side);
                let end_pos = get_connection_pos(tgt_node, arrow.target.side);
                if distance_sq_to_segment(click_pos, start_pos, end_pos) < ARROW_SELECT_DISTANCE.powi(2) {
                    clicked_on_arrow = Some(arrow.id);
                    break;
                }
            }
        }

        if let Some(arrow_id) = clicked_on_arrow {
            // Clic sur une flèche
            // Ne sélectionner que si on n'a pas cliqué sur un point de connexion
             let clicked_on_connection_point = find_closest_connection_point(
                &app_state.diagram,
                click_pos,
                CONNECTION_POINT_RADIUS * 3.0,
             ).is_some();

             if !clicked_on_connection_point {
                app_state.ui_state.selected_arrow = Some(arrow_id);
                app_state.ui_state.selected_node = None; // Désélectionner nœud
                log::debug!("Flèche sélectionnée: {}", arrow_id);
             } else {
                 log::trace!("Clic sur flèche ignoré car trop proche d'un point de connexion");
             }
        } else {
             // Clic DANS LE VIDE (ni nœud (vérifié dans l'autre fonction), ni flèche, ni point de connexion)
             let clicked_on_connection_point = find_closest_connection_point(
                &app_state.diagram,
                click_pos,
                CONNECTION_POINT_RADIUS * 3.0,
             ).is_some();
             // Vérifier aussi qu'on n'a pas cliqué sur un nœud (au cas où l'ordre d'exécution changerait)
             let mut clicked_on_node = false;
             for node in app_state.diagram.nodes.values() {
                if node.rect.contains(click_pos) {
                    clicked_on_node = true;
                    break;
                }
            }

            if !clicked_on_connection_point && !clicked_on_node {
                log::trace!("Clic détecté dans le vide, désélection.");
                app_state.ui_state.selected_node = None;
                app_state.ui_state.selected_arrow = None;
                // Ne pas arrêter le renommage ici, handle_rename le fera si on perd le focus
            }
        }
    }
}

fn handle_arrow_creation(ctx: &Context, response: &egui::Response, app_state: &mut AppState) {
    let pointer = &ctx.input(|i| i.pointer.clone());
    let mouse_pos = app_state.ui_state.mouse_pos;

    // --- Démarrage ---
    if pointer.button_pressed(PointerButton::Primary) {
        if let Some(start_point) = find_closest_connection_point(
            &app_state.diagram,
            pointer.press_origin().unwrap_or_default(),
            CONNECTION_POINT_RADIUS * 3.0, // Tolérance initiale
        ) {
            // Ne démarrer que si on ne drague pas déjà un noeud du canvas
            if !ctx.is_being_dragged(response.id) {
                 // Empêcher sélection de nœud si on démarre une flèche
                 app_state.ui_state.selected_node = None;
                 app_state.ui_state.selected_arrow = None;
                 // Démarrer création
                 app_state.ui_state.arrow_creation_start = Some(start_point.clone());
                 log::debug!("Début création flèche depuis: {:?}", start_point);
            } else {
                 log::trace!("Ignoré début flèche: drag de noeud en cours");
            }
        }
    }

    // --- Fin ou Annulation ---
    if let Some(start_point) = &app_state.ui_state.arrow_creation_start {
        if pointer.any_released() {
             log::debug!("Tentative fin création flèche à: {:?}", mouse_pos);
             if let Some(end_point) = find_closest_connection_point(
                &app_state.diagram,
                mouse_pos,
                CONNECTION_POINT_RADIUS * 4.0,
            ) {
                if start_point.node_id != end_point.node_id && app_state.diagram.nodes.contains_key(&end_point.node_id) {
                    let arrow_type = match (start_point.side, end_point.side) {
                         (Side::Right, Side::Left) => ArrowType::Output,
                         (Side::Left, Side::Right) => ArrowType::Input,
                         (_, Side::Top) => ArrowType::Control,
                         (_, Side::Bottom) => ArrowType::Mechanism,
                         (Side::Right, _) => ArrowType::Output,
                         (_, Side::Left) => ArrowType::Input,
                         _ => ArrowType::Input,
                    };
                    log::debug!("Fin création flèche vers: {:?}, type: {:?}", end_point, arrow_type);
                    app_state.diagram.add_arrow(start_point.clone(), end_point, arrow_type, None);
                } else {
                    log::debug!("Annulation flèche: connexion au même noeud ou noeud cible invalide.");
                }
            } else {
                 log::debug!("Annulation flèche: relâchement dans le vide.");
            }
            // Réinitialiser DANS TOUS LES CAS après relâchement
            app_state.ui_state.arrow_creation_start = None;
        }
    }
}

// Gère renommage nœuds et flèches
fn handle_rename(ctx: &Context, app_state: &mut AppState) {
    // Renommage Nœud
    if let Some(node_id) = app_state.ui_state.renaming_node {
        if let Some(node) = app_state.diagram.get_node(node_id) {
             let node_rect = node.rect;
             egui::Window::new("Renommer Nœud")
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, node_rect.center_bottom() - node_rect.center()) // Positionner
                .show(ctx, |ui| {
                    let text_edit_response = ui.text_edit_singleline(&mut app_state.ui_state.renaming_label_text);
                    if !ctx.memory(|mem| mem.has_focus(text_edit_response.id)) {
                        text_edit_response.request_focus();
                    }

                    let mut close_rename_window = false;
                    let mut rename_successful = false;

                    if text_edit_response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                         rename_successful = true;
                         close_rename_window = true;
                    } else if !text_edit_response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                         rename_successful = true;
                         close_rename_window = true;
                    } else if ui.input(|i| i.key_pressed(Key::Escape)) {
                         close_rename_window = true;
                         rename_successful = false;
                    } else if text_edit_response.lost_focus() {
                          close_rename_window = true;
                          rename_successful = false;
                          log::debug!("Renommage annulé par perte de focus.");
                    }

                    if close_rename_window {
                        if rename_successful {
                            if let Some(node_mut) = app_state.diagram.get_node_mut(node_id) {
                                 node_mut.name = app_state.ui_state.renaming_label_text.clone();
                                 log::info!("Nœud {} renommé en '{}'", node_id, node_mut.name);
                            }
                        } else {
                            log::info!("Renommage Nœud {} annulé.", node_id);
                        }
                        app_state.ui_state.renaming_node = None;
                        app_state.ui_state.renaming_label_text.clear();
                    }
                });
        } else {
            app_state.ui_state.renaming_node = None; // Nœud disparu ?
        }
    }
    // Renommage Flèche
    else if let Some(arrow_id) = app_state.ui_state.renaming_arrow {
         if let Some(arrow) = app_state.diagram.get_arrow(arrow_id) {
             let mid_pos = if let (Some(src_node), Some(tgt_node)) = (
                 app_state.diagram.get_node(arrow.source.node_id),
                 app_state.diagram.get_node(arrow.target.node_id),
             ) {
                 get_connection_pos(src_node, arrow.source.side).lerp(get_connection_pos(tgt_node, arrow.target.side), 0.5)
             } else {
                 Pos2::ZERO
             };

             egui::Window::new("Editer Label Flèche")
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, mid_pos - Pos2::ZERO)
                .show(ctx, |ui| {
                    let text_edit_response = ui.text_edit_singleline(&mut app_state.ui_state.renaming_label_text);
                    if !ctx.memory(|mem| mem.has_focus(text_edit_response.id)) {
                        text_edit_response.request_focus();
                    }

                    let mut close_rename_window = false;
                    let mut rename_successful = false;

                    if text_edit_response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                         rename_successful = true;
                         close_rename_window = true;
                    } else if !text_edit_response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                         rename_successful = true;
                         close_rename_window = true;
                    } else if ui.input(|i| i.key_pressed(Key::Escape)) {
                         close_rename_window = true;
                         rename_successful = false;
                    } else if text_edit_response.lost_focus() {
                          close_rename_window = true;
                          rename_successful = false;
                          log::debug!("Edition label annulée par perte de focus.");
                    }

                    if close_rename_window {
                        if rename_successful {
                            if let Some(arrow_mut) = app_state.diagram.arrows.get_mut(&arrow_id) {
                                 let new_label = app_state.ui_state.renaming_label_text.clone();
                                 arrow_mut.label = if new_label.is_empty() { None } else { Some(new_label) };
                                 log::info!("Label flèche {} édité en '{:?}'", arrow_id, arrow_mut.label);
                            }
                        } else {
                             log::info!("Edition label flèche {} annulée.", arrow_id);
                        }
                        app_state.ui_state.renaming_arrow = None;
                        app_state.ui_state.renaming_label_text.clear();
                    }
                });
        } else {
             app_state.ui_state.renaming_arrow = None; // Flèche disparue ?
        }
    }
    // Pas de détection double-clic pour l'instant
}

// Gère suppression via clavier
fn handle_deletion(ctx: &Context, app_state: &mut AppState) {
    if ctx.input(|i| i.key_pressed(Key::Delete)) || ctx.input(|i| i.key_pressed(Key::Backspace)) {
        if let Some(node_id) = app_state.ui_state.selected_node.take() {
             log::info!("Suppression noeud via clavier: {}", node_id);
            app_state.diagram.remove_node(node_id);
            app_state.ui_state.renaming_node = None;
            app_state.ui_state.renaming_arrow = None;
        } else if let Some(arrow_id) = app_state.ui_state.selected_arrow.take() {
             log::info!("Suppression flèche via clavier: {}", arrow_id);
             app_state.diagram.remove_arrow(arrow_id);
             app_state.ui_state.renaming_arrow = None;
             app_state.ui_state.renaming_node = None;
        }
    }
}