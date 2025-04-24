use crate::app::AppState; // On aura besoin de l'état global
use crate::sadt_elements::{ArrowType, Side};
use crate::ui::drawing::{find_closest_connection_point, CONNECTION_POINT_RADIUS};
use egui::{Context, Key, PointerButton, Ui};

const DRAG_THRESHOLD: f32 = 5.0; // Distance minimale pour commencer un drag

pub fn handle_canvas_interactions(
    ctx: &Context,
    _ui: &mut Ui,
    response: &egui::Response, // Réponse du CentralPanel où l'on dessine
    app_state: &mut AppState, // Accès mutable au modèle et à l'état UI
) {
    app_state.ui_state.mouse_pos = response.hover_pos().unwrap_or_default(); // Mémoriser la position souris

    handle_node_drag_and_select(ctx, response, app_state);
    handle_arrow_creation(ctx, response, app_state);
    handle_node_rename(ctx, response, app_state);
    handle_deletion(ctx, app_state);

    // Clic droit pour le menu contextuel (pas encore implémenté ici)
    response.context_menu(|ui| {
        if app_state.ui_state.selected_node.is_some() {
            if ui.button("Renommer Nœud").clicked() {
                 app_state.ui_state.renaming_node = app_state.ui_state.selected_node;
                 ui.close_menu();
            }
             if ui.button("Supprimer Nœud").clicked() {
                if let Some(id) = app_state.ui_state.selected_node.take() {
                    app_state.diagram.remove_node(id);
                }
                ui.close_menu();
            }
        } else if app_state.ui_state.selected_arrow.is_some() {
             // Ajouter options pour flèches si besoin
        } else {
            if ui.button("Ajouter Nœud").clicked() {
                let pos = response.hover_pos().unwrap_or_else(|| ui.available_rect_before_wrap().min);
                app_state.diagram.add_node("Nouveau Nœud".to_string(), pos);
                 ui.close_menu();
            }
        }
    });
}

fn handle_node_drag_and_select(ctx: &Context, response: &egui::Response, app_state: &mut AppState) {
    let pointer = &ctx.input(|i| i.pointer.clone());

    // Détection début de drag ou sélection
    if response.hovered() && pointer.button_pressed(PointerButton::Primary) {
        let click_pos = pointer.press_origin().unwrap_or_default();
        let mut clicked_on_node = None;

        // Trouver si on a cliqué sur un nœud (ordre inverse du dessin pour le z-index)
        for node in app_state.diagram.nodes.values() {
            if node.rect.contains(click_pos) {
                clicked_on_node = Some(node.id);
                break; // On prend le premier trouvé (le plus "haut")
            }
        }

        if let Some(node_id) = clicked_on_node {
            app_state.ui_state.selected_node = Some(node_id);
            app_state.ui_state.selected_arrow = None;
            // Marquer pour potentiel drag
            ctx.set_dragged_id(response.id);

        } else {
             // Clic dans le vide : désélectionner
             app_state.ui_state.selected_node = None;
             app_state.ui_state.selected_arrow = None;
             app_state.ui_state.renaming_node = None; // Arrêter le renommage si on clique ailleurs
        }
    }

    // Gestion du drag en cours
    if pointer.button_down(PointerButton::Primary) && ctx.is_being_dragged(response.id) {
       if let Some(node_id) = app_state.ui_state.selected_node {
           if let Some(node) = app_state.diagram.get_node_mut(node_id) {
                node.rect = node.rect.translate(pointer.delta());
           }
           // S'assurer qu'on est bien en mode drag (évite de créer flèche en même temps)
           app_state.ui_state.arrow_creation_start = None;
       }
    }

    // Fin du drag
    if ctx.input(|i| i.pointer.any_released()) {
        ctx.stop_dragging();
    }
}

fn handle_arrow_creation(ctx: &Context, response: &egui::Response, app_state: &mut AppState) {
    let pointer = &ctx.input(|i| i.pointer.clone());
    let mouse_pos = app_state.ui_state.mouse_pos;

    // Démarrer la création d'une flèche (clic sur un point de connexion)
    if response.hovered() && pointer.button_pressed(PointerButton::Primary) && app_state.ui_state.selected_node.is_none() { // Ne pas démarrer si on sélectionne/drag un noeud
        if let Some(start_point) = find_closest_connection_point(
            &app_state.diagram,
            pointer.press_origin().unwrap_or_default(),
            CONNECTION_POINT_RADIUS * 2.0, // Tolérance autour du point
        ) {
            // Vérifier qu'on ne drag pas déjà un noeud
            if ctx.dragged_id().is_none() {
                 app_state.ui_state.arrow_creation_start = Some(start_point.clone());
                 // Marquer pour potentiel drag (de la flèche)
                 ctx.set_dragged_id(response.id.with("arrow_drag"));
                 log::debug!("Début création flèche depuis: {:?}", start_point);
            }
        }
    }

    // Terminer la création de la flèche (relâchement sur un point de connexion)
    if let Some(start_point) = &app_state.ui_state.arrow_creation_start {
        if pointer.any_released() {
             if let Some(end_point) = find_closest_connection_point(
                &app_state.diagram,
                mouse_pos,
                CONNECTION_POINT_RADIUS * 4.0, // Tolérance plus grande au relâchement
            ) {
                if start_point.node_id != end_point.node_id { // Ne pas connecter un noeud à lui-même (simple check)
                    // Déterminer le type de flèche basé sur les côtés (heuristique simple)
                    let arrow_type = match (start_point.side, end_point.side) {
                         (Side::Right, Side::Left) => ArrowType::Output, // ou Input si inversé
                         (Side::Left, Side::Right) => ArrowType::Input,
                         (_, Side::Top) => ArrowType::Control,
                         (_, Side::Bottom) => ArrowType::Mechanism,
                         // Cas par défaut (peut être affiné)
                         (Side::Right, _) => ArrowType::Output,
                         (_, Side::Left) => ArrowType::Input,
                         _ => ArrowType::Input, // Ou un autre défaut
                    };

                    log::debug!("Fin création flèche vers: {:?}, type: {:?}", end_point, arrow_type);
                    // Cloner start_point ici car add_arrow attend une valeur possédée
                    app_state.diagram.add_arrow(start_point.clone(), end_point, arrow_type, None);
                } else {
                    log::debug!("Annulation flèche: connexion au même noeud.");
                }
            } else {
                 log::debug!("Annulation flèche: relâchement dans le vide.");
            }
            // Réinitialiser dans tous les cas
            app_state.ui_state.arrow_creation_start = None;
            ctx.stop_dragging(); // Arrêter le drag spécifique à la flèche
        }
        // Si on relâche sans être sur un point de connexion, la flèche est annulée implicitement par la réinitialisation ci-dessus.
    }
}

fn handle_node_rename(ctx: &Context, response: &egui::Response, app_state: &mut AppState) {
    if let Some(node_id) = app_state.ui_state.renaming_node {
        if let Some(node) = app_state.diagram.get_node(node_id) {
             let mut temp_name = node.name.clone();
             let node_rect = node.rect; // Copier rect avant d'emprunter mut app_state

            // Créer une fenêtre flottante pour l'édition
             egui::Window::new("Renommer Nœud")
                .collapsible(false)
                .resizable(false)
                .title_bar(false)
                .fixed_pos(node_rect.center_top() + egui::vec2(0.0, -30.0)) // Positionner au-dessus du noeud
                .show(ctx, |ui| {
                    let text_edit_response = ui.text_edit_singleline(&mut temp_name);
                    // Mettre le focus la première fois qu'on ouvre la fenêtre
                    if !ctx.memory(|mem| mem.has_focus(text_edit_response.id)) {
                        text_edit_response.request_focus();
                    }
                    // Valider avec Entrée ou perdre le focus
                    if text_edit_response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        if let Some(node_mut) = app_state.diagram.get_node_mut(node_id) {
                             node_mut.name = temp_name.clone();
                        }
                        app_state.ui_state.renaming_node = None; // Terminer le renommage
                    }
                    // Annuler avec Echap (ou clic extérieur géré par lost_focus sans Enter)
                    if text_edit_response.lost_focus() || ui.input(|i| i.key_pressed(Key::Escape)) {
                        app_state.ui_state.renaming_node = None;
                    }
                });

            // Si la fenêtre d'édition n'est plus ouverte (par ex. clic extérieur), arrêter le renommage
            // Note: egui gère cela implicitement si on utilise `id_source` mais ici on contrôle manuellement
             // On utilise le fait que si on clique ailleurs, le node_id est désélectionné/changé
        } else {
            // Le noeud a été supprimé pendant qu'on le renommait ?
            app_state.ui_state.renaming_node = None;
        }
    } else {
         // Détecter double-clic pour démarrer le renommage
        if response.double_clicked() {
            let click_pos = ctx.input(|i| i.pointer.interact_pos()).unwrap_or_default();
            let mut clicked_on_node = None;
             // Trouver si on a cliqué sur un nœud
            for node in app_state.diagram.nodes.values() {
                if node.rect.contains(click_pos) {
                    clicked_on_node = Some(node.id);
                    break;
                }
            }
             if let Some(node_id) = clicked_on_node {
                app_state.ui_state.renaming_node = Some(node_id);
                app_state.ui_state.selected_node = Some(node_id); // Sélectionner aussi
             }
        }
    }
}

fn handle_deletion(ctx: &Context, app_state: &mut AppState) {
    if ctx.input(|i| i.key_pressed(Key::Delete)) || ctx.input(|i| i.key_pressed(Key::Backspace)) {
        if let Some(node_id) = app_state.ui_state.selected_node.take() {
             log::info!("Suppression noeud: {}", node_id);
            app_state.diagram.remove_node(node_id);
            app_state.ui_state.renaming_node = None; // Assurer qu'on arrête de renommer si supprimé
        } else if let Some(arrow_id) = app_state.ui_state.selected_arrow.take() {
             log::info!("Suppression flèche: {}", arrow_id);
             app_state.diagram.remove_arrow(arrow_id);
        }
    }
}